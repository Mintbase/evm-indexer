use anyhow::{Context, Result};
use data_store::models::{Erc1155, Erc1155Owner};
use data_store::{
    models::{Nft, TokenContract, Transaction},
    store::DataStore,
};
use eth::{
    rpc::ethrpc::Client as EthRpcClient,
    rpc::EthNodeReading,
    types::{Address, BlockData, NftId, TxDetails},
};
use event_retriever::db_reader::{
    diesel::{BlockRange, EventSource},
    models::*,
};
use std::{collections::HashMap, sync::Arc};

#[derive(Default, Debug, PartialEq)]
pub struct UpdateCache {
    nfts: HashMap<NftId, Nft>,
    multi_tokens: HashMap<NftId, Erc1155>,
    /// (Token, Contract, Owner) -> Ownership
    multi_token_owners: HashMap<(NftId, Address, Address), Erc1155Owner>,
    contracts: HashMap<Address, TokenContract>,
    transactions: Vec<Transaction>,
    blocks: Vec<BlockData>,
}

impl UpdateCache {
    /// This method writes its records to the provided DataStore
    /// while relieving itself of its memory.
    pub async fn write(&mut self, db: &mut DataStore) {
        // TODO - It would be ideal if all db actions happened in a single commit
        //  so that failure to write any one of them results in no changes at all.
        //  this can be done with @databases typescript library so it should be possible here.

        // Write and clear transactions
        db.save_transactions(std::mem::take(&mut self.transactions));

        // Write and clear blocks
        db.save_blocks(std::mem::take(&mut self.blocks));

        // Write and clear contracts
        db.save_contracts(std::mem::take(
            &mut self.contracts.drain().map(|(_, v)| v).collect(),
        ));

        // Write and clear nfts
        db.save_nfts(std::mem::take(
            &mut self.nfts.drain().map(|(_, v)| v).collect(),
        ))
        .await;

        // Write and clear erc1155s
        db.save_erc1155s(std::mem::take(
            &mut self.multi_tokens.drain().map(|(_, v)| v).collect(),
        ))
        .await;

        // Write and clear erc1155_owners
        db.save_erc1155_owners(std::mem::take(
            &mut self.multi_token_owners.drain().map(|(_, v)| v).collect(),
        ))
        .await;
    }
}

/// Where chain data should be retrieved from
pub enum ChainDataSource {
    Database,
    Node,
}

pub struct HandlerConfig {
    /// Source of chain data (blocks & transactions)
    chain_data_source: ChainDataSource,
}

pub struct EventHandler {
    /// Source of events for processing
    source: EventSource,
    /// Location of existing stored content
    store: DataStore,
    /// A memory store updates.
    updates: UpdateCache,
    /// Web3 Provider
    eth_client: Arc<dyn EthNodeReading>,
    /// Runtime configuration parameters
    config: HandlerConfig,
}

impl EventHandler {
    pub fn new(
        source_url: &str,
        store_url: &str,
        eth_rpc: &str,
        config: HandlerConfig,
    ) -> Result<Self> {
        Ok(Self {
            source: EventSource::new(source_url).context("init EventSource")?,
            store: DataStore::new(store_url).context("init DataStore")?,
            updates: UpdateCache::default(),
            eth_client: Arc::new(EthRpcClient::new(eth_rpc).context("init EthRpcClient")?),
            config,
        })
    }

    pub async fn load_chain_data(&mut self, range: BlockRange) -> Result<HashMap<u64, BlockData>> {
        let block_info = match self.config.chain_data_source {
            ChainDataSource::Database => {
                tracing::info!("retrieving block and transaction data from arak");
                self.source.get_blocks_for_range(range)?
            }
            ChainDataSource::Node => {
                tracing::info!("retrieving block and transaction data from node");
                self.eth_client
                    .get_blocks_for_range(range.start as u64, range.end as u64)
                    .await?
            }
        };
        self.updates
            .transactions
            .extend(
                block_info
                    .clone()
                    .into_iter()
                    .flat_map(|(block, block_data)| {
                        block_data
                            .transactions
                            .into_iter()
                            .map(move |(idx, data)| Transaction::new(block, idx, data))
                    }),
            );

        self.updates.blocks.extend(block_info.clone().into_values());
        Ok(block_info)
    }

    fn check_for_contract(&mut self, event: &EventBase) {
        let address = event.contract_address;
        if self.updates.contracts.contains_key(&address)
            || self.store.load_contract(address).is_some()
        {
            return;
        }
        // tracing::debug!("new contract {:?}", address.0);
        self.updates
            .contracts
            .insert(address, TokenContract::from_event_base(event));
    }

    async fn get_missing_node_data(&mut self) {
        let (mut missing_uris, mut contract_details) = self
            .eth_client
            .get_uris_and_contract_details(
                self.updates
                    .nfts
                    .clone()
                    .into_iter()
                    // Without additional specification here this will retry to fetch things
                    // We can prevent this by perhaps by filtering also for range.start < mint_block
                    .filter(|(_, token)| token.token_uri.is_none())
                    .map(|(id, _)| id)
                    .collect(),
                self.updates.contracts.keys().copied().collect(),
            )
            .await;
        tracing::info!(
            "retrieving missing node data for {} contracts and {} tokens",
            contract_details.len(),
            missing_uris.len()
        );
        for (id, possible_uri) in missing_uris.drain() {
            if let Some(uri) = possible_uri {
                self.updates.nfts.get_mut(&id).expect("known").token_uri = Some(uri);
            }
        }

        for (address, details) in contract_details.drain() {
            let contract = self
                .updates
                .contracts
                .get_mut(&address)
                .expect("known to exist");
            contract.name = details.name;
            contract.symbol = details.symbol;
        }
    }

    pub async fn process_events_for_block_range(&mut self, range: BlockRange) -> Result<()> {
        tracing::info!("processing events for {:?}", range);
        let event_map = self.source.get_events_for_block_range(range)?;
        let mut block_data = self.load_chain_data(range).await?;
        for (block, block_events) in event_map.into_iter() {
            let tx_data = block_data
                .remove(&block)
                .unwrap_or_else(|| panic!("Missing block {} in {:?}", block, range))
                .transactions;
            for ((tidx, _lidx), tx_events) in block_events {
                let tx = tx_data.get(&tidx).expect("receipt known to exist!");
                for NftEvent { base, meta } in tx_events.into_iter() {
                    self.check_for_contract(&base);
                    match meta {
                        EventMeta::Erc721Approval(a) => self.handle_erc721_approval(base, a, tx),
                        EventMeta::Erc721Transfer(t) => self.handle_erc721_transfer(base, t, tx),
                        EventMeta::Erc1155TransferBatch(batch) => {
                            for (id, value) in batch.ids.into_iter().zip(batch.values.into_iter()) {
                                self.handle_erc1155_transfer(
                                    base,
                                    Erc1155TransferSingle {
                                        operator: batch.operator,
                                        from: batch.from,
                                        to: batch.to,
                                        id,
                                        value,
                                    },
                                    tx,
                                )
                            }
                        }
                        EventMeta::Erc1155TransferSingle(t) => {
                            self.handle_erc1155_transfer(base, t, tx)
                        }
                        _ => {
                            // tracing::error!("unhandled event!");
                            continue;
                        }
                    };
                }
            }
        }
        // Retrieve missing data from node.
        self.get_missing_node_data().await;

        // Drain cache and write to store
        self.updates.write(&mut self.store).await;
        tracing::info!("completed event processing for {:?}", range);
        Ok(())
    }
    fn handle_erc721_approval(
        &mut self,
        base: EventBase,
        approval: Erc721Approval,
        tx: &TxDetails,
    ) {
        let nft_id = NftId {
            address: base.contract_address,
            token_id: approval.id,
        };
        let mut nft = match self.updates.nfts.remove(&nft_id) {
            Some(nft) => nft,
            None => match self.store.load_nft(&nft_id) {
                Some(nft) => nft,
                None => {
                    // tracing::warn!("approval received before token mint {:?}", nft_id);
                    Nft::new(&base, &nft_id, tx)
                }
            },
        };
        if nft.event_applied(&base) {
            tracing::warn!(
                "skipping attempt to replay event {:?} at tx {:?} on {:?}",
                base,
                tx.hash,
                nft_id
            );
            // Put the nft back in cache!
            self.updates.nfts.insert(nft_id, nft);
            return;
        }
        nft.approved = if approval.approved == Address::zero() {
            None
        } else {
            Some(approval.approved.into())
        };
        nft.last_update_block = base.block_number as i64;
        nft.last_update_tx = base.transaction_index as i64;
        nft.last_update_log_index = base.log_index as i64;
        self.updates.nfts.insert(nft_id, nft);
    }

    fn handle_erc721_transfer(
        &mut self,
        base: EventBase,
        transfer: Erc721Transfer,
        tx: &TxDetails,
    ) {
        let nft_id = NftId {
            address: base.contract_address,
            token_id: transfer.token_id,
        };

        let mut nft = match self.updates.nfts.remove(&nft_id) {
            Some(nft) => nft,
            None => self.store.load_or_initialize_nft(&base, &nft_id, tx),
        };
        if nft.event_applied(&base) {
            tracing::warn!(
                "skipping attempt to replay event {:?} at tx {:?} on {:?}",
                base,
                tx.hash,
                nft_id
            );
            // Put the nft back in cache!
            self.updates.nfts.insert(nft_id, nft);
            return;
        }
        let EventBase {
            block_number,
            transaction_index,
            log_index,
            ..
        } = base;
        // TODO - Maybe we should just leave Event Base fields as i64...
        let block = block_number.try_into().expect("i64 block");
        let tx_index = transaction_index.try_into().expect("i64 tx_index");
        let log_index = log_index.try_into().expect("i64 log index");

        if transfer.to == Address::zero() {
            // burn token
            nft.burn_block = Some(block);
            nft.burn_tx = Some(tx_index);
        }
        if transfer.from == Address::zero() {
            // Mint: This case is already handled by load_or_initialize
        }
        nft.owner = transfer.to;
        nft.last_update_block = block;
        nft.last_update_tx = base.transaction_index as i64;
        nft.last_update_log_index = log_index;
        nft.last_transfer_block = Some(block);
        nft.last_transfer_tx = Some(tx_index);
        // TODO - fetch and set json. Maybe in load_or_initialize
        // Approvals are unset on transfer.
        nft.approved = None;
        self.updates.nfts.insert(nft_id, nft);
    }

    fn handle_erc1155_transfer(
        &mut self,
        base: EventBase,
        transfer: Erc1155TransferSingle,
        tx: &TxDetails,
    ) {
        let nft_id = NftId {
            address: base.contract_address,
            token_id: transfer.id,
        };

        let mut token = match self.updates.multi_tokens.remove(&nft_id) {
            Some(nft) => nft,
            None => self.store.load_or_initialize_erc1155(&base, &nft_id, tx),
        };
        if token.event_applied(&base) {
            tracing::warn!(
                "skipping attempt to replay event {:?} at tx {:?} on {:?}",
                base,
                tx.hash,
                nft_id
            );
            // Put the nft back in cache!
            self.updates.multi_tokens.insert(nft_id, token);
            return;
        }
        let EventBase {
            block_number,
            transaction_index,
            log_index,
            ..
        } = base;
        let block = block_number.try_into().expect("i64 block");
        let tx_index = transaction_index.try_into().expect("i64 tx_index");
        let log_index = log_index.try_into().expect("i64 log index");

        token.last_update_block = block;
        token.last_update_tx = tx_index;
        token.last_update_log_index = log_index;

        let from = transfer.from;
        let to = transfer.to;

        // Supply related updates.
        if to == Address::zero() {
            token.decrease_supply(transfer.value);
        }
        if from == Address::zero() {
            token.increase_supply(transfer.value);
        }

        // Ownership updates
        let contract = base.contract_address;
        if from != Address::zero() {
            let mut sender = match self
                .updates
                .multi_token_owners
                .remove(&(nft_id, contract, from))
            {
                Some(owner) => owner,
                None => self
                    .store
                    .load_or_initialize_erc1155_owner(&base, &nft_id, from),
            };
            sender.decrease_balance(transfer.value);
            self.updates
                .multi_token_owners
                .insert((nft_id, contract, from), sender);
        }
        let mut recipient = match self
            .updates
            .multi_token_owners
            .remove(&(nft_id, contract, to))
        {
            Some(owner) => owner,
            None => self
                .store
                .load_or_initialize_erc1155_owner(&base, &nft_id, to),
        };
        recipient.increase_balance(transfer.value);
        self.updates
            .multi_token_owners
            .insert((nft_id, contract, to), recipient);

        self.updates.multi_tokens.insert(nft_id, token);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eth::types::{Address, Bytes32, NftId, U256};
    use event_retriever::db_reader::diesel::BlockRange;
    use std::str::FromStr;
    use tracing_test::traced_test;
    static TEST_SOURCE_URL: &str = "postgresql://postgres:postgres@localhost:5432/arak";
    static TEST_STORE_URL: &str = "postgresql://postgres:postgres@localhost:5432/store";
    static TEST_ETH_RPC: &str = "https://rpc.ankr.com/eth";

    fn test_handler() -> EventHandler {
        EventHandler::new(
            TEST_SOURCE_URL,
            TEST_STORE_URL,
            TEST_ETH_RPC,
            HandlerConfig {
                chain_data_source: ChainDataSource::Database,
            },
        )
        .unwrap()
    }

    #[tokio::test]
    #[ignore]
    #[traced_test]
    async fn event_processing() {
        let mut handler = test_handler();
        let block = std::cmp::max(handler.store.get_max_block() + 1, 15_000_000);
        let range = BlockRange {
            start: block,
            end: block + 100,
        };
        let result = handler.process_events_for_block_range(range).await;
        match result {
            Ok(_) => assert_eq!(handler.store.get_max_block(), range.end - 1),
            Err(err) => panic!("{}", err.to_string()),
        }
        // TODO - construct a sequence of events and actually check the Store State is as expected here.
    }

    struct SetupData {
        handler: EventHandler,
        // contract_address: Address,
        token_id: U256,
        token: NftId,
        base: EventBase,
        tx: TxDetails,
    }

    fn setup_data() -> SetupData {
        let handler = test_handler();
        let contract_address = Address::from(1);
        let token_id = U256::from(123);
        let token = NftId {
            address: contract_address,
            token_id,
        };
        let base = EventBase {
            block_number: 1,
            log_index: 2,
            transaction_index: 3,
            contract_address,
        };
        let tx = TxDetails {
            hash: Bytes32::from_str(
                "0xe9e91f1ee4b56c0df2e9f06c2b8c27c6076195a88a7b8537ba8313d80e6f124e",
            )
            .unwrap(),
            from: Address::from_str("0x32be343b94f860124dc4fee278fdcbd38c102d88").unwrap(),
            to: Some(Address::from_str("0xdf190dc7190dfba737d7777a163445b7fff16133").unwrap()),
        };
        SetupData {
            handler,
            token_id,
            token,
            base,
            tx,
        }
    }

    // These tests shouldn't need to be async, but the handler struct contains async fields.
    #[tokio::test]
    async fn erc721_approval_handler() {
        let SetupData {
            mut handler,
            token_id: _,
            token,
            mut base,
            tx,
        } = setup_data();

        let approved = Address::from(3);
        let first_approval = Erc721Approval {
            owner: Address::from(2),
            approved,
            id: token.token_id,
        };
        // Approval before token existence (handled the way the event said)
        handler.handle_erc721_approval(base, first_approval, &tx);
        let nft = handler.updates.nfts.get(&token).unwrap();
        assert_eq!(
            Address::from(nft.clone().approved.unwrap()),
            approved,
            "first approval"
        );
        base.block_number += 1; // reuse incremented base.
        handler.handle_erc721_approval(
            base,
            Erc721Approval {
                owner: Address::from(2),
                approved: Address::zero(),
                id: token.token_id,
            },
            &tx,
        );
        let nft = handler.updates.nfts.get(&token).unwrap();
        assert_eq!(nft.approved, None, "second approval");

        // Idempotency: Try to replay the first approval
        base.block_number -= 1;
        handler.handle_erc721_approval(base, first_approval, &tx);
        // Approval not applied.
        assert_eq!(
            handler.updates.nfts.get(&token).unwrap().approved,
            None,
            "idempotency"
        );
    }

    #[tokio::test]
    async fn erc721_transfer_handler() {
        let SetupData {
            mut handler,
            token_id,
            token,
            base,
            tx,
        } = setup_data();
        let from = Address::from(2);
        let to = Address::from(3);
        let first_transfer = Erc721Transfer { from, to, token_id };
        handler.handle_erc721_transfer(base, first_transfer.clone(), &tx);

        assert_eq!(
            handler.updates.nfts.get(&token).unwrap(),
            &Nft {
                contract_address: base.contract_address,
                token_id: token_id.into(),
                token_uri: None,
                owner: to,
                last_update_block: base.block_number as i64,
                last_update_tx: base.transaction_index as i64,
                last_update_log_index: base.log_index as i64,
                last_transfer_block: Some(base.block_number as i64),
                last_transfer_tx: Some(base.transaction_index as i64),
                mint_block: base.block_number as i64,
                mint_tx: base.transaction_index as i64,
                burn_block: None,
                burn_tx: None,
                minter: tx.from,
                approved: None,
            },
            "first transfer"
        );
        let base_2 = EventBase {
            block_number: 4,
            log_index: 5,
            transaction_index: 6,
            contract_address: base.contract_address,
        };
        // Transfer back
        handler.handle_erc721_transfer(
            base_2,
            Erc721Transfer {
                from: to,
                to: from,
                token_id,
            },
            &tx,
        );

        assert_eq!(
            handler.updates.nfts.get(&token).unwrap(),
            &Nft {
                contract_address: base.contract_address,
                token_id: token_id.into(),
                token_uri: None,
                owner: from,
                last_update_block: base_2.block_number as i64,
                last_update_tx: base_2.transaction_index as i64,
                last_update_log_index: base_2.log_index as i64,
                last_transfer_block: Some(base_2.block_number as i64),
                last_transfer_tx: Some(base_2.transaction_index as i64),
                mint_block: base.block_number as i64,
                mint_tx: base.transaction_index as i64,
                burn_block: None,
                burn_tx: None,
                minter: tx.from,
                approved: None,
            },
            "transfer back"
        );

        // Burn Token
        let base_3 = EventBase {
            block_number: 7,
            log_index: 8,
            transaction_index: 9,
            contract_address: base.contract_address,
        };
        handler.handle_erc721_transfer(
            base_3,
            Erc721Transfer {
                from,
                to: Address::zero(),
                token_id,
            },
            &tx,
        );
        assert_eq!(
            handler.updates.nfts.get(&token).unwrap(),
            &Nft {
                contract_address: base.contract_address,
                token_id: token_id.into(),
                token_uri: None,
                owner: Address::zero(),
                last_update_block: base_3.block_number as i64,
                last_update_tx: base_3.transaction_index as i64,
                last_update_log_index: base_3.log_index as i64,
                last_transfer_block: Some(base_3.block_number as i64),
                last_transfer_tx: Some(base_3.transaction_index as i64),
                mint_block: base.block_number as i64,
                mint_tx: base.transaction_index as i64,
                burn_block: Some(base_3.block_number as i64),
                burn_tx: Some(base_3.transaction_index as i64),
                minter: tx.from,
                approved: None,
            },
            "burn transfer"
        );

        // Idempotency: try to replay earlier transfers
        handler.handle_erc721_transfer(base, first_transfer, &tx);
        assert_eq!(
            handler.updates.nfts.get(&token).unwrap().owner,
            Address::zero(),
            "idempotency"
        )
    }

    #[tokio::test]
    async fn erc1155_transfer_handler() {
        let SetupData {
            mut handler,
            token_id: id,
            token,
            base,
            tx,
        } = setup_data();
        let from = Address::from(2);
        let to = Address::from(3);
        let value = U256::from(456789);
        let first_transfer = Erc1155TransferSingle {
            operator: Default::default(),
            from,
            to,
            id,
            value,
        };
        handler.handle_erc1155_transfer(base, first_transfer.clone(), &tx);

        assert_eq!(
            handler.updates.multi_tokens.get(&token).unwrap(),
            &Erc1155 {
                contract_address: base.contract_address,
                token_id: id.into(),
                token_uri: None,
                // Note that we did not mint first so this transferred value
                // is not realized in the total supply
                total_supply: 0.into(),
                last_update_block: base.block_number as i64,
                last_update_tx: base.transaction_index as i64,
                last_update_log_index: base.log_index as i64,
                mint_block: base.block_number as i64,
                mint_tx: base.transaction_index as i64,
                creator_address: tx.from,
            },
            "first transfer"
        );

        assert_eq!(
            handler
                .updates
                .multi_token_owners
                .get(&(token, base.contract_address, to))
                .unwrap(),
            &Erc1155Owner {
                contract_address: base.contract_address,
                token_id: id.into(),
                owner: to,
                balance: value.into(),
            },
            "first transfer recipient balance"
        );

        assert_eq!(
            handler
                .updates
                .multi_token_owners
                .get(&(token, base.contract_address, from))
                .unwrap(),
            &Erc1155Owner {
                contract_address: base.contract_address,
                token_id: id.into(),
                owner: from,
                // Negative balance because they sent before ever receiving!
                balance: (-456789).into(),
            },
            "first transfer sender balance"
        );

        let base_2 = EventBase {
            block_number: 4,
            log_index: 5,
            transaction_index: 6,
            contract_address: base.contract_address,
        };
        // Transfer Balance back
        handler.handle_erc1155_transfer(
            base_2,
            Erc1155TransferSingle {
                from: to,
                to: from,
                id,
                operator: Address::zero(),
                value,
            },
            &tx,
        );

        assert_eq!(
            handler.updates.multi_tokens.get(&token).unwrap(),
            &Erc1155 {
                contract_address: base.contract_address,
                token_id: id.into(),
                token_uri: None,
                total_supply: 0.into(),
                last_update_block: base_2.block_number as i64,
                last_update_tx: base_2.transaction_index as i64,
                last_update_log_index: base_2.log_index as i64,
                mint_block: base.block_number as i64,
                mint_tx: base.transaction_index as i64,
                creator_address: tx.from,
            },
            "transfer back"
        );
        assert_eq!(
            handler
                .updates
                .multi_token_owners
                .get(&(token, base.contract_address, to))
                .unwrap(),
            &Erc1155Owner {
                contract_address: base.contract_address,
                token_id: id.into(),
                owner: to,
                balance: 0.into(),
            },
            "second transfer (back) recipient balance"
        );
        assert_eq!(
            handler
                .updates
                .multi_token_owners
                .get(&(token, base.contract_address, from))
                .unwrap(),
            &Erc1155Owner {
                contract_address: base.contract_address,
                token_id: id.into(),
                owner: from,
                // Negative balance because they sent before ever receiving!
                balance: 0.into(),
            },
            "second transfer (back) sender balance"
        );

        // Mint:
        let mint_base = EventBase {
            block_number: 5,
            log_index: 5,
            transaction_index: 6,
            contract_address: base.contract_address,
        };
        let mint_transfer = Erc1155TransferSingle {
            from: Address::zero(),
            to,
            id,
            operator: Address::zero(),
            value,
        };
        handler.handle_erc1155_transfer(mint_base, mint_transfer, &tx);

        assert_eq!(
            handler.updates.multi_tokens.get(&token).unwrap(),
            &Erc1155 {
                contract_address: base.contract_address,
                token_id: id.into(),
                token_uri: None,
                total_supply: value.into(),
                last_update_block: mint_base.block_number as i64,
                last_update_tx: mint_base.transaction_index as i64,
                last_update_log_index: mint_base.log_index as i64,
                mint_block: base.block_number as i64,
                mint_tx: base.transaction_index as i64,
                creator_address: tx.from,
            },
            "mint"
        );
        assert_eq!(
            handler
                .updates
                .multi_token_owners
                .get(&(token, base.contract_address, to))
                .unwrap(),
            &Erc1155Owner {
                contract_address: base.contract_address,
                token_id: id.into(),
                owner: to,
                balance: value.into(),
            },
            "mint recipient balance"
        );

        // Idempotency: try to replay mint event
        handler.handle_erc1155_transfer(base, first_transfer, &tx);
        assert_eq!(
            handler
                .updates
                .multi_tokens
                .get(&token)
                .unwrap()
                .total_supply,
            value.into(),
            "idempotency"
        );

        // Burn Token
        let base_4 = EventBase {
            block_number: 7,
            log_index: 8,
            transaction_index: 9,
            contract_address: base.contract_address,
        };
        handler.handle_erc1155_transfer(
            base_4,
            Erc1155TransferSingle {
                from: to,
                to: Address::zero(),
                id,
                operator: Address::zero(),
                value,
            },
            &tx,
        );
        assert_eq!(
            handler.updates.multi_tokens.get(&token).unwrap(),
            &Erc1155 {
                contract_address: base.contract_address,
                token_id: id.into(),
                token_uri: None,
                total_supply: 0.into(),
                last_update_block: base_4.block_number as i64,
                last_update_tx: base_4.transaction_index as i64,
                last_update_log_index: base_4.log_index as i64,
                mint_block: base.block_number as i64,
                mint_tx: base.transaction_index as i64,
                creator_address: tx.from,
            },
            "burn"
        );
        assert_eq!(
            handler
                .updates
                .multi_token_owners
                .get(&(token, base.contract_address, to))
                .unwrap(),
            &Erc1155Owner {
                contract_address: base.contract_address,
                token_id: id.into(),
                owner: to,
                balance: 0.into(),
            },
            "burner balance"
        );
    }
}

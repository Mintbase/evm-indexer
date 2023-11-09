use crate::{
    models::{Nft, TokenContract, Transaction},
    store::DataStore,
};
use anyhow::{Context, Result};
use eth::{
    rpc::ethers::Client as EthersClient,
    rpc::ethrpc::Client as EthRpcClient,
    rpc::EthNodeReading,
    types::{Address, BlockData, NftId, TxDetails},
};
use event_retriever::db_reader::{
    diesel::{BlockRange, EventSource},
    models::*,
};
use std::collections::HashMap;

#[derive(Default, Debug, PartialEq)]
pub struct UpdateCache {
    nfts: HashMap<NftId, Nft>,
    contracts: HashMap<Address, TokenContract>,
    transactions: Vec<Transaction>,
    blocks: Vec<BlockData>,
}

impl UpdateCache {
    /// This method writes its records to the provided DataStore
    /// while relieving itself of its memory.
    pub fn write(&mut self, db: &mut DataStore) {
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

        // drain memory into database.
        for (_, nft) in self.nfts.drain() {
            // TODO - Batch these updates.
            db.save_nft(&nft);
        }
    }
}

pub type ChainData = HashMap<u64, TxDetails>;

pub struct EventHandler {
    /// Source of events for processing
    source: EventSource,
    /// Location of existing stored content
    store: DataStore,
    /// A memory store updates.
    updates: UpdateCache,
    /// Web3 Provider
    eth_client: EthRpcClient,
    ethers_client: EthersClient,
}

impl EventHandler {
    pub fn new(source_url: &str, store_url: &str, eth_rpc: &str) -> Result<Self> {
        Ok(Self {
            source: EventSource::new(source_url).context("init EventSource")?,
            store: DataStore::new(store_url).context("init DataStore")?,
            updates: UpdateCache::default(),
            eth_client: EthRpcClient::new(eth_rpc).context("init EthRpcClient")?,
            ethers_client: EthersClient::new(eth_rpc).context("init EthersClient")?,
        })
    }

    pub async fn load_chain_data(&mut self, range: BlockRange) -> Result<HashMap<u64, ChainData>> {
        let tx_data = self
            .ethers_client
            .get_receipts_for_range(range.start as u64, range.end as u64)
            .await?;
        self.updates
            .transactions
            .extend(tx_data.clone().into_iter().flat_map(|(block, block_data)| {
                block_data
                    .into_iter()
                    .map(move |(idx, data)| Transaction::new(block, idx, data))
            }));
        // TODO - fetch all at once with: https://github.com/Mintbase/evm-indexer/issues/57
        let block_info = self
            .eth_client
            .get_blocks_for_range(range.start as u64, range.end as u64)
            .await?;
        self.updates.blocks.extend(block_info.into_values());
        Ok(tx_data)
    }

    fn check_for_contract(&mut self, event: &EventBase) {
        let address = event.contract_address;
        if self.updates.contracts.contains_key(&address)
            || self.store.load_contract(address).is_some()
        {
            return;
        }
        tracing::info!("new contract {:?}", address);
        self.updates
            .contracts
            .insert(address, TokenContract::from_event_base(event));
    }

    async fn get_missing_node_data(&mut self) {
        let mut missing_uris = self
            .eth_client
            .get_uris(
                self.updates
                    .nfts
                    .iter()
                    // Without additional specification here this will retry to fetch things
                    // We can prevent this by perhaps by filtering also for range.start < mint_block
                    .filter(|(_, token)| token.token_uri.is_none())
                    .map(|(id, _)| id)
                    .collect(),
            )
            .await;
        for (id, nft) in self.updates.nfts.iter_mut() {
            if let Some(uri) = missing_uris.remove(id) {
                nft.token_uri = uri;
            }
        }

        let mut contract_details = self
            .eth_client
            .get_contract_details(self.updates.contracts.keys().copied().collect())
            .await;

        for (id, contract) in self.updates.contracts.iter_mut() {
            if let Some(details) = contract_details.remove(id) {
                contract.name = details.name;
                contract.symbol = details.symbol;
            }
        }
    }

    pub async fn process_events_for_block_range(&mut self, range: BlockRange) -> Result<()> {
        let event_map = self.source.get_events_for_block_range(range)?;
        let mut block_data = self.load_chain_data(range).await?;
        for (block, block_events) in event_map.into_iter() {
            let tx_data = block_data.remove(&block).expect("always blue");
            for ((tidx, _lidx), tx_events) in block_events {
                let tx = tx_data.get(&tidx).expect("receipt known to exist!");
                for NftEvent { base, meta } in tx_events.into_iter() {
                    self.check_for_contract(&base);
                    match meta {
                        EventMeta::Erc721Approval(a) => self.handle_erc721_approval(base, a, tx),
                        EventMeta::Erc721Transfer(t) => self.handle_erc721_transfer(base, t, tx),
                        _ => {
                            tracing::error!("unhandled event!");
                            continue;
                        }
                    };
                }
            }
        }

        // Retrieve missing data from node.
        self.get_missing_node_data().await;

        // Drain cache and write to store
        self.updates.write(&mut self.store);
        Ok(())
    }
    fn handle_erc721_approval(
        &mut self,
        base: EventBase,
        approval: Erc721Approval,
        tx: &TxDetails,
    ) {
        tracing::debug!("Processing {:?} of {:?}", approval, base.contract_address);
        let nft_id = NftId {
            address: base.contract_address,
            token_id: approval.id,
        };
        let mut nft = match self.updates.nfts.remove(&nft_id) {
            Some(nft) => nft,
            None => match self.store.load_nft(&nft_id) {
                Some(nft) => nft,
                None => {
                    tracing::warn!("approval received before token mint {:?}", nft_id);
                    Nft::build_from(&base, &nft_id, tx)
                }
            },
        };
        if nft.event_applied(&base) {
            tracing::warn!(
                "skippping attempt to replay event {:?} at tx {:?} on {:?}",
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
        nft.last_update_log_index = base.log_index as i64;
        self.updates.nfts.insert(nft_id, nft);
    }

    fn handle_erc721_transfer(
        &mut self,
        base: EventBase,
        transfer: Erc721Transfer,
        tx: &TxDetails,
    ) {
        tracing::debug!("Processing {:?} of {:?}", transfer, base.contract_address);
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
                "skippping attempt to replay event {:?} at tx {:?} on {:?}",
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
        nft.owner = transfer.to.0.as_slice().to_vec();
        nft.last_update_block = block;
        nft.last_update_log_index = log_index;
        nft.last_transfer_block = Some(block);
        nft.last_transfer_tx = Some(tx_index);
        // TODO - fetch and set json. Maybe in load_or_initialize
        // Approvals are unset on transfer.
        nft.approved = None;
        self.updates.nfts.insert(nft_id, nft);
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use dotenv::dotenv;
    use eth::types::{Address, Bytes32, NftId, U256};
    use event_retriever::db_reader::diesel::BlockRange;
    static TEST_SOURCE_URL: &str = "postgresql://postgres:postgres@localhost:5432/arak";
    static TEST_STORE_URL: &str = "postgresql://postgres:postgres@localhost:5432/store";
    static TEST_ETH_RPC: &str = "https://rpc.ankr.com/eth";

    fn test_handler() -> EventHandler {
        let mut handler = EventHandler::new(TEST_SOURCE_URL, TEST_STORE_URL, TEST_ETH_RPC).unwrap();
        handler.store.clear_tables();
        handler
    }

    #[tokio::test]
    #[ignore]
    async fn event_processing() {
        dotenv().ok();
        let mut handler = EventHandler::new(
            TEST_SOURCE_URL,
            TEST_STORE_URL,
            &std::env::var("NODE_URL").unwrap_or(TEST_ETH_RPC.to_string()),
        )
        .unwrap();
        let block = 15_000_000;
        assert!(handler
            .process_events_for_block_range(BlockRange {
                start: block,
                end: block + 5,
            })
            .await
            .is_ok());
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
        // Approval before token existance (handled the way the event said)
        handler.handle_erc721_approval(base, first_approval, &tx);
        let nft = handler.updates.nfts.get(&token).unwrap();
        assert_eq!(
            Address::expect_from(nft.clone().approved.unwrap()),
            approved,
            "first approval"
        );
        base.block_number += 1; // resuse incremented base.
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
                contract_address: base.contract_address.into(),
                token_id: token_id.into(),
                token_uri: None,
                owner: to.into(),
                last_update_block: 1,
                last_update_log_index: 2,
                last_transfer_block: Some(base.block_number as i64),
                last_transfer_tx: Some(base.transaction_index as i64),
                mint_block: base.block_number as i64,
                mint_tx: base.transaction_index as i64,
                burn_block: None,
                burn_tx: None,
                minter: tx.from.into(),
                approved: None,
                json: None
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
                contract_address: base.contract_address.into(),
                token_id: token_id.into(),
                token_uri: None,
                owner: from.into(),
                last_update_block: 4,
                last_update_log_index: 5,
                last_transfer_block: Some(base_2.block_number as i64),
                last_transfer_tx: Some(base_2.transaction_index as i64),
                mint_block: base.block_number as i64,
                mint_tx: base.transaction_index as i64,
                burn_block: None,
                burn_tx: None,
                minter: tx.from.into(),
                approved: None,
                json: None
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
                contract_address: base.contract_address.into(),
                token_id: token_id.into(),
                token_uri: None,
                owner: [0u8; 20].to_vec(),
                last_update_block: 7,
                last_update_log_index: 8,
                last_transfer_block: Some(base_3.block_number as i64),
                last_transfer_tx: Some(base_3.transaction_index as i64),
                mint_block: base.block_number as i64,
                mint_tx: base.transaction_index as i64,
                burn_block: Some(base_3.block_number as i64),
                burn_tx: Some(base_3.transaction_index as i64),
                minter: tx.from.into(),
                approved: None,
                json: None
            },
            "burn transfer"
        );

        // Idempotency: try to replay earlier transfers
        handler.handle_erc721_transfer(base, first_transfer, &tx);
        assert_eq!(
            handler.updates.nfts.get(&token).unwrap().owner,
            [0u8; 20].to_vec(),
            "idempotency"
        )
    }
}

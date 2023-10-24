use std::collections::HashMap;

use crate::{
    models::{Nft, NftId, Transaction},
    store::DataStore,
};
use anyhow::{Context, Result};
use eth::{
    rpc::{Client as EthClient, TxDetails},
    types::Address,
};
use event_retriever::db_reader::{
    diesel::{BlockRange, EventSource},
    models::*,
};

pub struct EventHandler {
    /// Source of events for processing
    source: EventSource,
    /// Location of existing stored content
    store: DataStore,
    /// This is a memory store of nft updates.
    nft_updates: HashMap<NftId, Nft>,
    /// Web3 Provider
    eth_client: EthClient,
}

impl EventHandler {
    pub fn new(source_url: &str, store_url: &str, eth_rpc: &str) -> Result<Self> {
        Ok(Self {
            source: EventSource::new(source_url).context("init EventSource")?,
            store: DataStore::new(store_url).context("init DataStore")?,
            nft_updates: HashMap::new(),
            eth_client: EthClient::new(eth_rpc).context("init EthClient")?,
        })
    }

    pub async fn process_events_for_block_range(&mut self, range: BlockRange) -> Result<()> {
        let event_map = self.source.get_events_for_block_range(range)?;
        let mut transaction_updates = Vec::new();
        for (block, block_events) in event_map.into_iter() {
            let tx_data = self
                .eth_client
                .get_block_receipts(block, block_events.keys().cloned().map(|k| k.0).collect())
                .await?;
            transaction_updates.extend(
                tx_data
                    .clone()
                    .into_iter()
                    .map(|(idx, data)| Transaction::new(block, idx, data))
                    .collect::<Vec<_>>(),
            );
            for ((tidx, _lidx), tx_events) in block_events {
                let tx = tx_data.get(&tidx).expect("receipt known to exist!");
                for NftEvent { base, meta } in tx_events.into_iter() {
                    match meta {
                        EventMeta::Erc721Approval(a) => self.handle_erc721_approval(base, a, tx),
                        EventMeta::Erc721Transfer(t) => self.handle_erc721_transfer(base, t, tx),
                        _ => unimplemented!("unhandled event!"),
                    };
                }
            }
        }

        self.store.save_transactions(transaction_updates);
        // drain memory into database.
        for (_, nft) in self.nft_updates.drain() {
            // TODO - Batch these updates.
            self.store.save_nft(&nft);
        }
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
        let mut nft = match self.nft_updates.remove(&nft_id) {
            Some(nft) => nft,
            None => match self.store.load_nft(&nft_id) {
                Some(nft) => nft,
                None => {
                    tracing::warn!("approval received before token mint {:?}", nft_id);
                    self.store.initialize_nft(&base, &nft_id, tx)
                }
            },
        };
        nft.approved = if approval.approved == Address::zero() {
            None
        } else {
            Some(approval.approved.into())
        };
        self.nft_updates.insert(nft_id, nft);
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

        let mut nft = match self.nft_updates.remove(&nft_id) {
            Some(nft) => nft,
            None => self.store.load_or_initialize_nft(&base, &nft_id, tx),
        };
        let EventBase {
            block_number,
            transaction_index,
            ..
        } = base;
        let block = block_number.try_into().expect("i64 block");
        let tx_index = transaction_index.try_into().expect("i64 block");

        if transfer.to == Address::zero() {
            // burn token
            nft.burn_block = Some(block);
            nft.burn_tx = Some(tx_index);
        }
        if transfer.from == Address::zero() {
            // Mint: This case is already handled by load_or_initialize
        }
        nft.owner = transfer.to.0.as_bytes().to_vec();
        nft.last_transfer_block = Some(block);
        nft.last_transfer_tx = Some(tx_index);
        // TODO - fetch and set json. Maybe in load_or_initialize
        // Approvals are unset on transfer.
        nft.approved = None;
        self.nft_updates.insert(nft_id, nft);
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::models::NftId;
    use eth::types::{Address, Bytes32, U256};
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
        let mut handler = EventHandler::new(TEST_SOURCE_URL, TEST_STORE_URL, TEST_ETH_RPC).unwrap();
        let block = 15_000_000;
        assert!(handler
            .process_events_for_block_range(BlockRange {
                start: block,
                end: block + 70,
            })
            .await
            .is_ok())
        // TODO - construct a sequence of events and actually check the Store State is as expected here.
    }

    #[test]
    fn erc721_approval_handler() {
        let mut handler = test_handler();
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
        let approved = Address::from(3);
        let approval = Erc721Approval {
            owner: Address::from(2),
            approved,
            id: token.token_id,
        };
        let tx = TxDetails {
            hash: Bytes32::from_str(
                "0xe9e91f1ee4b56c0df2e9f06c2b8c27c6076195a88a7b8537ba8313d80e6f124e",
            )
            .unwrap(),
            from: Address::from_str("0x32be343b94f860124dc4fee278fdcbd38c102d88").unwrap(),
            to: Some(Address::from_str("0xdf190dc7190dfba737d7777a163445b7fff16133").unwrap()),
        };
        // Approval before token existance (handled the way the event said)
        handler.handle_erc721_approval(base, approval, &tx);
        let nft = handler.nft_updates.get(&token).unwrap();
        assert_eq!(
            Address::expect_from(nft.clone().approved.unwrap()),
            approved
        );
        handler.handle_erc721_approval(
            base,
            Erc721Approval {
                owner: Address::from(2),
                approved: Address::zero(),
                id: token.token_id,
            },
            &tx,
        );
        let nft = handler.nft_updates.get(&token).unwrap();
        assert_eq!(nft.approved, None);
    }

    #[test]
    fn erc721_transfer_handler() {
        let mut handler = test_handler();
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
        let from = Address::from(2);
        let to = Address::from(3);
        let tx_from = Address::from(4);
        let transfer = Erc721Transfer { from, to, token_id };
        let tx = TxDetails {
            hash: Bytes32::from_str(
                "0xe9e91f1ee4b56c0df2e9f06c2b8c27c6076195a88a7b8537ba8313d80e6f124e",
            )
            .unwrap(),
            from: tx_from,
            to: Some(Address::from_str("0xdf190dc7190dfba737d7777a163445b7fff16133").unwrap()),
        };
        handler.handle_erc721_transfer(base, transfer, &tx);

        assert_eq!(
            handler.nft_updates.get(&token).unwrap(),
            &Nft {
                contract_address: contract_address.into(),
                token_id: token_id.into(),
                owner: to.into(),
                last_transfer_block: Some(base.block_number as i64),
                last_transfer_tx: Some(base.transaction_index as i64),
                mint_block: base.block_number as i64,
                mint_tx: base.transaction_index as i64,
                burn_block: None,
                burn_tx: None,
                minter: tx_from.into(),
                approved: None,
                json: None
            },
            "first transfer"
        );
        let base_2 = EventBase {
            block_number: 4,
            log_index: 5,
            transaction_index: 6,
            contract_address,
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
            handler.nft_updates.get(&token).unwrap(),
            &Nft {
                contract_address: contract_address.into(),
                token_id: token_id.into(),
                owner: from.into(),
                last_transfer_block: Some(base_2.block_number as i64),
                last_transfer_tx: Some(base_2.transaction_index as i64),
                mint_block: base.block_number as i64,
                mint_tx: base.transaction_index as i64,
                burn_block: None,
                burn_tx: None,
                minter: tx_from.into(),
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
            contract_address,
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
            handler.nft_updates.get(&token).unwrap(),
            &Nft {
                contract_address: contract_address.into(),
                token_id: token_id.into(),
                owner: [0u8; 20].to_vec(),
                last_transfer_block: Some(base_3.block_number as i64),
                last_transfer_tx: Some(base_3.transaction_index as i64),
                mint_block: base.block_number as i64,
                mint_tx: base.transaction_index as i64,
                burn_block: Some(base_3.block_number as i64),
                burn_tx: Some(base_3.transaction_index as i64),
                minter: tx_from.into(),
                approved: None,
                json: None
            },
            "burn transfer"
        );
    }
}

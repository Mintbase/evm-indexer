use crate::processor::EventProcessor;
use data_store::models::Erc1155;
use eth::types::{NftId, TxDetails, U256};
use event_retriever::db_reader::models::EventBase;

pub mod approval_for_all;
pub mod erc1155_transfer;
pub mod erc1155_uri;
pub mod erc721_approval;
pub mod erc721_transfer;

pub trait EventHandler<E> {
    fn handle_event(&mut self, base: EventBase, event: E, tx: &TxDetails);
}

impl EventProcessor {
    pub(crate) fn before_erc1155_event(
        &mut self,
        base: EventBase,
        id: U256,
        tx: &TxDetails,
    ) -> Option<Erc1155> {
        let nft_id = NftId {
            address: base.contract_address,
            token_id: id,
        };

        let mut token = self
            .updates
            .multi_tokens
            .remove(&nft_id)
            .unwrap_or_else(|| self.store.load_or_initialize_erc1155(&base, &nft_id, tx));
        if token.event_applied(&base) {
            tracing::warn!(
                "skipping attempt to replay event {:?} at tx {:?} on {:?}",
                base,
                tx.hash,
                nft_id
            );
            // Put the nft back in cache!
            self.updates.multi_tokens.insert(nft_id, token);
            return None;
        }

        token.last_update_block = base.block_number as i64;
        token.last_update_tx = base.transaction_index as i64;
        token.last_update_log_index = base.log_index as i64;

        Some(token)
    }
}

#[cfg(test)]
pub mod test_util {
    use crate::config::{ChainDataSource, HandlerConfig};
    use crate::processor::EventProcessor;
    use eth::types::{Address, Bytes32, NftId, TxDetails, U256};
    use event_retriever::db_reader::models::EventBase;
    use std::str::FromStr;

    static TEST_SOURCE_URL: &str = "postgresql://postgres:postgres@localhost:5432/arak";
    static TEST_STORE_URL: &str = "postgresql://postgres:postgres@localhost:5432/store";
    static TEST_ETH_RPC: &str = "https://rpc.ankr.com/eth";

    pub fn test_processor() -> EventProcessor {
        EventProcessor::new(
            TEST_SOURCE_URL,
            TEST_STORE_URL,
            TEST_ETH_RPC,
            HandlerConfig {
                chain_data_source: ChainDataSource::Database,
                page_size: 10,
                fetch_node_data: false,
                db_schema: "public".to_string(),
                uri_retry_blocks: 10,
                batch_delay: 1,
            },
            None,
        )
        .unwrap()
    }
    pub struct SetupData {
        pub handler: EventProcessor,
        // contract_address: Address,
        pub token_id: U256,
        pub token: NftId,
        pub base: EventBase,
        pub tx: TxDetails,
    }

    pub fn setup_data() -> SetupData {
        let handler = test_processor();
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
}

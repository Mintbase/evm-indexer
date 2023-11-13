use crate::{models::*, schema::*};
use anyhow::{Context, Result};
use diesel::{pg::PgConnection, prelude::*, Connection, RunQueryDsl};
use eth::types::{Address, BlockData, NftId, TxDetails};
use event_retriever::db_reader::models::EventBase;

pub struct DataStore {
    client: PgConnection,
}

fn handle_insert_result(result: QueryResult<usize>, expected_updates: usize, context: String) {
    match result {
        Ok(value) => {
            if value != expected_updates {
                tracing::warn!(
                    "unexpected update number for {} expected {} got {}",
                    context,
                    expected_updates,
                    value
                )
            }
        }
        Err(err) => {
            panic!("unhandled query result error on {}: {:?}", context, err)
        }
    }
}

fn handle_query_result<T>(result: QueryResult<T>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => {
            panic!("unhandled query result error: {:?}", err)
        }
    }
}

impl DataStore {
    fn establish_connection(db_url: &str) -> Result<PgConnection> {
        PgConnection::establish(db_url).context("Error connecting to Diesel Client")
    }

    pub fn new(connection: &str) -> Result<Self> {
        Ok(Self {
            client: Self::establish_connection(connection)?,
        })
    }

    pub fn save_transactions(&mut self, txs: Vec<Transaction>) {
        // These inserts must be broken into chunks because of:
        // DatabaseError(UnableToSendCommand, "number of parameters must be between 0 and 65535\n")
        let chunk_size = 10_000;
        tracing::info!(
            "saving {} EVM transactions over {} SQL transactions",
            txs.len(),
            (txs.len() / chunk_size) + 1
        );
        for chunk in txs.chunks(chunk_size) {
            let expected_inserts = chunk.len();
            let result = diesel::insert_into(transactions::dsl::transactions)
                .values(chunk.to_vec())
                .on_conflict((transactions::block_number, transactions::index))
                .do_nothing()
                .execute(&mut self.client);
            handle_insert_result(result, expected_inserts, "save_transactions".to_string())
        }
    }

    pub fn save_blocks(&mut self, blocks: Vec<BlockData>) {
        let chunk_size = 10_000;
        tracing::info!(
            "saving {} EVM blocks over {} SQL transactions",
            blocks.len(),
            (blocks.len() / chunk_size) + 1
        );
        for chunk in blocks.chunks(chunk_size) {
            let expected_inserts = chunk.len();
            let result = diesel::insert_into(blocks::dsl::blocks)
                .values(chunk.iter().map(Block::new).collect::<Vec<_>>())
                .on_conflict(blocks::number)
                .do_nothing()
                .execute(&mut self.client);
            handle_insert_result(result, expected_inserts, "save_blocks".to_string())
        }
    }

    pub fn save_nft(&mut self, nft: &Nft) {
        let result = diesel::insert_into(nfts::dsl::nfts)
            .values(nft)
            .on_conflict((nfts::contract_address, nfts::token_id))
            .do_update()
            .set(nft)
            .execute(&mut self.client);
        handle_insert_result(result, 1, format!("save_nft {:?}", nft))
    }

    pub fn save_contract(&mut self, contract: &TokenContract) {
        let result = diesel::insert_into(token_contracts::dsl::token_contracts)
            .values(contract)
            .on_conflict(token_contracts::address)
            .do_update()
            .set(contract)
            .execute(&mut self.client);
        handle_insert_result(result, 1, format!("save_contract {:?}", contract))
    }

    /// This method, as opposed to its singular counter part may be used under the assumption
    /// that the contracts are not being updated during event handling.
    pub fn save_contracts(&mut self, contracts: Vec<TokenContract>) {
        let expected_inserts = contracts.len();
        tracing::info!("saving {} contracts", expected_inserts);
        let result = diesel::insert_into(token_contracts::dsl::token_contracts)
            .values(contracts)
            .on_conflict(token_contracts::address)
            .do_nothing()
            .execute(&mut self.client);
        handle_insert_result(result, expected_inserts, "save_contracts".to_string())
    }

    pub fn set_approval_for_all(&mut self, approval: ApprovalForAll) {
        let result = diesel::insert_into(approval_for_all::dsl::approval_for_all)
            .values(&approval)
            .on_conflict((approval_for_all::contract_address, approval_for_all::owner))
            .do_update()
            .set(&approval)
            .execute(&mut self.client);
        handle_insert_result(result, 1, format!("set_approval_for_all {:?}", approval))
    }

    pub fn load_nft(&mut self, token: &NftId) -> Option<Nft> {
        let result = nfts::dsl::nfts
            .filter(nfts::contract_address.eq(&token.db_address()))
            .filter(nfts::token_id.eq(&token.db_token_id()))
            .first(&mut self.client)
            .optional();
        handle_query_result(result)
    }

    pub fn load_contract(&mut self, address: Address) -> Option<TokenContract> {
        let result = token_contracts::dsl::token_contracts
            .filter(token_contracts::address.eq::<&Vec<u8>>(&address.into()))
            .first(&mut self.client)
            .optional();
        handle_query_result(result)
    }

    pub fn load_or_initialize_nft(
        &mut self,
        base: &EventBase,
        nft_id: &NftId,
        tx: &TxDetails,
    ) -> Nft {
        match self.load_nft(nft_id) {
            Some(nft) => nft,
            None => {
                tracing::debug!("new nft {:?}", nft_id);
                Nft::build_from(base, nft_id, tx)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::contract_abis;
    use diesel::{QueryDsl, RunQueryDsl};
    use eth::types::{Address, Bytes32, TxDetails, U256};
    use event_retriever::db_reader::models::EventBase;

    static TEST_STORE_URL: &str = "postgresql://postgres:postgres@localhost:5432/store";

    fn get_new_store() -> DataStore {
        let mut store = DataStore::new(TEST_STORE_URL).unwrap();
        store.clear_tables();
        store
    }

    impl DataStore {
        pub fn clear_tables(&mut self) {
            diesel::delete(nfts::dsl::nfts)
                .execute(&mut self.client)
                .unwrap();
            diesel::delete(approval_for_all::dsl::approval_for_all)
                .execute(&mut self.client)
                .unwrap();
            diesel::delete(contract_abis::dsl::contract_abis)
                .execute(&mut self.client)
                .unwrap();
            diesel::delete(token_contracts::dsl::token_contracts)
                .execute(&mut self.client)
                .unwrap();
            diesel::delete(transactions::dsl::transactions)
                .execute(&mut self.client)
                .unwrap();
            diesel::delete(blocks::dsl::blocks)
                .execute(&mut self.client)
                .unwrap();
        }
    }

    fn test_event_base() -> EventBase {
        EventBase {
            block_number: 1,
            log_index: 2,
            transaction_index: 3,
            contract_address: Address::from(1),
        }
    }

    #[test]
    fn save_transactions() {
        let mut store = get_new_store();
        let details = TxDetails {
            hash: Bytes32::from(1),
            from: Address::from(1),
            to: Some(Address::from(2)),
        };
        // First call should not panic or log
        store.save_transactions(vec![
            Transaction::new(1, 2, details),
            Transaction::new(3, 4, details),
        ]);

        assert_eq!(
            Ok(2),
            transactions::dsl::transactions
                .count()
                .get_result(&mut store.client)
        );

        // This call will do nothing.
        store.save_transactions(vec![
            // Notice same (block, index) = (1, 2) as above.
            Transaction::new(1, 2, details),
        ]);
        assert_eq!(
            Ok(2),
            transactions::dsl::transactions
                .count()
                .get_result(&mut store.client)
        );
    }

    #[test]
    fn save_blocks() {
        let mut store = get_new_store();
        let blocks = vec![
            BlockData {
                number: 1,
                time: 123456789,
            },
            BlockData {
                number: 2,
                time: 234567891,
            },
            BlockData {
                number: 3,
                time: 345678912,
            },
            BlockData {
                number: 3,
                time: 345678912,
            },
        ];
        store.save_blocks(blocks);
        assert_eq!(
            Ok(3),
            blocks::dsl::blocks.count().get_result(&mut store.client)
        );
    }

    #[test]
    fn save_and_load_nft() {
        let mut store = get_new_store();
        let base = test_event_base();
        let token = NftId {
            address: base.contract_address,
            token_id: U256::from(123),
        };
        let tx = TxDetails {
            hash: Bytes32::from(1),
            from: Address::from(1),
            to: Some(Address::from(2)),
        };
        let nft = Nft::build_from(&base, &token, &tx);
        store.save_nft(&nft);
        assert_eq!(store.load_nft(&token).unwrap(), nft);
    }

    #[test]
    fn load_or_initialize_nft() {
        let mut store = get_new_store();
        let base = test_event_base();
        let token = NftId {
            address: base.contract_address,
            token_id: U256::from(123),
        };
        let tx = TxDetails {
            hash: Bytes32::from(1),
            from: Address::from(1),
            to: Some(Address::from(2)),
        };
        assert_eq!(
            store.load_or_initialize_nft(&base, &token, &tx),
            Nft::build_from(&base, &token, &tx)
        );
    }

    #[test]
    fn save_and_load_contract() {
        let mut store = get_new_store();
        let base = test_event_base();
        let contract = TokenContract::from_event_base(&base);
        assert!(store.load_contract(base.contract_address).is_none());
        store.save_contract(&contract);
        assert!(store.load_contract(base.contract_address).is_some());
    }
}

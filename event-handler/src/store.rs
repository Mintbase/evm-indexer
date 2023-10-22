use crate::{
    models::*,
    receipts::TxDetails,
    schema::{approval_for_all, nfts, token_contracts, transactions},
};
use anyhow::{Context, Result};
use diesel::{pg::PgConnection, prelude::*, Connection, RunQueryDsl};
use event_retriever::db_reader::models::EventBase;
use shared::eth::Address;

pub struct DataStore {
    client: PgConnection,
}

fn handle_insert_result(result: QueryResult<usize>, expected_updates: usize, context: String) {
    match result {
        Ok(value) => {
            if value != expected_updates {
                tracing::warn!(
                    "unexpected update number for {context} expected {expected_updates} got {value}",
                )
            }
        }
        Err(err) => {
            tracing::error!("execution error {:?}", err);
            panic!("unhandled query result error")
        }
    }
}

fn handle_query_result<T>(result: QueryResult<T>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => {
            tracing::error!("execution error {:?}", err);
            panic!("unhandled query result error")
        }
    }
}

impl DataStore {
    pub fn new(connection: &str) -> Result<Self> {
        Ok(Self {
            client: Self::establish_connection(connection)?,
        })
    }

    fn establish_connection(db_url: &str) -> Result<PgConnection> {
        PgConnection::establish(db_url).context("Error connecting to Diesel Client")
    }

    pub fn save_transactions(&mut self, txs: Vec<Transaction>) {
        let expected_inserts = txs.len();
        let result = diesel::insert_into(transactions::dsl::transactions)
            .values(txs)
            .on_conflict((transactions::block_number, transactions::index))
            .do_nothing()
            .execute(&mut self.client);
        handle_insert_result(result, expected_inserts, "save_transactions".to_string())
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
            .filter(nfts::token_id.eq(&token.db_id()))
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
        tx: TxDetails,
    ) -> Nft {
        // TODO - get Uri
        match self.load_nft(nft_id) {
            Some(nft) => nft,
            None => {
                tracing::debug!("new nft {:?}", nft_id);
                self.initialize_nft(base, nft_id, tx)
            }
        }
    }

    pub fn initialize_nft(&mut self, base: &EventBase, nft_id: &NftId, tx: TxDetails) -> Nft {
        // Check for contract (currently happening if new Nft is detected).
        // We may want a more efficient way to determine if a contract has
        // already been indexed.
        let _ = self.load_or_initialize_contract(base);
        // We don't save_nft yet, just construct and return.
        // User is reponsible to call save_nft.
        Nft::build_from(base, nft_id, tx)
    }

    pub fn load_or_initialize_contract(&mut self, base: &EventBase) -> TokenContract {
        match self.load_contract(base.contract_address) {
            Some(contract) => contract,
            None => {
                tracing::info!("new contract {:?}", base.contract_address);
                let contract = TokenContract::from_event_base(base);
                self.save_contract(&contract);
                contract
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        models::{Nft, TokenContract, Transaction},
        receipts::TxDetails,
        schema::{approval_for_all, contract_abis, nfts, token_contracts, transactions},
        store::{DataStore, NftId},
    };
    use diesel::{QueryDsl, RunQueryDsl};
    use ethers::types::{H160, H256};
    use event_retriever::db_reader::models::EventBase;
    use shared::eth::{Address, U256};

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
            hash: H256::from_low_u64_be(1),
            from: H160::from_low_u64_be(1),
            to: Some(H160::from_low_u64_be(2)),
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
    fn save_and_load_nft() {
        let mut store = get_new_store();
        let base = test_event_base();
        let token = NftId {
            address: base.contract_address,
            token_id: U256::from(123),
        };
        let tx = TxDetails {
            hash: H256::from_low_u64_be(1),
            from: Address::from(1).0,
            to: Some(Address::from(2).0),
        };
        let nft = Nft::build_from(&base, &token, tx);
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
            hash: H256::from_low_u64_be(1),
            from: Address::from(1).0,
            to: Some(Address::from(2).0),
        };
        assert_eq!(
            store.load_or_initialize_nft(&base, &token, tx),
            store.initialize_nft(&base, &token, tx)
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

    #[test]
    fn load_or_initialize_contract() {
        let mut store = get_new_store();
        let event = test_event_base();
        assert_eq!(
            store.load_or_initialize_contract(&event),
            TokenContract::from_event_base(&event)
        );
    }
}

use crate::{
    models::*,
    schema::{approval_for_all, nfts, token_contracts},
};
use anyhow::{anyhow, Context, Result};
use diesel::{pg::PgConnection, prelude::*, Connection, RunQueryDsl};
use event_retriever::db_reader::models::EventBase;
use shared::eth::{Address, U256};

pub struct DataStore {
    client: PgConnection,
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

    pub fn save_nft(&mut self, nft: &Nft) -> Result<()> {
        diesel::insert_into(nfts::dsl::nfts)
            .values(nft)
            .on_conflict((nfts::contract_address, nfts::token_id))
            .do_update()
            .set(nft)
            .execute(&mut self.client);
        Ok(())
    }

    pub fn load_nft(&mut self, token: &NftId) -> Result<Option<Nft>> {
        nfts::dsl::nfts
            .filter(nfts::contract_address.eq(&token.db_address()))
            .filter(nfts::token_id.eq(&token.db_id()))
            .first(&mut self.client)
            .optional()
            .context("load_nft")
    }

    pub fn load_or_initialize_nft(&mut self, base: &EventBase, nft_id: &NftId) -> Result<Nft> {
        match self.load_nft(nft_id)? {
            Some(nft) => Ok(nft),
            None => {
                tracing::debug!("new nft {:?}", nft_id);
                self.initialize_nft(base, nft_id)
            }
        }
    }

    pub fn initialize_nft(&mut self, base: &EventBase, nft_id: &NftId) -> Result<Nft> {
        // Check for contract (currently happening if new Nft is detected).
        // We may want a more efficient way to determine if a contract has
        // already been indexed.
        let _ = self.load_or_initialize_contract(base);
        let mut nft = Nft::build_from(base, nft_id);
        self.save_nft(&nft)?;
        Ok(nft)
    }

    pub fn save_contract(&mut self, contract: &TokenContract) -> Result<()> {
        diesel::insert_into(token_contracts::dsl::token_contracts)
            .values(contract)
            .on_conflict(token_contracts::address)
            .do_update()
            .set(contract)
            .execute(&mut self.client)?;
        Ok(())
    }

    pub fn load_contract(&mut self, address: Address) -> Option<TokenContract> {
        let contract: Option<TokenContract> = token_contracts::dsl::token_contracts
            .filter(token_contracts::address.eq::<&Vec<u8>>(&address.into()))
            .first(&mut self.client)
            .optional()
            .expect("load_contract");
        contract
    }

    pub fn load_or_initialize_contract(&mut self, base: &EventBase) -> Result<TokenContract> {
        match self.load_contract(base.contract_address) {
            Some(contract) => Ok(contract),
            None => {
                tracing::info!("new contract {:?}", base.contract_address);
                let contract = TokenContract::from_event_base(base);
                self.save_contract(&contract)?;
                Ok(contract)
            }
        }
    }

    pub fn set_approval_for_all(&mut self, approval: ApprovalForAll) -> Result<()> {
        diesel::insert_into(approval_for_all::dsl::approval_for_all)
            .values(&approval)
            .on_conflict((approval_for_all::contract_address, approval_for_all::owner))
            .do_update()
            .set(&approval)
            .execute(&mut self.client)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        models::{Nft, TokenContract},
        schema::{approval_for_all, contract_abis, nfts, token_contracts, transactions},
        store::{DataStore, NftId},
    };
    use diesel::RunQueryDsl;
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
    fn save_and_load_nft() {
        let mut store = get_new_store();
        let base = test_event_base();
        let token = NftId {
            address: base.contract_address,
            token_id: U256::from(123),
        };
        let nft = Nft::build_from(&base, &token);
        assert!(store.save_nft(&nft).is_ok());
        let loaded = store.load_nft(&token).unwrap().unwrap();
        assert_eq!(nft, loaded);
    }

    #[test]
    fn load_or_initialize_nft() {
        let mut store = get_new_store();
        let base = test_event_base();
        let token = NftId {
            address: base.contract_address,
            token_id: U256::from(123),
        };
        assert!(store.load_or_initialize_nft(&base, &token).is_ok());
    }

    #[test]
    fn save_and_load_contract() {
        let mut store = get_new_store();
        let base = test_event_base();
        let contract = TokenContract::from_event_base(&base);
        assert!(store.save_contract(&contract).is_ok());
        assert!(store.load_contract(base.contract_address).is_some());
    }

    #[test]
    fn load_or_initialize_contract() {
        let mut store = get_new_store();
        assert!(store
            .load_or_initialize_contract(&test_event_base())
            .is_ok());
    }
}

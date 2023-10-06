use crate::schema::nft_approvals;
use crate::schema::nfts::token_id;
use crate::{
    models::*,
    schema::{nfts, token_contracts},
};
use anyhow::{Context, Result};
use bigdecimal::{BigDecimal, Num};
use diesel::{pg::PgConnection, prelude::*, Connection, RunQueryDsl};
use ethers::types::Address;

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

    pub fn load_nft(&mut self, token: NftId) -> Option<Nft> {
        let nft: Option<Nft> = nfts::dsl::nfts
            .filter(nfts::contract_address.eq(&token.address.as_bytes().to_vec()))
            .filter(
                token_id.eq(&BigDecimal::from_str_radix(&token.token_id.to_string(), 10).unwrap()),
            )
            .first(&mut self.client)
            .optional()
            .expect("load_nft");
        nft
    }

    pub fn save_nft(&mut self, nft: Nft) -> Result<()> {
        diesel::insert_into(nfts::dsl::nfts)
            .values(&nft)
            .on_conflict((nfts::contract_address, nfts::token_id))
            .do_update()
            .set(&nft)
            .execute(&mut self.client)?;
        Ok(())
    }
    pub fn load_contract(&mut self, address: Address) -> Option<TokenContract> {
        let contract: Option<TokenContract> = token_contracts::dsl::token_contracts
            .filter(token_contracts::address.eq(&address.as_bytes().to_vec()))
            .first(&mut self.client)
            .optional()
            .expect("load_contract");
        contract
    }
    pub fn save_contract(&mut self, contract: TokenContract) -> Result<()> {
        diesel::insert_into(token_contracts::dsl::token_contracts)
            .values(&contract)
            .on_conflict(token_contracts::address)
            .do_update()
            .set(&contract)
            .execute(&mut self.client)?;
        Ok(())
    }

    pub fn set_approval(&mut self, approval: NftApproval) -> Result<()> {
        diesel::insert_into(nft_approvals::dsl::nft_approvals)
            .values(&approval)
            .on_conflict((nft_approvals::contract_address, nft_approvals::token_id))
            .do_update()
            .set(&approval)
            .execute(&mut self.client)?;
        Ok(())
    }
    pub fn clear_approval(&mut self, token: NftId) -> Result<()> {
        let result = diesel::delete(
            nft_approvals::dsl::nft_approvals
                .filter(nft_approvals::contract_address.eq(&token.address.as_bytes().to_vec()))
                .filter(
                    nft_approvals::token_id.eq(&BigDecimal::from_str_radix(
                        &token.token_id.to_string(),
                        10,
                    )
                    .unwrap()),
                ),
        )
        .execute(&mut self.client)?;
        println!("{:?}", result);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        models::NftApproval,
        schema::{
            approval_for_all, contract_abis, nft_approvals, nfts, token_contracts, transactions,
        },
        store::DataStore,
    };
    use diesel::RunQueryDsl;
    use ethers::types::{Address, U256};
    use event_retriever::db_reader::models::Erc721Approval;

    static TEST_STORE_URL: &str = "postgresql://postgres:postgres@localhost:5432/store";

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
            diesel::delete(nft_approvals::dsl::nft_approvals)
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
    #[test]
    fn store_and_clear_approval() {
        let mut store = DataStore::new(TEST_STORE_URL).unwrap();
        store.clear_tables();

        let contract_address = Address::from_low_u64_be(1);
        let approval = NftApproval::from_event(
            contract_address,
            Erc721Approval {
                owner: Address::from_low_u64_be(2),
                approved: Address::from_low_u64_be(3),
                id: U256::from(123),
            },
        );
        assert!(store.set_approval(approval).is_ok());
        // TODO - test set values are expected
        // TODO - test clear approval
    }
}

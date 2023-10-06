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
}

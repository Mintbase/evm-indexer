use crate::db_reader::{
    models::{db::DbErc721Transfer, Erc721Transfer},
    schema::erc721_transfer::dsl::{block_number, erc721_transfer},
};
use anyhow::{Context, Result};
use diesel::{pg::PgConnection, prelude::*, Connection};

pub struct DieselClient {
    client: PgConnection,
}

impl DieselClient {
    pub fn new(connection: &str) -> Result<Self> {
        Ok(Self {
            client: DieselClient::establish_connection(connection)?,
        })
    }

    fn establish_connection(db_url: &str) -> Result<PgConnection> {
        PgConnection::establish(db_url).context("Error connecting to Diesel Client")
    }

    // fn get_finalized_block(&mut self) -> Result<i64> {
    //     unimplemented!()
    // }
    pub fn get_erc721_transfers_for_block(
        &mut self,
        block: i64,
    ) -> Result<impl Iterator<Item = Erc721Transfer>> {
        let db_transfers: Vec<DbErc721Transfer> = erc721_transfer
            .filter(block_number.eq(&block))
            .load(&mut self.client)?;
        Ok(db_transfers.into_iter().map(|t| t.into()))
    }
}

#[cfg(test)]
mod tests {
    use crate::db_reader::diesel::DieselClient;

    static TEST_DB_URL: &str = "postgresql://postgres:postgres@localhost:5432/postgres";
    #[test]
    fn test_example() {
        let mut client = DieselClient::new(TEST_DB_URL).unwrap();
        let transfers = client.get_erc721_transfers_for_block(1001165).unwrap();
        assert!(!transfers.collect::<Vec<_>>().is_empty());
    }
}

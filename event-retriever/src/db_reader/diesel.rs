use crate::db_reader::{models::Erc721Transfer, schema::erc721_transfer::dsl::*, DBClient};
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
        PgConnection::establish(&db_url).context("Error connecting to Diesel Client")
    }
}

impl DBClient for DieselClient {
    fn get_finalized_block(&mut self) -> Result<i64> {
        unimplemented!()
    }
    fn get_erc721_transfers_for_block(&mut self, block: i64) -> Result<Vec<Erc721Transfer>> {
        Ok(erc721_transfer
            .filter(block_number.eq(&block))
            .load::<Erc721Transfer>(&mut self.client)?)
    }
}

#[cfg(test)]
mod tests {
    use crate::db_reader::diesel::DieselClient;
    use crate::db_reader::DBClient;

    static TEST_DB_URL: &str = "postgresql://postgres:postgres@localhost:5432/postgres";
    #[test]
    #[ignore]
    fn test_example() {
        let mut client = DieselClient::new(TEST_DB_URL).unwrap();
        let transfers = client.get_erc721_transfers_for_block(1001165).unwrap();
        assert!(!transfers.is_empty());
    }
}

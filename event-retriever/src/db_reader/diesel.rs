use crate::db_reader::{
    models::{
        db::{
            DbApprovalForAll, DbErc1155TransferSingle, DbErc1155Uri, DbErc721Approval,
            DbErc721Transfer,
        },
        ApprovalForAll, Erc1155TransferSingle, Erc1155Uri, Erc721Approval, Erc721Transfer,
    },
    schema::{
        self, approval_for_all::dsl::approval_for_all,
        erc1155_transfer_single::dsl::erc1155_transfer_single, erc1155_uri::dsl::erc1155_uri,
        erc721_approval::dsl::erc721_approval, erc721_transfer::dsl::erc721_transfer,
    },
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
    pub fn get_approvals_for_all_for_block(
        &mut self,
        block: i64,
    ) -> Result<impl Iterator<Item = ApprovalForAll>> {
        let events: Vec<DbApprovalForAll> = approval_for_all
            .filter(schema::approval_for_all::dsl::block_number.eq(&block))
            .load(&mut self.client)?;
        Ok(events.into_iter().map(|t| t.into()))
    }

    pub fn get_erc1155_transfers_single_for_block(
        &mut self,
        block: i64,
    ) -> Result<impl Iterator<Item = Erc1155TransferSingle>> {
        let events: Vec<DbErc1155TransferSingle> = erc1155_transfer_single
            .filter(schema::erc1155_transfer_single::dsl::block_number.eq(&block))
            .load(&mut self.client)?;
        Ok(events.into_iter().map(|t| t.into()))
    }

    pub fn get_erc1155_uri_for_block(
        &mut self,
        block: i64,
    ) -> Result<impl Iterator<Item = Erc1155Uri>> {
        let events: Vec<DbErc1155Uri> = erc1155_uri
            .filter(schema::erc1155_uri::dsl::block_number.eq(&block))
            .load(&mut self.client)?;
        Ok(events.into_iter().map(|t| t.into()))
    }

    pub fn get_erc721_approvals_for_block(
        &mut self,
        block: i64,
    ) -> Result<impl Iterator<Item = Erc721Approval>> {
        let events: Vec<DbErc721Approval> = erc721_approval
            .filter(schema::erc721_approval::dsl::block_number.eq(&block))
            .load(&mut self.client)?;
        Ok(events.into_iter().map(|t| t.into()))
    }
    pub fn get_erc721_transfers_for_block(
        &mut self,
        block: i64,
    ) -> Result<impl Iterator<Item = Erc721Transfer>> {
        let db_transfers: Vec<DbErc721Transfer> = erc721_transfer
            .filter(schema::erc721_transfer::dsl::block_number.eq(&block))
            .load(&mut self.client)?;
        Ok(db_transfers.into_iter().map(|t| t.into()))
    }
}

#[cfg(test)]
mod tests {
    use crate::db_reader::diesel::DieselClient;

    static TEST_DB_URL: &str = "postgresql://postgres:postgres@localhost:5432/postgres";

    fn test_client() -> DieselClient {
        DieselClient::new(TEST_DB_URL).unwrap()
    }

    #[test]
    fn approvals_for_all() {
        // select block_number, count(*) cnt
        // from approval_for_all
        // group by block_number
        // order by cnt desc
        // limit 1;
        let approvals = test_client()
            .get_approvals_for_all_for_block(10_000_788)
            .unwrap();
        assert!(!approvals.collect::<Vec<_>>().is_empty());
    }

    #[test]
    fn erc1155_transfer_single() {
        let transfer_singles = test_client()
            .get_erc1155_transfers_single_for_block(10_000_275)
            .unwrap();
        assert!(!transfer_singles.collect::<Vec<_>>().is_empty());
    }
    #[test]
    fn erc1155_uri() {
        let uris = test_client().get_erc1155_uri_for_block(10_000_380).unwrap();
        assert!(!uris.collect::<Vec<_>>().is_empty());
    }
    #[test]
    fn erc721_approvals() {
        let approvals = test_client()
            .get_erc721_approvals_for_block(10_000_002)
            .unwrap();
        assert!(!approvals.collect::<Vec<_>>().is_empty());
    }
    #[test]
    fn erc721_transfers() {
        let transfers = test_client()
            .get_erc721_transfers_for_block(1_001_165)
            .unwrap();
        assert!(!transfers.collect::<Vec<_>>().is_empty());
    }
}

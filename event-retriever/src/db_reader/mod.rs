use crate::db_reader::models::Erc721Transfer;
use anyhow::Result;
pub mod diesel;
mod models;
mod schema;

pub trait DBClient {
    fn get_finalized_block(&mut self) -> Result<i64>;
    fn get_erc721_transfers_for_block(&mut self, block_number: i64) -> Result<Vec<Erc721Transfer>>;
}

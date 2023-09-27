use anyhow::Result;

use self::models::Erc721Transfer;
pub mod diesel;
mod models;
mod schema;

pub trait DBClient {
    fn get_finalized_block(&mut self) -> Result<i64>;
    fn get_erc721_transfers_for_block(
        &mut self,
        block_number: i64,
    ) -> Result<Box<dyn Iterator<Item = Erc721Transfer>>>;
}

use crate::db_reader::schema::*;
use bigdecimal::BigDecimal;
use diesel::{Queryable, Selectable};

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = erc721_transfer)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Erc721Transfer {
    pub block_number: i64,
    log_index: i64,
    transaction_index: i64,
    address: Vec<u8>,
    from_0: Option<Vec<u8>>,
    to_1: Option<Vec<u8>>,
    // Had to install bigdecimal with serde feature and fix version to 0.3.1
    // https://docs.rs/bigdecimal/0.3.1/bigdecimal/index.html
    tokenid_2: Option<BigDecimal>,
}

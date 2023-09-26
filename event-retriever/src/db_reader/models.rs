use crate::db_reader::schema::*;
use bigdecimal::BigDecimal;
use diesel::{Queryable, Selectable};
use ethers::types::{Address, U256};

#[derive(Queryable, Selectable)]
#[diesel(table_name = erc721_transfer)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct DbErc721Transfer {
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

#[derive(Debug, PartialEq)]
pub struct Erc721Transfer {
    pub block_number: u64,
    pub log_index: u64,
    pub transaction_index: u64,
    pub contract_address: Address,
    pub from: Address,
    pub to: Address,
    // Had to install bigdecimal with serde feature and fix version to 0.3.1
    // https://docs.rs/bigdecimal/0.3.1/bigdecimal/index.html
    pub id: U256,
}

impl From<DbErc721Transfer> for Erc721Transfer {
    fn from(val: DbErc721Transfer) -> Self {
        Erc721Transfer {
            // these i64 fields are always non-negative.
            block_number: val.block_number.try_into().expect("negative block_number"),
            log_index: val.log_index.try_into().expect("negative log_index"),
            transaction_index: val
                .transaction_index
                .try_into()
                .expect("negative transaction_index"),
            contract_address: Address::from_slice(val.address.as_slice()),
            from: Address::from_slice(val.from_0.expect("Null from_0").as_slice()),
            to: Address::from_slice(val.to_1.expect("Null to_1").as_slice()),
            id: U256::from_dec_str(&val.tokenid_2.expect("Null token_id2").to_string())
                .expect("Invalid token_id"),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::str::FromStr;

    #[test]
    fn transfer_model_into() {
        let db_transfer = DbErc721Transfer {
            block_number: 1,
            log_index: 2,
            transaction_index: 3,
            address: [0u8; 20].to_vec(),
            from_0: Some([1u8; 20].to_vec()),
            to_1: Some([2u8; 20].to_vec()),
            tokenid_2: BigDecimal::parse_bytes(b"10", 10),
        };

        let transfer: Erc721Transfer = db_transfer.into();
        assert_eq!(
            transfer,
            Erc721Transfer {
                block_number: 1,
                log_index: 2,
                transaction_index: 3,
                contract_address: Address::from_str("0x0000000000000000000000000000000000000000")
                    .unwrap(),
                from: Address::from_str("0x0101010101010101010101010101010101010101").unwrap(),
                to: Address::from_str("0x0202020202020202020202020202020202020202").unwrap(),
                id: U256::from(10)
            }
        )
    }
}

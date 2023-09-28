use crate::db_reader::models::{Erc1155TransferSingle, Erc1155Uri, Erc721Approval};
use crate::db_reader::{
    models::{ApprovalForAll, Erc721Transfer, EventBase},
    schema::*,
};
use bigdecimal::BigDecimal;
use diesel::{Queryable, Selectable};
use ethers::types::{Address, U256};

#[macro_export]
macro_rules! event_base {
    ($val:tt) => {
        EventBase::try_new(
            $val.block_number,
            $val.log_index,
            $val.transaction_index,
            $val.address,
        )
        .expect("invalid input")
    };
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = approval_for_all)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct DbApprovalForAll {
    block_number: i64,
    log_index: i64,
    transaction_index: i64,
    address: Vec<u8>,
    owner_0: Option<Vec<u8>>,
    operator_1: Option<Vec<u8>>,
    approved_2: Option<bool>,
}

impl From<DbApprovalForAll> for ApprovalForAll {
    fn from(val: DbApprovalForAll) -> Self {
        ApprovalForAll {
            base: event_base!(val),
            owner: Address::from_slice(val.owner_0.expect("unexpected Null").as_slice()),
            operator: Address::from_slice(val.operator_1.expect("unexpected Null").as_slice()),
            approved: val.approved_2.expect("unexpected None"),
        }
    }
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = erc1155_transfer_single)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct DbErc1155TransferSingle {
    block_number: i64,
    log_index: i64,
    transaction_index: i64,
    address: Vec<u8>,
    operator_0: Option<Vec<u8>>,
    from_1: Option<Vec<u8>>,
    to_2: Option<Vec<u8>>,
    id_3: Option<BigDecimal>,
    value_4: Option<BigDecimal>,
}

impl From<DbErc1155TransferSingle> for Erc1155TransferSingle {
    fn from(val: DbErc1155TransferSingle) -> Self {
        Erc1155TransferSingle {
            base: event_base!(val),
            operator: Address::from_slice(val.operator_0.expect("unexpected Null").as_slice()),
            from: Address::from_slice(val.from_1.expect("unexpected Null").as_slice()),
            to: Address::from_slice(val.to_2.expect("unexpected Null").as_slice()),
            id: U256::from_dec_str(&val.id_3.expect("Null token_id2").to_string())
                .expect("Invalid token_id"),
            value: U256::from_dec_str(&val.value_4.expect("Null token_id2").to_string())
                .expect("Invalid value"),
        }
    }
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = erc1155_uri)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct DbErc1155Uri {
    block_number: i64,
    log_index: i64,
    transaction_index: i64,
    address: Vec<u8>,
    value_0: Option<String>,
    id_1: Option<BigDecimal>,
}

impl From<DbErc1155Uri> for Erc1155Uri {
    fn from(val: DbErc1155Uri) -> Self {
        Erc1155Uri {
            base: event_base!(val),
            id: U256::from_dec_str(&val.id_1.expect("Null id_1").to_string())
                .expect("Invalid value"),
            value: val.value_0.expect("Null value_0"),
        }
    }
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = erc721_approval)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct DbErc721Approval {
    block_number: i64,
    log_index: i64,
    transaction_index: i64,
    address: Vec<u8>,
    owner_0: Option<Vec<u8>>,
    approved_1: Option<Vec<u8>>,
    tokenid_2: Option<BigDecimal>,
}

impl From<DbErc721Approval> for Erc721Approval {
    fn from(val: DbErc721Approval) -> Self {
        Erc721Approval {
            base: event_base!(val),
            owner: Address::from_slice(val.owner_0.expect("unexpected Null").as_slice()),
            approved: Address::from_slice(val.approved_1.expect("unexpected Null").as_slice()),
            id: U256::from_dec_str(&val.tokenid_2.expect("Null tokenid_2").to_string())
                .expect("Invalid value"),
        }
    }
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = erc721_transfer)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct DbErc721Transfer {
    block_number: i64,
    log_index: i64,
    transaction_index: i64,
    address: Vec<u8>,
    from_0: Option<Vec<u8>>,
    to_1: Option<Vec<u8>>,
    // Had to install bigdecimal with serde feature and fix version to 0.3.1
    // https://docs.rs/bigdecimal/0.3.1/bigdecimal/index.html
    tokenid_2: Option<BigDecimal>,
}

impl From<DbErc721Transfer> for Erc721Transfer {
    fn from(val: DbErc721Transfer) -> Self {
        Erc721Transfer {
            base: event_base!(val),
            from: Address::from_slice(val.from_0.expect("Null from_0").as_slice()),
            to: Address::from_slice(val.to_1.expect("Null to_1").as_slice()),
            token_id: U256::from_dec_str(&val.tokenid_2.expect("Null token_id2").to_string())
                .expect("Invalid token_id"),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn n_addresses(n: u64) -> Vec<Address> {
        (0..n).map(|i| Address::from_low_u64_be(i)).collect()
    }
    #[test]
    fn approval_for_all_from_db() {
        let addresses = n_addresses(3);
        assert_eq!(
            ApprovalForAll::from(DbApprovalForAll {
                block_number: 1,
                log_index: 2,
                transaction_index: 3,
                address: addresses[0].as_fixed_bytes().to_vec(),
                owner_0: Some(addresses[1].as_fixed_bytes().to_vec()),
                operator_1: Some(addresses[2].as_fixed_bytes().to_vec()),
                approved_2: Some(true),
            }),
            ApprovalForAll {
                base: EventBase {
                    block_number: 1,
                    log_index: 2,
                    transaction_index: 3,
                    contract_address: addresses[0],
                },
                owner: addresses[1],
                operator: addresses[2],
                approved: true,
            }
        )
    }

    #[test]
    fn approval_from_db() {
        let addresses = n_addresses(3);
        assert_eq!(
            Erc721Approval::from(DbErc721Approval {
                block_number: 1,
                log_index: 2,
                transaction_index: 3,
                address: addresses[0].as_fixed_bytes().to_vec(),
                owner_0: Some(addresses[1].as_fixed_bytes().to_vec()),
                approved_1: Some(false),
                tokenid_2: None,
            }),
            Erc721Approval {
                base: EventBase {
                    block_number: 1,
                    log_index: 2,
                    transaction_index: 3,
                    contract_address: addresses[0],
                },
                owner: addresses[1],
                approved: true,
                id: (),
            }
        )
    }

    #[test]
    fn erc721_transfer_from_db() {
        let addresses = n_addresses(3);
        assert_eq!(
            Erc721Transfer::from(DbErc721Transfer {
                block_number: 1,
                log_index: 2,
                transaction_index: 3,
                address: addresses[0].as_fixed_bytes().to_vec(),
                from_0: Some(addresses[1].as_fixed_bytes().to_vec()),
                to_1: Some(addresses[2].as_fixed_bytes().to_vec()),
                tokenid_2: BigDecimal::parse_bytes(b"10", 10),
            }),
            Erc721Transfer {
                base: EventBase {
                    block_number: 1,
                    log_index: 2,
                    transaction_index: 3,
                    contract_address: addresses[0],
                },
                from: addresses[1],
                to: addresses[2],
                token_id: U256::from(10)
            }
        )
    }

    struct Test {
        block_number: i64,
        log_index: i64,
        transaction_index: i64,
        address: Vec<u8>,
    }

    #[test]
    fn event_base_marco() {
        let x = Test {
            block_number: 0,
            log_index: 0,
            transaction_index: 0,
            address: vec![0; 20],
        };

        assert_eq!(
            event_base!(x),
            EventBase {
                block_number: 0,
                log_index: 0,
                transaction_index: 0,
                contract_address: Address::zero()
            }
        )
    }

    #[test]
    #[should_panic]
    fn test_panic() {
        let x = Test {
            block_number: -1,
            log_index: 0,
            transaction_index: 0,
            address: vec![0; 20],
        };
        event_base!(x);
    }
}

use crate::db_reader::{
    models::{
        conversions::u256_from_big_decimal, ApprovalForAll, Erc1155TransferBatch,
        Erc1155TransferSingle, Erc1155Uri, Erc721Approval, Erc721Transfer, EventBase,
    },
    schema::*,
};
use bigdecimal::BigDecimal;
use diesel::{Queryable, QueryableByName, Selectable};
use ethers::types::{Address, U256};

pub trait EvmEventTable {
    fn block_number(&self) -> u64;
    fn log_index(&self) -> u64;
    fn transaction_index(&self) -> u64;
    fn address(&self) -> Address;

    fn event_base(&self) -> EventBase {
        EventBase {
            block_number: self.block_number(),
            log_index: self.log_index(),
            transaction_index: self.transaction_index(),
            contract_address: self.address(),
        }
    }
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
            owner: Address::from_slice(val.owner_0.expect("unexpected Null").as_slice()),
            operator: Address::from_slice(val.operator_1.expect("unexpected Null").as_slice()),
            approved: val.approved_2.expect("unexpected None"),
        }
    }
}
#[derive(Queryable, QueryableByName, Debug)]
pub(crate) struct DbErc1155TransferBatch {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    block_number: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    log_index: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    transaction_index: i64,
    #[diesel(sql_type = diesel::sql_types::Bytea)]
    address: Vec<u8>,
    #[diesel(sql_type = diesel::sql_types::Bytea)]
    operator: Vec<u8>,
    #[diesel(sql_type = diesel::sql_types::Bytea)]
    from: Vec<u8>,
    #[diesel(sql_type = diesel::sql_types::Bytea)]
    to: Vec<u8>,
    #[diesel(sql_type = diesel::sql_types::Array<diesel::sql_types::Numeric>)]
    ids: Vec<BigDecimal>,
    #[diesel(sql_type = diesel::sql_types::Array<diesel::sql_types::Numeric>)]
    values: Vec<BigDecimal>,
}
impl From<DbErc1155TransferBatch> for Erc1155TransferBatch {
    fn from(val: DbErc1155TransferBatch) -> Self {
        Erc1155TransferBatch {
            operator: Address::from_slice(val.operator.as_slice()),
            from: Address::from_slice(val.from.as_slice()),
            to: Address::from_slice(val.to.as_slice()),
            ids: val.ids.iter().map(u256_from_big_decimal).collect(),
            values: val.values.iter().map(u256_from_big_decimal).collect(),
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
            from: Address::from_slice(val.from_0.expect("Null from_0").as_slice()),
            to: Address::from_slice(val.to_1.expect("Null to_1").as_slice()),
            token_id: U256::from_dec_str(&val.tokenid_2.expect("Null token_id2").to_string())
                .expect("Invalid token_id"),
        }
    }
}

macro_rules! impl_evm_event_table {
    ($x:ident) => {
        impl EvmEventTable for $x {
            fn block_number(&self) -> u64 {
                self.block_number.try_into().expect("negative block_number")
            }
            fn log_index(&self) -> u64 {
                self.log_index.try_into().expect("negative log_index")
            }
            fn transaction_index(&self) -> u64 {
                self.transaction_index
                    .try_into()
                    .expect("negative transaction_index")
            }
            fn address(&self) -> Address {
                if self.address.len() != 20 {
                    panic!(
                        "Invalid Address bytes: {:?} - must have length 20",
                        self.address
                    );
                }
                Address::from_slice(self.address.as_slice())
            }
        }
    };
}

impl_evm_event_table!(DbApprovalForAll);
impl_evm_event_table!(DbErc1155TransferBatch);
impl_evm_event_table!(DbErc1155TransferSingle);
impl_evm_event_table!(DbErc1155Uri);
impl_evm_event_table!(DbErc721Approval);
impl_evm_event_table!(DbErc721Transfer);

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic;

    fn n_addresses(n: u64) -> Vec<Address> {
        (0..n).map(Address::from_low_u64_be).collect()
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
                owner: addresses[1],
                operator: addresses[2],
                approved: true,
            }
        )
    }

    #[test]
    fn erc1155_transfer_batch_from_db() {
        let addresses = n_addresses(4);
        assert_eq!(
            Erc1155TransferBatch::from(DbErc1155TransferBatch {
                block_number: 1,
                log_index: 2,
                transaction_index: 3,
                address: addresses[0].as_fixed_bytes().to_vec(),
                operator: addresses[1].as_fixed_bytes().to_vec(),
                from: addresses[2].as_fixed_bytes().to_vec(),
                to: addresses[3].as_fixed_bytes().to_vec(),
                ids: vec![BigDecimal::try_from(1).unwrap()],
                values: vec![BigDecimal::try_from(2).unwrap()],
            }),
            Erc1155TransferBatch {
                operator: addresses[1],
                from: addresses[2],
                to: addresses[3],
                ids: vec![U256::from(1)],
                values: vec![U256::from(2)],
            }
        )
    }

    #[test]
    fn erc1155_transfer_single_from_db() {
        let addresses = n_addresses(4);
        let id = 49;
        let value = 77;
        assert_eq!(
            Erc1155TransferSingle::from(DbErc1155TransferSingle {
                block_number: 1,
                log_index: 2,
                transaction_index: 3,
                address: addresses[0].as_fixed_bytes().to_vec(),
                operator_0: Some(addresses[1].as_fixed_bytes().to_vec()),
                from_1: Some(addresses[2].as_fixed_bytes().to_vec()),
                to_2: Some(addresses[3].as_fixed_bytes().to_vec()),
                id_3: Some(BigDecimal::try_from(id).unwrap()),
                value_4: Some(BigDecimal::try_from(value).unwrap()),
            }),
            Erc1155TransferSingle {
                operator: addresses[1],
                from: addresses[2],
                value: U256::from(value),
                id: U256::from(id),
                to: addresses[3],
            }
        )
    }
    #[test]
    fn erc1155_uri_from_db() {
        let addresses = n_addresses(3);
        let value = "TokenUri".to_string();
        let id = 49;
        assert_eq!(
            Erc1155Uri::from(DbErc1155Uri {
                block_number: 1,
                log_index: 2,
                transaction_index: 3,
                address: addresses[0].as_fixed_bytes().to_vec(),
                value_0: Some(value.clone()),
                id_1: Some(BigDecimal::try_from(id).unwrap())
            }),
            Erc1155Uri {
                value,
                id: U256::from(id),
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
                approved_1: Some(addresses[2].as_fixed_bytes().to_vec()),
                tokenid_2: Some(BigDecimal::parse_bytes(b"49", 10).unwrap()),
            }),
            Erc721Approval {
                owner: addresses[1],
                approved: addresses[2],
                id: U256::from(49),
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
                from: addresses[1],
                to: addresses[2],
                token_id: U256::from(10)
            }
        )
    }

    struct TestStruct {
        block_number: i64,
        log_index: i64,
        transaction_index: i64,
        address: Vec<u8>,
    }
    impl_evm_event_table!(TestStruct);

    #[test]
    fn test_impl_evm_event_base_panics() {
        assert!(panic::catch_unwind(|| {
            TestStruct {
                block_number: -1, // Bad Value!
                log_index: 0,
                transaction_index: 0,
                address: vec![0; 20],
            }
            .event_base();
        })
        .is_err());

        assert!(panic::catch_unwind(|| {
            TestStruct {
                block_number: 0,
                log_index: -1, // Bad Value!
                transaction_index: 0,
                address: vec![0; 20],
            }
            .event_base();
        })
        .is_err());

        assert!(panic::catch_unwind(|| {
            TestStruct {
                block_number: 0,
                log_index: 0,
                transaction_index: -1, // Bad Value!
                address: vec![0; 20],
            }
            .event_base();
        })
        .is_err());

        assert!(panic::catch_unwind(|| {
            TestStruct {
                block_number: 0,
                log_index: 0,
                transaction_index: 0,
                address: vec![1u8, 2u8], // Bad Value!
            }
            .event_base();
        })
        .is_err());
    }
}

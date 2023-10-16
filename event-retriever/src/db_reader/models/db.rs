use crate::db_reader::{
    models::{
        ApprovalForAll, Erc1155TransferBatch, Erc1155TransferSingle, Erc1155Uri, Erc721Approval,
        Erc721Transfer, EventBase,
    },
    schema::*,
};
use bigdecimal::BigDecimal;
use diesel::{Queryable, QueryableByName, Selectable};
use shared::eth::{Address, U256};

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
    owner_0: Vec<u8>,
    operator_1: Vec<u8>,
    approved_2: bool,
}

impl From<DbApprovalForAll> for ApprovalForAll {
    fn from(val: DbApprovalForAll) -> Self {
        ApprovalForAll {
            owner: Address::expect_from(val.owner_0),
            operator: Address::expect_from(val.operator_1),
            approved: val.approved_2,
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
            operator: Address::expect_from(val.operator),
            from: Address::expect_from(val.from),
            to: Address::expect_from(val.to),
            ids: val.ids.into_iter().map(U256::from).collect(),
            values: val.values.into_iter().map(U256::from).collect(),
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
    operator_0: Vec<u8>,
    from_1: Vec<u8>,
    to_2: Vec<u8>,
    id_3: BigDecimal,
    value_4: BigDecimal,
}

impl From<DbErc1155TransferSingle> for Erc1155TransferSingle {
    fn from(val: DbErc1155TransferSingle) -> Self {
        Erc1155TransferSingle {
            operator: Address::expect_from(val.operator_0),
            from: Address::expect_from(val.from_1),
            to: Address::expect_from(val.to_2),
            id: val.id_3.into(),
            value: val.value_4.into(),
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
    value_0: String,
    id_1: BigDecimal,
}

impl From<DbErc1155Uri> for Erc1155Uri {
    fn from(val: DbErc1155Uri) -> Self {
        Erc1155Uri {
            id: val.id_1.into(),
            value: val.value_0,
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
    owner_0: Vec<u8>,
    approved_1: Vec<u8>,
    tokenid_2: BigDecimal,
}

impl From<DbErc721Approval> for Erc721Approval {
    fn from(val: DbErc721Approval) -> Self {
        Erc721Approval {
            owner: Address::expect_from(val.owner_0),
            approved: Address::expect_from(val.approved_1),
            id: val.tokenid_2.into(),
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
    from_0: Vec<u8>,
    to_1: Vec<u8>,
    // Had to install bigdecimal with serde feature and fix version to 0.3.1
    // https://docs.rs/bigdecimal/0.3.1/bigdecimal/index.html
    tokenid_2: BigDecimal,
}

impl From<DbErc721Transfer> for Erc721Transfer {
    fn from(val: DbErc721Transfer) -> Self {
        Erc721Transfer {
            from: Address::expect_from(val.from_0),
            to: Address::expect_from(val.to_1),
            token_id: val.tokenid_2.into(),
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
                Address::expect_from(self.address.clone())
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
    use shared::eth::U256;
    use std::panic;

    fn n_addresses(n: u64) -> Vec<Address> {
        (0..n).map(Address::from).collect()
    }
    #[test]
    fn approval_for_all_from_db() {
        let addresses = n_addresses(3);
        assert_eq!(
            ApprovalForAll::from(DbApprovalForAll {
                block_number: 1,
                log_index: 2,
                transaction_index: 3,
                address: addresses[0].into(),
                owner_0: addresses[1].into(),
                operator_1: addresses[2].into(),
                approved_2: true,
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
                address: addresses[0].into(),
                operator: addresses[1].into(),
                from: addresses[2].into(),
                to: addresses[3].into(),
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
                address: addresses[0].into(),
                operator_0: addresses[1].into(),
                from_1: addresses[2].into(),
                to_2: addresses[3].into(),
                id_3: BigDecimal::try_from(id).unwrap(),
                value_4: BigDecimal::try_from(value).unwrap(),
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
                address: addresses[0].into(),
                value_0: value.clone(),
                id_1: BigDecimal::try_from(id).unwrap()
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
                address: addresses[0].into(),
                owner_0: addresses[1].into(),
                approved_1: addresses[2].into(),
                tokenid_2: BigDecimal::parse_bytes(b"49", 10).unwrap(),
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
                address: addresses[0].into(),
                from_0: addresses[1].into(),
                to_1: addresses[2].into(),
                tokenid_2: BigDecimal::parse_bytes(b"10", 10).unwrap(),
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

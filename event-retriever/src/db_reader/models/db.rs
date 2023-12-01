use crate::db_reader::{
    models::{
        ApprovalForAll, Erc1155TransferBatch, Erc1155TransferSingle, Erc1155Uri, Erc721Approval,
        Erc721Transfer, EventBase,
    },
    schema::*,
};
use diesel::internal::derives::multiconnection::chrono::NaiveDateTime;
use diesel::{Queryable, QueryableByName, Selectable};
use eth::types::{Address, Bytes32, U256};

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
    address: Address,
    owner_0: Address,
    operator_1: Address,
    approved_2: bool,
}

impl From<DbApprovalForAll> for ApprovalForAll {
    fn from(val: DbApprovalForAll) -> Self {
        ApprovalForAll {
            owner: val.owner_0,
            operator: val.operator_1,
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
    #[diesel(sql_type = eth::types::Address)]
    address: Address,
    #[diesel(sql_type = Address)]
    operator: Address,
    #[diesel(sql_type = Address)]
    from: Address,
    #[diesel(sql_type = Address)]
    to: Address,
    #[diesel(sql_type = diesel::sql_types::Array<diesel::sql_types::Numeric>)]
    ids: Vec<U256>,
    #[diesel(sql_type = diesel::sql_types::Array<diesel::sql_types::Numeric>)]
    values: Vec<U256>,
}
impl From<DbErc1155TransferBatch> for Erc1155TransferBatch {
    fn from(val: DbErc1155TransferBatch) -> Self {
        Erc1155TransferBatch {
            operator: val.operator,
            from: val.from,
            to: val.to,
            ids: val.ids,
            values: val.values,
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
    address: Address,
    operator_0: Address,
    from_1: Address,
    to_2: Address,
    id_3: U256,
    value_4: U256,
}

impl From<DbErc1155TransferSingle> for Erc1155TransferSingle {
    fn from(val: DbErc1155TransferSingle) -> Self {
        Erc1155TransferSingle {
            operator: val.operator_0,
            from: val.from_1,
            to: val.to_2,
            id: val.id_3,
            value: val.value_4,
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
    address: Address,
    value_0: String,
    id_1: U256,
}

impl From<DbErc1155Uri> for Erc1155Uri {
    fn from(val: DbErc1155Uri) -> Self {
        Erc1155Uri {
            id: val.id_1,
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
    address: Address,
    owner_0: Address,
    approved_1: Address,
    tokenid_2: U256,
}

impl From<DbErc721Approval> for Erc721Approval {
    fn from(val: DbErc721Approval) -> Self {
        Erc721Approval {
            owner: val.owner_0,
            approved: val.approved_1,
            id: val.tokenid_2,
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
    address: Address,
    from_0: Address,
    to_1: Address,
    // Had to install bigdecimal with serde feature and fix version to 0.3.1
    // https://docs.rs/bigdecimal/0.3.1/bigdecimal/index.html
    tokenid_2: U256,
}

impl From<DbErc721Transfer> for Erc721Transfer {
    fn from(val: DbErc721Transfer) -> Self {
        Erc721Transfer {
            from: val.from_0,
            to: val.to_1,
            token_id: val.tokenid_2,
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
                self.address
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

#[derive(Queryable, Selectable, Clone, Debug, PartialEq)]
#[diesel(table_name = transactions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Transaction {
    pub block_number: i64,
    pub index: i64,
    pub hash: Bytes32,
    pub from: Address,
    pub to: Option<Vec<u8>>,
}

#[derive(Queryable, Selectable, Clone)]
#[diesel(table_name = blocks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Block {
    pub number: i64,
    pub time: NaiveDateTime,
}

#[cfg(test)]
mod tests {
    use super::*;
    use eth::types::U256;
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
                address: addresses[0],
                owner_0: addresses[1],
                operator_1: addresses[2],
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
        let ids = vec![U256::from(1)];
        let values = vec![U256::from(2)];
        assert_eq!(
            Erc1155TransferBatch::from(DbErc1155TransferBatch {
                block_number: 1,
                log_index: 2,
                transaction_index: 3,
                address: addresses[0],
                operator: addresses[1],
                from: addresses[2],
                to: addresses[3],
                ids: ids.clone(),
                values: values.clone(),
            }),
            Erc1155TransferBatch {
                operator: addresses[1],
                from: addresses[2],
                to: addresses[3],
                ids,
                values,
            }
        )
    }

    #[test]
    fn erc1155_transfer_single_from_db() {
        let addresses = n_addresses(4);
        let id = U256::from(49);
        let value = U256::from(77);
        assert_eq!(
            Erc1155TransferSingle::from(DbErc1155TransferSingle {
                block_number: 1,
                log_index: 2,
                transaction_index: 3,
                address: addresses[0],
                operator_0: addresses[1],
                from_1: addresses[2],
                to_2: addresses[3],
                id_3: id,
                value_4: value,
            }),
            Erc1155TransferSingle {
                operator: addresses[1],
                from: addresses[2],
                to: addresses[3],
                value,
                id,
            }
        )
    }
    #[test]
    fn erc1155_uri_from_db() {
        let addresses = n_addresses(3);
        let value = "TokenUri".to_string();
        let id = U256::from(49);
        assert_eq!(
            Erc1155Uri::from(DbErc1155Uri {
                block_number: 1,
                log_index: 2,
                transaction_index: 3,
                address: addresses[0],
                value_0: value.clone(),
                id_1: id
            }),
            Erc1155Uri { value, id }
        )
    }
    #[test]
    fn approval_from_db() {
        let addresses = n_addresses(3);
        let id = U256::from(49);
        assert_eq!(
            Erc721Approval::from(DbErc721Approval {
                block_number: 1,
                log_index: 2,
                transaction_index: 3,
                address: addresses[0],
                owner_0: addresses[1],
                approved_1: addresses[2],
                tokenid_2: id,
            }),
            Erc721Approval {
                owner: addresses[1],
                approved: addresses[2],
                id,
            }
        )
    }

    #[test]
    fn erc721_transfer_from_db() {
        let addresses = n_addresses(3);
        let id = U256::from(10);
        assert_eq!(
            Erc721Transfer::from(DbErc721Transfer {
                block_number: 1,
                log_index: 2,
                transaction_index: 3,
                address: addresses[0],
                from_0: addresses[1],
                to_1: addresses[2],
                tokenid_2: id,
            }),
            Erc721Transfer {
                from: addresses[1],
                to: addresses[2],
                token_id: id
            }
        )
    }

    struct TestStruct {
        block_number: i64,
        log_index: i64,
        transaction_index: i64,
        address: Address,
    }
    impl_evm_event_table!(TestStruct);

    #[test]
    fn test_impl_evm_event_base_panics() {
        assert!(panic::catch_unwind(|| {
            TestStruct {
                block_number: -1, // Bad Value!
                log_index: 0,
                transaction_index: 0,
                address: Address::zero(),
            }
            .event_base();
        })
        .is_err());

        assert!(panic::catch_unwind(|| {
            TestStruct {
                block_number: 0,
                log_index: -1, // Bad Value!
                transaction_index: 0,
                address: Address::zero(),
            }
            .event_base();
        })
        .is_err());

        assert!(panic::catch_unwind(|| {
            TestStruct {
                block_number: 0,
                log_index: 0,
                transaction_index: -1, // Bad Value!
                address: Address::zero(),
            }
            .event_base();
        })
        .is_err());
    }
}

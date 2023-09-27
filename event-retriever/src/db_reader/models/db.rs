use crate::db_reader::models::{Erc1155TransferSingle, Erc1155Uri, Erc721Approval};
use crate::db_reader::{
    models::{ApprovalForAll, Erc721Transfer, EventBase},
    schema::*,
};
use bigdecimal::BigDecimal;
use diesel::{Queryable, Selectable};
use ethers::types::{Address, U256};

// #[derive(Queryable, Selectable)]
// #[diesel(table_name = _event_block)]
// #[diesel(check_for_backend(diesel::pg::Pg))]
// pub(crate) struct EventBlock {
//     event: String,
//     indexed: i64,
//     finalized: i64,
// }

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
            base: EventBase::try_new(
                val.block_number,
                val.log_index,
                val.transaction_index,
                val.address,
            )
            .expect("invalid input data"),
            owner: Address::from_slice(val.owner_0.expect("unexpected Null").as_slice()),
            operator: Address::from_slice(val.operator_1.expect("unexpected Null").as_slice()),
            approved: val.approved_2.expect("unexpected None"),
        }
    }
}

// #[derive(Queryable, Selectable)]
// #[diesel(table_name = erc1155_transfer_batch)]
// #[diesel(check_for_backend(diesel::pg::Pg))]
// pub(crate) struct DbErc1155TransferBatch {
//     block_number: i64,
//     log_index: i64,
//     transaction_index: i64,
//     address: Vec<u8>,
//     operator_0: Option<Vec<u8>>,
//     from_1: Option<Vec<u8>>,
//     to_2: Option<Vec<u8>>,
// }

// impl From<DbErc1155TransferBatch> for Erc1155TransferBatch {
//     fn from(val: DbErc1155TransferBatch) -> Self {
//         Erc1155TransferBatch {
//             base: EventBase::try_new(
//                 val.block_number,
//                 val.log_index,
//                 val.transaction_index,
//                 val.address,
//             )
//                 .expect("invalid input data"),
//             owner: (),
//             operator: (),
//             from: (),
//             to: (),
//             ids: vec![],
//             values: vec![],
//         }
//     }
// }

// #[derive(Queryable, Selectable)]
// #[diesel(table_name = erc1155_transfer_batch_ids_0)]
// #[diesel(check_for_backend(diesel::pg::Pg))]
// pub(crate) struct DbErc1155TransferBatchIds {
//     block_number: i64,
//     log_index: i64,
//     transaction_index: i64,
//     address: Vec<u8>,
//     array_index: i64,
//     ids_0: Option<BigDecimal>,
// }
// #[derive(Queryable, Selectable)]
// #[diesel(table_name = erc1155_transfer_batch_values_1)]
// #[diesel(check_for_backend(diesel::pg::Pg))]
// pub(crate) struct DbErc1155TransferBatchValues {
//     block_number: i64,
//     log_index: i64,
//     transaction_index: i64,
//     address: Vec<u8>,
//     array_index: i64,
//     values_0: Option<BigDecimal>,
// }

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
            base: EventBase::try_new(
                val.block_number,
                val.log_index,
                val.transaction_index,
                val.address,
            )
            .expect("invalid input data"),
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
            base: EventBase::try_new(
                val.block_number,
                val.log_index,
                val.transaction_index,
                val.address,
            )
            .expect("invalid input data"),
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
            base: EventBase::try_new(
                val.block_number,
                val.log_index,
                val.transaction_index,
                val.address,
            )
            .expect("invalid input data"),
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
            base: EventBase::try_new(
                val.block_number,
                val.log_index,
                val.transaction_index,
                val.address,
            )
            .expect("invalid input data"),
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
                base: EventBase {
                    block_number: 1,
                    log_index: 2,
                    transaction_index: 3,
                    contract_address: Address::from_str(
                        "0x0000000000000000000000000000000000000000"
                    )
                    .unwrap(),
                },
                from: Address::from_str("0x0101010101010101010101010101010101010101").unwrap(),
                to: Address::from_str("0x0202020202020202020202020202020202020202").unwrap(),
                token_id: U256::from(10)
            }
        )
    }
}

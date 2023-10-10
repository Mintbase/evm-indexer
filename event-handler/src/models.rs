use crate::schema::*;
use bigdecimal::BigDecimal;
use diesel::internal::derives::multiconnection::chrono::NaiveDateTime;
use diesel::{AsChangeset, Insertable, Queryable, Selectable};
use ethers::types::{Address, U256};
use serde_json::Value;
use std::fmt::Debug;

#[derive(Debug)]
pub struct NftId {
    pub address: Address,
    pub token_id: U256,
}

#[derive(Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = approval_for_all)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ApprovalForAll {
    contract_address: Vec<u8>,
    owner: Vec<u8>,
    operator: Vec<u8>,
    approved: bool,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = contract_abis)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct ContractAbi {
    address: Vec<u8>,
    abi: Option<Value>,
}

#[derive(Queryable, Selectable, Insertable, AsChangeset, PartialEq, Debug)]
#[diesel(table_name = nft_approvals)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NftApproval {
    contract_address: Vec<u8>,
    token_id: BigDecimal,
    approved: Vec<u8>,
}
#[derive(Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = nfts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Nft {
    contract_address: Vec<u8>,
    token_id: BigDecimal,
    owner: Vec<u8>,
    last_transfer_block: Option<i64>,
    last_transfer_tx: Option<i64>,
    // Todo - This should not be optional
    mint_block: Option<i64>,
    mint_tx: Option<i64>,
    burn_block: Option<i64>,
    burn_tx: Option<i64>,
    minter: Option<Vec<u8>>,
    json: Option<Value>,
}

#[derive(Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = token_contracts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TokenContract {
    pub address: Vec<u8>,
    // token_type: TokenType,
    name: Option<String>,
    symbol: Option<String>,
    decimals: Option<i16>,
    token_uri: Option<String>,
    created_block: i64,
    created_tx_index: i64,
    // content_flags -> Nullable<Array<Nullable<ContentFlag>>>,
    // content_category -> Nullable<Array<Nullable<ContentCategory>>>
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = transactions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct Transaction {
    block_number: i64,
    index: i64,
    hash: Vec<u8>,
    block_time: NaiveDateTime,
}

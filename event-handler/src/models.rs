use crate::schema::*;
use bigdecimal::BigDecimal;
use diesel::{
    internal::derives::multiconnection::chrono::NaiveDateTime, AsChangeset, Insertable, Queryable,
    Selectable,
};
use ethers::types::{Address, U256};
use event_retriever::db_reader::models::{
    conversions::*, ApprovalForAll as ApprovalForAllEvent, Erc721Approval, EventBase,
};
use serde_json::{Map, Value};
use std::fmt::Debug;

#[derive(Debug)]
pub struct NftId {
    pub address: Address,
    pub token_id: U256,
}

impl NftId {
    pub fn new(address: Vec<u8>, token_id: BigDecimal) -> Self {
        Self {
            address: Address::from_slice(address.as_slice()),
            token_id: u256_from_big_decimal(&token_id),
        }
    }
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

impl ApprovalForAll {
    pub fn from_event(contract_address: Address, event: ApprovalForAllEvent) -> Self {
        Self {
            contract_address: contract_address.as_bytes().to_vec(),
            owner: event.owner.as_bytes().to_vec(),
            approved: event.approved,
            operator: event.operator.as_bytes().to_vec(),
        }
    }
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

impl NftApproval {
    pub fn from_event(contract_address: Address, event: Erc721Approval) -> Self {
        // Note that event.owner is unused here.
        Self {
            contract_address: contract_address.as_bytes().to_vec(),
            token_id: big_decimal_from_u256(&event.id),
            approved: event.approved.as_bytes().to_vec(),
        }
    }
}
#[derive(Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = nfts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Nft {
    contract_address: Vec<u8>,
    token_id: BigDecimal,
    pub owner: Vec<u8>,
    pub last_transfer_block: Option<i64>,
    pub last_transfer_tx: Option<i64>,
    // Todo - This should not be optional
    mint_block: Option<i64>,
    mint_tx: Option<i64>,
    pub burn_block: Option<i64>,
    pub burn_tx: Option<i64>,
    pub minter: Option<Vec<u8>>,
    pub json: Option<Value>,
}

impl Nft {
    pub fn build_from(base: &EventBase, nft_id: &NftId) -> Self {
        Self {
            contract_address: nft_id.address.as_bytes().to_vec(),
            token_id: big_decimal_from_u256(&nft_id.token_id),
            owner: vec![],
            last_transfer_block: None,
            last_transfer_tx: None,
            // Maybe its best if we set this when transfer comes from Zero.
            mint_block: Some(base.block_number.try_into().expect("i64 block_number")),
            mint_tx: Some(
                base.transaction_index
                    .try_into()
                    .expect("i64 transaction_index"),
            ),
            burn_block: None,
            burn_tx: None,
            minter: None,
            json: None,
        }
    }

    pub fn update_json(&mut self, key: String, value: String) {
        let mut json = match self.json.clone().unwrap_or_default() {
            Value::Object(map) => map,
            _ => Map::new(),
        };
        json.insert(key, Value::String(value));
        self.json = Some(Value::Object(json));
    }
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

impl TokenContract {
    pub fn from_event_base(event: &EventBase) -> Self {
        Self {
            address: event.contract_address.as_bytes().to_vec(),
            // TODO - find these an put them.
            name: None,
            symbol: None,
            decimals: None,
            // TODO - this should be base_url
            token_uri: None,
            // assume that the first time a contract is seen is the created block
            created_block: event.block_number.try_into().expect("u64 conversion"),
            created_tx_index: event.transaction_index.try_into().expect("u64 conversion"),
        }
    }
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

use ethers::types::{Address, U256};
use std::cmp::Ordering;

mod conversions;
pub(crate) mod db;

#[derive(Debug, PartialEq)]
pub struct NftEvent {
    pub base: EventBase,
    pub meta: EventMeta,
}
#[derive(Debug, PartialEq)]
pub enum EventMeta {
    ApprovalForAll(ApprovalForAll),
    Erc1155TransferBatch(Erc1155TransferBatch),
    Erc1155TransferSingle(Erc1155TransferSingle),
    Erc1155Uri(Erc1155Uri),
    Erc721Approval(Erc721Approval),
    Erc721Transfer(Erc721Transfer),
}

/// Every Ethereum Event emits these properties
#[derive(Debug, PartialEq)]
pub struct EventBase {
    pub block_number: u64,
    pub log_index: u64,
    pub transaction_index: u64,
    pub contract_address: Address,
}

#[derive(Debug, PartialEq)]
pub struct ApprovalForAll {
    pub owner: Address,
    pub operator: Address,
    pub approved: bool,
}

#[derive(Debug, PartialEq)]
pub struct Erc1155TransferBatch {
    pub operator: Address,
    pub from: Address,
    pub to: Address,
    pub ids: Vec<U256>,
    pub values: Vec<U256>,
}

#[derive(Debug, PartialEq)]
pub struct Erc1155TransferSingle {
    pub operator: Address,
    pub from: Address,
    pub to: Address,
    pub id: U256,
    pub value: U256,
}

#[derive(Debug, PartialEq)]
pub struct Erc1155Uri {
    pub id: U256,
    pub value: String,
}

#[derive(Debug, PartialEq)]
pub struct Erc721Approval {
    pub owner: Address,
    pub approved: Address,
    pub id: U256,
}

#[derive(Debug, PartialEq)]
pub struct Erc721Transfer {
    pub from: Address,
    pub to: Address,
    pub token_id: U256,
}

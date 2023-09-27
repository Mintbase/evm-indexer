use anyhow::Result;
use ethers::types::{Address, U256};
pub(crate) mod db;

/// Every Ethereum Event emits these properties
#[derive(Debug, PartialEq)]
pub struct EventBase {
    pub block_number: u64,
    pub log_index: u64,
    pub transaction_index: u64,
    pub contract_address: Address,
}

impl EventBase {
    fn try_new(
        block_number: i64,
        log_index: i64,
        transaction_index: i64,
        address: Vec<u8>,
    ) -> Result<Self> {
        Ok(Self {
            block_number: block_number.try_into()?,
            log_index: log_index.try_into()?,
            transaction_index: transaction_index.try_into()?,
            contract_address: Address::from_slice(address.as_slice()),
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct ApprovalForAll {
    pub base: EventBase,
    pub owner: Address,
    pub operator: Address,
    pub approved: bool,
}

#[derive(Debug, PartialEq)]
pub struct Erc1155TransferBatch {
    pub base: EventBase,
    pub owner: Address,
    pub operator: Address,
    pub from: Address,
    pub to: Address,
    pub ids: Vec<U256>,
    pub values: Vec<U256>,
}

#[derive(Debug, PartialEq)]
pub struct Erc1155TransferSingle {
    pub base: EventBase,
    pub operator: Address,
    pub from: Address,
    pub to: Address,
    pub id: U256,
    pub value: U256,
}

#[derive(Debug, PartialEq)]
pub struct Erc1155Uri {
    pub base: EventBase,
    pub id: U256,
    pub value: String,
}

#[derive(Debug, PartialEq)]
pub struct Erc721Approval {
    pub base: EventBase,
    pub owner: Address,
    pub approved: Address,
    pub id: U256,
}

#[derive(Debug, PartialEq)]
pub struct Erc721Transfer {
    pub base: EventBase,
    pub from: Address,
    pub to: Address,
    pub token_id: U256,
}

// /// Our block query engine will output a sorted iterator of Events
// pub enum Event {
//     ApprovalForAll(ApprovalForAll),
//     Erc1155TransferBatch(Erc1155TransferBatch),
//     Erc1155TransferSingle(Erc1155TransferSingle),
//     Erc1155Uri(Erc1155Uri),
//     Erc721Approval(Erc721Approval),
//     Erc721Transfer(Erc721Transfer),
// }

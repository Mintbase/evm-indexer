use anyhow::{anyhow, Context, Result};
use ethers::types::{Address, U256};
mod conversions;
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
        if address.len() != 20 {
            return Err(anyhow!(
                "Invalid Address bytes: {:?} - must have length 20",
                address
            ));
        }
        Ok(Self {
            block_number: block_number.try_into().context("negative block_number")?,
            log_index: log_index.try_into().context("negative log_index")?,
            transaction_index: transaction_index
                .try_into()
                .context("negative transaction_index")?,
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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn event_base_try_new() {
        let address = vec![0; 20];
        assert_eq!(
            EventBase::try_new(-1, 0, 0, address.clone())
                .unwrap_err()
                .to_string(),
            "negative block_number"
        );

        assert_eq!(
            EventBase::try_new(0, -1, 0, address.clone())
                .unwrap_err()
                .to_string(),
            "negative log_index"
        );
        assert_eq!(
            EventBase::try_new(0, 0, -1, address.clone())
                .unwrap_err()
                .to_string(),
            "negative transaction_index"
        );

        assert_eq!(
            EventBase::try_new(0, 0, 0, vec![1u8, 2u8])
                .unwrap_err()
                .to_string(),
            "Invalid Address bytes: [1, 2] - must have length 20"
        );
    }
}

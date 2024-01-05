use crate::types::{Address, Bytes32, U256};
use bigdecimal::BigDecimal;
use diesel::{self, internal::derives::multiconnection::chrono::NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq, Clone, Default)]
pub struct BlockData {
    /// Block Number
    pub number: u64,
    /// Unix timestamp as 64-bit integer
    pub time: u64,
    pub transactions: HashMap<u64, TxDetails>,
}

impl BlockData {
    pub fn db_time(&self) -> NaiveDateTime {
        NaiveDateTime::from_timestamp_opt(self.time.try_into().expect("no crazy times"), 0)
            .expect("No crazy times plz")
    }
}
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize)]
pub struct NftId {
    pub address: Address,
    pub token_id: U256,
}

impl Display for NftId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.address.0, self.token_id.0)
    }
}

impl NftId {
    pub fn db_address(&self) -> Vec<u8> {
        self.address.into()
    }

    pub fn db_token_id(&self) -> BigDecimal {
        self.token_id.into()
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct TxDetails {
    pub hash: Bytes32,
    pub from: Address,
    pub to: Option<Address>,
}

impl From<ethrpc::types::SignedTransaction> for TxDetails {
    fn from(value: ethrpc::types::SignedTransaction) -> Self {
        TxDetails {
            hash: Bytes32::from(value.hash()),
            from: Address::from(value.from()),
            to: value.to().map(Address::from),
        }
    }
}

impl From<ethers::types::TransactionReceipt> for TxDetails {
    fn from(value: ethers::types::TransactionReceipt) -> Self {
        TxDetails {
            hash: Bytes32::from(value.transaction_hash),
            from: Address::from(value.from),
            to: value.to.map(Address::from),
        }
    }
}

impl From<ethers::types::Transaction> for TxDetails {
    fn from(value: ethers::types::Transaction) -> Self {
        TxDetails {
            hash: Bytes32::from(value.hash),
            from: Address::from(value.from),
            to: value.to.map(Address::from),
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct ContractDetails {
    pub address: Address,
    pub name: Option<String>,
    pub symbol: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn impl_block() {
        let block = BlockData {
            number: 10_000_000,
            time: 1588598533,
            ..Default::default()
        };
        assert_eq!(
            block.db_time(),
            NaiveDateTime::from_str("2020-05-04T13:22:13").unwrap()
        )
    }
}

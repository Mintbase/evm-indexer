use crate::types::{Address, Bytes32, U256};
use bigdecimal::BigDecimal;
use diesel::{self, internal::derives::multiconnection::chrono::NaiveDateTime};
use serde::{Deserialize, Serialize};
use solabi::ethprim::ParseAddressError;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::num::ParseIntError;
use std::str::FromStr;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct BlockData {
    /// Block Number
    pub number: u64,
    /// Unix timestamp as 64-bit integer
    pub time: u64,
    pub transactions: HashMap<u64, TxDetails>,
}

impl Eq for BlockData {}

// Implement Hash based solely on the 'number' field
impl std::hash::Hash for BlockData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.number.hash(state);
    }
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseTokenError {
    /// The string does not have the correct format.
    InvalidFormat,
    Address(ParseAddressError),
    Id(ParseIntError),
}

impl FromStr for NftId {
    type Err = ParseTokenError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Format is Address/TokenId (i.e. 0xDEADBEEF.../123)
        let parts: Vec<_> = s.split('/').collect();
        if parts.len() != 2 {
            return Err(Self::Err::InvalidFormat);
        }
        let address = Address::from_str(parts[0]).map_err(Self::Err::Address)?;
        let token_id = U256::from_dec_str(parts[1]).map_err(Self::Err::Id)?;
        Ok(Self { address, token_id })
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

    #[test]
    fn nft_id_from_str() {
        let valid_id_str =
            "64671196571681841248190411691641946869002480279128285790058847953168666315";
        let valid_address_str = "0x57F1887A8BF19B14FC0DF6FD9B2ACC9AF147EA85";
        let valid_token_string = format!("{}/{}", valid_address_str, valid_id_str);
        // Ok
        assert_eq!(
            NftId::from_str(&valid_token_string).unwrap(),
            NftId {
                address: Address::from_str(valid_address_str).unwrap(),
                token_id: U256::from_dec_str(valid_id_str).unwrap(),
            }
        );
        // Error
        let token_str = format!("0xDEADBEEF/{}", valid_id_str); // Too short address
        assert_eq!(
            NftId::from_str(&token_str).unwrap_err(),
            ParseTokenError::Address(ParseAddressError::InvalidLength)
        );
        let token_str = format!("{}//{}", valid_address_str, valid_id_str); // too many '/'s
        assert_eq!(
            NftId::from_str(&token_str).unwrap_err(),
            ParseTokenError::InvalidFormat
        );

        let token_str = format!("{}/999{}", valid_address_str, U256::MAX); // U256 Overflow
        if let ParseTokenError::Id(int_error) = NftId::from_str(&token_str).unwrap_err() {
            assert_eq!(
                int_error.to_string(),
                "number too large to fit in target type"
            )
        }

        let token_str = format!("{}/NotANumber", valid_address_str); // Different Parse Int Error.
        if let ParseTokenError::Id(int_error) = NftId::from_str(&token_str).unwrap_err() {
            assert_eq!(int_error.to_string(), "invalid digit found in string")
        }
    }
}

use bigdecimal::{BigDecimal, Num};
use ethers::{
    abi::ethereum_types::FromDecStrErr,
    types::{H160, U256 as Uint256},
};
use std::str::FromStr;

/// An address. Can be an EOA or a smart contract address.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Address(pub H160);

impl Address {
    pub fn zero() -> Self {
        Self(H160::zero())
    }

    /// ! WARNING! This function is meant to be used by Diesel
    /// for Ethereum address fields encoded in postgres
    /// as BYTEA type (since there is no fixed length type)
    pub fn expect_from(value: Vec<u8>) -> Self {
        Self::try_from(value).expect("address from vec")
    }
}

impl From<Address> for Vec<u8> {
    fn from(value: Address) -> Self {
        value.0.as_bytes().to_vec()
    }
}

impl TryFrom<Vec<u8>> for Address {
    type Error = (&'static str, Vec<u8>);

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() == 20 {
            Ok(Self(H160::from_slice(value.as_slice())))
        } else {
            Err(("Address bytes must have length 20!", value))
        }
    }
}

impl TryFrom<Option<Vec<u8>>> for Address {
    type Error = (&'static str, Vec<u8>);

    fn try_from(value: Option<Vec<u8>>) -> Result<Self, Self::Error> {
        if let Some(addr) = value {
            addr.try_into()
        } else {
            Err(("Unexpected Null", vec![]))
        }
    }
}

impl From<H160> for Address {
    fn from(value: H160) -> Self {
        Self(value)
    }
}

impl FromStr for Address {
    type Err = rustc_hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match H160::from_str(s) {
            Ok(res) => Ok(Address(res)),
            Err(err) => Err(err),
        }
    }
}

impl From<u64> for Address {
    fn from(value: u64) -> Self {
        Address(H160::from_low_u64_be(value))
    }
}

impl From<[u8; 20]> for Address {
    fn from(value: [u8; 20]) -> Self {
        Address(H160::from(value))
    }
}

impl From<Address> for H160 {
    fn from(value: Address) -> Self {
        value.0
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct U256(pub Uint256);

impl From<BigDecimal> for U256 {
    fn from(val: BigDecimal) -> Self {
        U256(Uint256::from_dec_str(&val.to_string()).expect("Invalid value"))
    }
}

impl From<U256> for BigDecimal {
    fn from(value: U256) -> Self {
        BigDecimal::from_str_radix(&value.0.to_string(), 10).unwrap()
    }
}

impl From<u64> for U256 {
    fn from(value: u64) -> Self {
        U256(Uint256::from(value))
    }
}

impl U256 {
    pub fn from_dec_str(value: &str) -> Result<Self, FromDecStrErr> {
        match Uint256::from_dec_str(value) {
            Ok(res) => Ok(U256(res)),
            Err(err) => Err(err),
        }
    }
}

use std::str::FromStr;

use ethers::types::H160;

/// An address. Can be an EOA or a smart contract address.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Address(pub H160);

impl Address {
    pub fn zero() -> Self {
        Self(H160::zero())
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

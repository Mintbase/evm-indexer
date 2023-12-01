use diesel::{
    self,
    deserialize::{self, FromSql},
    pg::{Pg, PgValue},
    sql_types::{Binary, SqlType},
    Queryable,
};
use ethrpc::types::Digest as H256;
use serde::Serialize;
use solabi::ethprim::ParseDigestError;
use std::str::FromStr;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, SqlType)]
#[diesel(postgres_type(name = "Bytes32"))]
pub struct Bytes32(pub H256);

impl Serialize for Bytes32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("0x{:x}", self.0))
    }
}

impl FromSql<Bytes32, Pg> for Bytes32 {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        Bytes32::try_from(bytes.as_bytes().to_vec()).map_err(|(message, _)| message.into())
    }
}

impl Queryable<Binary, Pg> for Bytes32 {
    type Row = Vec<u8>;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        row.try_into().map_err(|(x, _): (&str, _)| x.into())
    }
}

impl Bytes32 {
    pub fn zero() -> Self {
        Self(H256([0; 32]))
    }

    /// ! WARNING! This function is meant to be used by Diesel
    /// for Ethereum address fields encoded in postgres
    /// as BYTEA type (since there is no fixed length type)
    pub fn expect_from(value: Vec<u8>) -> Self {
        Self::try_from(value).expect("address from vec")
    }
}

impl From<Bytes32> for Vec<u8> {
    fn from(value: Bytes32) -> Self {
        value.0.as_slice().to_vec()
    }
}

impl TryFrom<Vec<u8>> for Bytes32 {
    type Error = (&'static str, Vec<u8>);

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() == 32 {
            Ok(Self(H256::from_slice(value.as_slice())))
        } else {
            Err(("Bytes32 bytes must have length 32!", value))
        }
    }
}

impl TryFrom<Option<Vec<u8>>> for Bytes32 {
    type Error = (&'static str, Vec<u8>);

    fn try_from(value: Option<Vec<u8>>) -> Result<Self, Self::Error> {
        if let Some(hash) = value {
            hash.try_into()
        } else {
            Err(("Unexpected Null", vec![]))
        }
    }
}

impl From<H256> for Bytes32 {
    fn from(value: H256) -> Self {
        Self(value)
    }
}

impl From<ethers::types::H256> for Bytes32 {
    fn from(value: ethers::types::H256) -> Self {
        Self::from(value.0)
    }
}

impl FromStr for Bytes32 {
    type Err = ParseDigestError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match H256::from_str(s) {
            Ok(res) => Ok(Bytes32(res)),
            Err(err) => Err(err),
        }
    }
}

/// This is only useful for testing!
impl From<u64> for Bytes32 {
    fn from(value: u64) -> Self {
        let mut new_array: [u8; 32] = [0; 32];
        new_array[24..].copy_from_slice(&value.to_be_bytes());
        Self(H256(new_array))
    }
}

impl From<[u8; 32]> for Bytes32 {
    fn from(value: [u8; 32]) -> Self {
        Bytes32(H256(value))
    }
}

impl From<Bytes32> for H256 {
    fn from(value: Bytes32) -> Self {
        value.0
    }
}

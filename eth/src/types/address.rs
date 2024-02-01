use diesel::{
    self,
    backend::Backend,
    deserialize::{self, FromSql},
    pg::{Pg, PgValue},
    serialize::ToSql,
    sql_types::{Binary, SqlType},
    Expression, Queryable,
};
use ethrpc::types::Address as H160;
use serde::{Deserialize, Serialize};
use solabi::ethprim::ParseAddressError;
use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

/// ENS registry address (`0x57F1887A8BF19B14FC0DF6FD9B2ACC9AF147EA85`)
pub const ENS_ADDRESS: Address = Address(H160([
    87, 241, 136, 122, 139, 241, 155, 20, 252, 13, 246, 253, 155, 42, 204, 154, 241, 71, 234, 133,
]));

/// An address. Can be an EOA or a smart contract address.
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, SqlType, Deserialize)]
pub struct Address(pub H160);

impl Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Debug for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Address")
            .field(&format_args!("{}", self.0))
            .finish()
    }
}

impl FromSql<Address, Pg> for Address {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        Ok(Address::from(bytes.as_bytes().to_vec()))
    }
}

impl Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("0x{:x}", self.0))
    }
}

/// ! WARNING! This function is meant to be used by Diesel
/// for Ethereum address fields encoded in postgres
/// as BYTEA type (since there is no fixed length type)
impl From<Vec<u8>> for Address {
    fn from(value: Vec<u8>) -> Self {
        Self(H160::from_slice(value.as_slice()))
    }
}

impl<DB> ToSql<Binary, DB> for Address
where
    DB: Backend,
    [u8]: ToSql<Binary, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        self.0 .0.as_slice().to_sql(out)
    }
}

impl Expression for Address {
    type SqlType = Binary;
}

impl Queryable<Binary, Pg> for Address {
    type Row = Vec<u8>;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row.into())
    }
}

impl Address {
    pub fn zero() -> Self {
        Self(H160([0; 20]))
    }
}

impl From<Address> for Vec<u8> {
    fn from(value: Address) -> Self {
        value.0.as_slice().to_vec()
    }
}

impl From<ethers::types::Address> for Address {
    fn from(value: ethers::types::Address) -> Self {
        Self::from(value.0)
    }
}

impl From<H160> for Address {
    fn from(value: H160) -> Self {
        Self(value)
    }
}

impl TryFrom<Option<Vec<u8>>> for Address {
    type Error = (&'static str, Vec<u8>);

    fn try_from(value: Option<Vec<u8>>) -> Result<Self, Self::Error> {
        if let Some(addr) = value {
            Ok(addr.into())
        } else {
            Err(("Unexpected Null", vec![]))
        }
    }
}

impl FromStr for Address {
    type Err = ParseAddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match H160::from_str(s) {
            Ok(res) => Ok(Address(res)),
            Err(err) => Err(err),
        }
    }
}

/// This is a lazy constructor only for testing.
impl From<u64> for Address {
    fn from(value: u64) -> Self {
        let mut new_array: [u8; 20] = [0; 20];
        new_array[12..].copy_from_slice(&value.to_be_bytes());
        Self(H160(new_array))
    }
}

impl From<[u8; 20]> for Address {
    fn from(value: [u8; 20]) -> Self {
        Self(H160(value))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn addresses() {
        let ens_contract = Address::from_str("0x57f1887a8bf19b14fc0df6fd9b2acc9af147ea85").unwrap();
        assert_eq!(
            ens_contract.0.as_slice().to_vec(),
            [
                87, 241, 136, 122, 139, 241, 155, 20, 252, 13, 246, 253, 155, 42, 204, 154, 241,
                71, 234, 133
            ]
        );
    }
}

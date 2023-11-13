use diesel::{
    self,
    deserialize::{self, FromSql},
    pg::{Pg, PgValue},
    sql_types::{Binary, SqlType},
    Queryable,
};
use ethrpc::types::Address as H160;
use solabi::ethprim::ParseAddressError;
use std::str::FromStr;

/// An address. Can be an EOA or a smart contract address.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, SqlType)]
#[diesel(postgres_type(name = "Address"))]
pub struct Address(pub H160);

impl FromSql<Address, Pg> for Address {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        Address::try_from(bytes.as_bytes().to_vec()).map_err(|(message, _)| message.into())
    }
}

impl Queryable<Binary, Pg> for Address {
    type Row = Vec<u8>;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        row.try_into().map_err(|(x, _): (&str, _)| x.into())
    }
}

impl Address {
    pub fn zero() -> Self {
        Self(H160([0; 20]))
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

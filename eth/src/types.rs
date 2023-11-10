use bigdecimal::{BigDecimal, Num};
use diesel::{
    self,
    data_types::PgNumeric,
    deserialize::{self, FromSql},
    internal::derives::multiconnection::chrono::NaiveDateTime,
    pg::{Pg, PgValue},
    sql_types::{Binary, Numeric, SqlType},
    Queryable,
};
// use ethers::types::H256;
use ethrpc::types::{Address as H160, Digest as H256, U256 as Uint256};
use solabi::ethprim::{ParseAddressError, ParseDigestError};
use std::{num::ParseIntError, str::FromStr};

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
    type Err = ParseAddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match H160::from_str(s) {
            Ok(res) => Ok(Address(res)),
            Err(err) => Err(err),
        }
    }
}

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

impl From<Address> for H160 {
    fn from(value: Address) -> Self {
        value.0
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, SqlType, Hash)]
#[diesel(postgres_type(name = "U256"))]
pub struct U256(pub Uint256);

impl FromSql<Numeric, Pg> for U256 {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let big_decimal: BigDecimal = PgNumeric::from_sql(bytes)?.try_into()?;
        Ok(U256::from(big_decimal))
    }
}

impl Queryable<Numeric, Pg> for U256 {
    type Row = BigDecimal;

    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row.into())
    }
}

impl From<BigDecimal> for U256 {
    fn from(val: BigDecimal) -> Self {
        U256(Uint256::from_str(&val.to_string()).expect("Invalid value"))
    }
}

impl From<U256> for BigDecimal {
    fn from(value: U256) -> Self {
        BigDecimal::from_str_radix(&value.0.to_string(), 10).unwrap()
    }
}

impl From<ethers::types::U256> for U256 {
    fn from(value: ethers::types::U256) -> Self {
        // ethrpc uses [u128; 2] for U256 and ethers uses [u64; 4] we convert between the two.
        let arr = value.0;
        let first_u128 = ((arr[1] as u128) << 64) | (arr[0] as u128);
        let second_u128 = ((arr[3] as u128) << 64) | (arr[2] as u128);
        Self(Uint256([first_u128, second_u128]))
    }
}

impl From<u64> for U256 {
    fn from(value: u64) -> Self {
        U256(Uint256::from(value))
    }
}

impl U256 {
    pub fn from_dec_str(value: &str) -> Result<Self, ParseIntError> {
        match Uint256::from_str(value) {
            Ok(res) => Ok(U256(res)),
            Err(err) => Err(err),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, SqlType)]
#[diesel(postgres_type(name = "Bytes32"))]
pub struct Bytes32(pub H256);

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

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct NftId {
    pub address: Address,
    pub token_id: U256,
}

impl NftId {
    pub fn db_address(&self) -> Vec<u8> {
        self.address.into()
    }

    pub fn db_token_id(&self) -> BigDecimal {
        self.token_id.into()
    }
}
#[derive(Debug, PartialEq)]
pub struct BlockData {
    /// Block Number
    pub number: u64,
    /// Unix timestamp as 64-bit integer
    pub time: u64,
}

impl BlockData {
    pub fn db_time(&self) -> NaiveDateTime {
        NaiveDateTime::from_timestamp_opt(self.time.try_into().expect("no crazy times"), 0)
            .expect("No crazy times plz")
    }
}

#[derive(PartialEq, Debug)]
pub struct ContractDetails {
    pub name: Option<String>,
    pub symbol: Option<String>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct TxDetails {
    pub hash: Bytes32,
    pub from: Address,
    pub to: Option<Address>,
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

    #[test]
    fn impl_block() {
        let block = BlockData {
            number: 10_000_000,
            time: 1588598533,
        };
        assert_eq!(
            block.db_time(),
            NaiveDateTime::from_str("2020-05-04T13:22:13").unwrap()
        )
    }

    #[test]
    fn u256_compatiblity() {
        // One
        assert_eq!(
            U256::from(1),
            U256::from(ethers::types::U256::from_dec_str("1").unwrap())
        );
        // Some arbitrary number
        let num_string = "111122223333444455556666777788889999";
        assert_eq!(
            U256(Uint256::from_str_radix(num_string, 10).unwrap()),
            U256::from(ethers::types::U256::from_dec_str(num_string).unwrap())
        );
        // Max
        assert_eq!(U256(Uint256::MAX), U256::from(ethers::types::U256::MAX));
    }
}

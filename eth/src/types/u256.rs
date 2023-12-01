use bigdecimal::{BigDecimal, Num};
use diesel::{
    self,
    data_types::PgNumeric,
    deserialize::{self, FromSql},
    pg::{Pg, PgValue},
    sql_types::{Numeric, SqlType},
    Queryable,
};
use ethrpc::types::U256 as Uint256;
use serde::{de, Deserialize, Deserializer, Serialize};
use std::{num::ParseIntError, str::FromStr};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, SqlType, Hash)]
#[diesel(postgres_type(name = "U256"))]
pub struct U256(pub Uint256);

impl FromSql<Numeric, Pg> for U256 {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let big_decimal: BigDecimal = PgNumeric::from_sql(bytes)?.try_into()?;
        Ok(U256::from(big_decimal))
    }
}

impl Serialize for U256 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for U256 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct U256Visitor;

        impl<'de> de::Visitor<'de> for U256Visitor {
            type Value = U256;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string representing U256")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                value
                    .parse()
                    .map(U256)
                    .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(value), &self))
            }
        }

        deserializer.deserialize_str(U256Visitor)
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

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn u256_compatibility() {
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

    #[test]
    fn u256_deserialization() {
        let number = U256::from(1);
        let string = serde_json::to_string(&number).expect("Failed to serialize to JSON");
        println!("Number {:?}, String {}", number, string);
        let deserialized_number: U256 =
            serde_json::from_str(&string).expect("Failed to deserialize from JSON");
        assert_eq!(number, deserialized_number);
    }
}

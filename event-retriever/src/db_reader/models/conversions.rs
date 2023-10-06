use bigdecimal::{BigDecimal, Num};
use ethers::types::U256;

pub(crate) fn u256_from_big_decimal(value: &BigDecimal) -> U256 {
    U256::from_dec_str(&value.to_string()).expect("Invalid value")
}

pub(crate) fn big_decimal_from_u256(value: &U256) -> BigDecimal {
    BigDecimal::from_str_radix(&value.to_string(), 10).unwrap()
}

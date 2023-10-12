use bigdecimal::{BigDecimal, Num};
use ethers::types::{Address, U256};

pub fn u256_from_big_decimal(value: &BigDecimal) -> U256 {
    U256::from_dec_str(&value.to_string()).expect("Invalid value")
}

pub fn big_decimal_from_u256(value: &U256) -> BigDecimal {
    BigDecimal::from_str_radix(&value.to_string(), 10).unwrap()
}

pub fn address_to_vec(address: Address) -> Vec<u8> {
    address.as_bytes().to_vec()
}

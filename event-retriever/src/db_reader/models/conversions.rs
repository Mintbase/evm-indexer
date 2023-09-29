use bigdecimal::BigDecimal;
use ethers::types::U256;

pub(crate) fn u256_from_big_decimal(value: &BigDecimal) -> U256 {
    U256::from_dec_str(&value.to_string()).expect("Invalid value")
}

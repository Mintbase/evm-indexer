use anyhow::{Context, Result};

pub struct Config {
    pub store_url: String,
    pub store_schema: String,
    pub etherscan_key: String,
    pub alchemy_key: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            store_url: std::env::var("STORE_URL").context("missing STORE_URL")?,
            store_schema: std::env::var("DB_SCHEMA").context("missing DB_SCHEMA")?,
            etherscan_key: std::env::var("ETHERSCAN_KEY").context("missing ETHERSCAN_KEY")?,
            alchemy_key: std::env::var("ALCHEMY_KEY").ok(),
        })
    }
}

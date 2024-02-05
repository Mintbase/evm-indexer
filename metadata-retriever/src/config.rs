use anyhow::{Context, Result};

pub struct Config {
    pub node_url: String,
    pub store_url: String,
    pub etherscan_key: String,
    pub alchemy_key: Option<String>,
    pub db_schema: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            node_url: std::env::var("NODE_URL").unwrap_or("https://rpc.ankr.com/eth".to_string()),
            store_url: std::env::var("STORE_URL")
                .unwrap_or("postgresql://postgres:postgres@localhost:5432/store".to_string()),
            etherscan_key: std::env::var("ETHERSCAN_KEY").context("missing ETHERSCAN_KEY")?,
            alchemy_key: std::env::var("ALCHEMY_KEY").ok(),
            db_schema: std::env::var("DB_SCHEMA").context("missing DB_SCHEMA")?,
        })
    }
}

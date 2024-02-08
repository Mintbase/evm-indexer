use anyhow::{Context, Result};
use async_trait;
use eth::types::Address;
use reqwest::Client;
use serde_json::Value;
use tokio::time::{sleep, Duration as TokioDuration};

#[async_trait::async_trait]
pub trait AbiFetching: Send + Sync {
    async fn get_contract_abi(&self, address: Address) -> Result<Option<Value>>;
}

use serde::{Deserialize, Serialize};

// Common response structure for all Etherscan API responses
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub status: String,
    pub message: Option<String>,
    pub result: Option<T>,
}

pub struct EtherscanApi {
    api_key: String,
}

impl EtherscanApi {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_owned(),
        }
    }

    async fn call_etherscan_api(&self, request: &str) -> Result<Option<Value>> {
        let api_url = format!("https://api.etherscan.io/api?{request}");
        tracing::debug!("etherscan request to {api_url}");
        let ApiResponse {
            status,
            message,
            result,
        } = Client::new()
            .get(&format!("{api_url}&apikey={}", self.api_key))
            .send()
            .await?
            .json::<ApiResponse<Value>>()
            .await?;
        tracing::debug!("Status {}, message: {:?}", status, message);

        if status == "1" && result.is_some() {
            // The request was successful, return the result
            Ok(
                serde_json::from_str(result.unwrap().as_str().expect("should be string array"))
                    .context("expected string-ified JSON")?,
            )
        } else {
            match message {
                Some(message) => {
                    if message == "Contract source code not verified" {
                        return Ok(None);
                    }
                    Err(anyhow::anyhow!("API request failed: {}", message))
                }
                None => Err(anyhow::anyhow!("API request failed with unknown error")),
            }
        }
    }
}
#[async_trait::async_trait]
impl AbiFetching for EtherscanApi {
    async fn get_contract_abi(&self, address: Address) -> Result<Option<Value>> {
        const MAX_RETRIES: usize = 5;
        const INITIAL_BACKOFF: u64 = 1000; // Initial backoff in milliseconds

        let mut retries = 0;
        let mut backoff = INITIAL_BACKOFF;

        loop {
            let request = format!("module=contract&action=getabi&address={}", address);
            match self.call_etherscan_api(&request).await {
                Ok(result) => return Ok(result),
                Err(_error) if retries < MAX_RETRIES => {
                    sleep(TokioDuration::from_millis(backoff)).await;
                    retries += 1;
                    backoff *= 2; // Exponential backoff
                }
                Err(error) => return Err(error), // Max retries reached, return the error
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use std::str::FromStr;

    use super::*;
    use dotenv::dotenv;

    fn test_api() -> EtherscanApi {
        dotenv().ok();
        EtherscanApi::new(&std::env::var("ETHERSCAN_KEY").unwrap())
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    #[ignore = "requires ETHERSCAN KEY"]
    async fn get_contract_abi() {
        let etherscan = test_api();

        let unverified = etherscan.get_contract_abi(Address::zero()).await.unwrap();
        assert!(unverified.is_none());

        let verified = etherscan
            .get_contract_abi(
                Address::from_str("0x966731DFD9B9925DD105FF465687F5AA8F54EE9F").unwrap(),
            )
            .await
            .unwrap();
        assert!(verified.is_some());
    }
}

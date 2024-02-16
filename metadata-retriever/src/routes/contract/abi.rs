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
        let response = Client::new()
            .get(&format!("{api_url}&apikey={}", self.api_key))
            .send()
            .await?
            .json::<ApiResponse<Value>>()
            .await?;
        Self::handle_response(response)
    }

    fn handle_response(
        ApiResponse {
            status,
            message,
            result,
        }: ApiResponse<Value>,
    ) -> Result<Option<Value>> {
        tracing::debug!(
            "Status {}, message: {:?}, result: {:?}",
            status,
            message,
            result
        );
        if status == "1" && result.is_some() {
            // The request was successful, return the result
            Ok(
                serde_json::from_str(result.unwrap().as_str().expect("should be string array"))
                    .context("expected string-ified JSON")?,
            )
        } else {
            match result {
                Some(message) => {
                    if message == "Contract source code not verified" {
                        return Ok(Some(serde_json::Value::from("[]")));
                    }
                    Err(anyhow::anyhow!("request failed with: {}", message))
                }
                None => Err(anyhow::anyhow!("request failed with unknown error")),
            }
        }
    }
}
#[async_trait::async_trait]
impl AbiFetching for EtherscanApi {
    async fn get_contract_abi(&self, address: Address) -> Result<Option<Value>> {
        const MAX_RETRIES: usize = 1;
        const INITIAL_BACKOFF: u64 = 1000; // Initial backoff in milliseconds

        let mut retries = 0;
        let mut backoff = INITIAL_BACKOFF;

        loop {
            let request = format!("module=contract&action=getabi&address={}", address);
            match self.call_etherscan_api(&request).await {
                Ok(result) => return Ok(result),
                Err(error) if retries < MAX_RETRIES => {
                    tracing::info!(
                        "attempt {} failed with {:?} retrying in {backoff}",
                        retries + 1,
                        error
                    );
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
    use serde_json::json;

    fn test_api() -> EtherscanApi {
        dotenv().ok();
        EtherscanApi::new(&std::env::var("ETHERSCAN_KEY").unwrap())
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    #[ignore = "requires ETHERSCAN_KEY"]
    async fn get_contract_abi_found() {
        let etherscan = test_api();
        let verified = etherscan
            .get_contract_abi(
                Address::from_str("0x966731DFD9B9925DD105FF465687F5AA8F54EE9F").unwrap(),
            )
            .await
            .unwrap();
        assert!(verified.is_some());
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    #[ignore = "requires ETHERSCAN_KEY"]
    async fn get_contract_abi_not_found() {
        let etherscan = test_api();

        let unverified = etherscan.get_contract_abi(Address::zero()).await.unwrap();
        assert_eq!(unverified, Some(Value::String("[]".to_string())));
    }

    #[test]
    fn handle_response() {
        let bad_api_key_response: ApiResponse<Value> = serde_json::from_value(json!({
            "status": "0",
            "message": "NOTOK-Missing/Invalid API Key, rate limit of 1/5sec applied",
            "result": "Contract source code not verified"
        }))
        .unwrap();
        let result = EtherscanApi::handle_response(bad_api_key_response);
        assert_eq!(result.unwrap(), Some(serde_json::Value::from("[]")));
    }
}

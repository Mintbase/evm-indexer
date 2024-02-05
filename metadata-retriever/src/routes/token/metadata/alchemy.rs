// Based off the alchemy NFT docs: https://docs.alchemy.com/reference/nft-api-quickstart
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use eth::types::NftId;
use reqwest;
use serde_json::Value;

use super::MetadataFetching;

pub struct AlchemyApi {
    api_key: String,
    base_url: String,
}

impl AlchemyApi {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            // TODO - support other networks by config.
            base_url: "https://eth-mainnet.g.alchemy.com/nft/v3".to_string(),
        }
    }

    async fn get(&self, route: String) -> Result<serde_json::Value> {
        tracing::debug!("GET request on route: {}", route);
        let url = format!("{}/{}/{}", self.base_url, self.api_key, route);
        let response = reqwest::get(&url).await.context("get_metedata reqwest")?;

        if response.status().is_success() {
            let json_response = response
                .json::<serde_json::Value>()
                .await
                .context("serde_json::Value")?;
            Ok(json_response)
        } else {
            Err(anyhow!(format!(
                "Error: HTTP request failed with status code {}",
                response.status()
            )))
        }
    }
}

#[async_trait]
impl MetadataFetching for AlchemyApi {
    async fn get_nft_metadata(&self, token: NftId, _: Option<String>) -> Result<Value> {
        let NftId { address, token_id } = token;
        let route = format!(
            "getNFTMetadata/?contractAddress={}&tokenId={}",
            address, token_id
        );
        tracing::debug!("alchemy GET request {route}");
        let value = self.get(route).await?;
        tracing::debug!("alchemy GET response {value}");
        Ok(value)
        // serde_json::from_value::<NftContent>(value).context("serialize")
    }
}

#[cfg(test)]
mod tests {
    use eth::types::{Address, U256};
    use std::str::FromStr;

    use super::*;

    fn test_client() -> AlchemyApi {
        AlchemyApi::new(std::env::var("ALCHEMY_KEY").unwrap())
    }

    fn test_token_list() -> Vec<NftId> {
        [
            NftId {
                address: Address::from_str("0xbc4ca0eda7647a8ab7c2061c2e118a18a936f13d").unwrap(),
                token_id: U256::from(2),
            },
            // POAP
            NftId {
                address: Address::from_str("0x22C1f6050E56d2876009903609a2cC3fEf83B415").unwrap(),
                token_id: U256::from(8521),
            },
            // ENS
            NftId {
                address: Address::from_str("0x57F1887A8BF19B14FC0DF6FD9B2ACC9AF147EA85").unwrap(),
                token_id: U256::from(1),
            },
            // Simulcra
            NftId {
                address: Address::from_str("0x8644053aadb0df38e7734f5010fef643316bbb92").unwrap(),
                token_id: U256::from(21),
            },
            // Enjin
            NftId {
                address: Address::from_str("0xFAAFDC07907FF5120A76B34B731B278C38D6043C").unwrap(),
                token_id: U256::from_dec_str(
                    "10855508365998405147019449313071050427871334385647330815536805870982878199808",
                )
                .unwrap(),
            },
            // Alchemy Demo
            NftId {
                address: Address::from_str("0xe785E82358879F061BC3dcAC6f0444462D4b5330").unwrap(),
                token_id: U256::from(44),
            },
        ]
        .into_iter()
        .collect()
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    #[ignore = "requires ALCHEMY_KEY"]
    async fn get_metadata() {
        let api = test_client();
        for token in test_token_list() {
            let content_result = api.get_nft_metadata(token, None).await;
            assert!(content_result.is_ok())
        }
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    #[ignore = "requires ALCHEMY_KEY"]
    async fn try_one() {
        let api = test_client();
        let content_result = api
            .get_nft_metadata(
                NftId {
                    address: Address::from_str("0x659A4BDAAACC62D2BD9CB18225D9C89B5B697A5A")
                        .unwrap(),
                    token_id: U256::from_dec_str("1200").unwrap(),
                },
                None,
            )
            .await;
        assert!(content_result.is_ok())
    }
}

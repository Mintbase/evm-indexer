use anyhow::Result;
use async_trait;
use data_store::models::NftMetadata;
use eth::types::NftId;
use reqwest::Response;
use serde_json::Value;

pub mod homebrew;
mod util;
mod ipfs;

#[async_trait::async_trait]
pub trait MetadataFetching: Send + Sync {
    async fn get_nft_metadata(&self, token: NftId, uri: Option<String>) -> Result<FetchedMetadata>;
}

#[derive(Debug, PartialEq)]
pub struct FetchedMetadata {
    raw: String,
    json: Option<Value>,
}

impl From<FetchedMetadata> for NftMetadata {
    fn from(val: FetchedMetadata) -> Self {
        NftMetadata::new(&val.raw, val.json)
    }
}

impl FetchedMetadata {
    pub async fn from_response(response: Response) -> Result<Self> {
        let body = response.text().await?;
        let json = match serde_json::from_str::<Value>(&body) {
            Ok(json) => Some(json),
            Err(err) => {
                // Log the error issue
                tracing::warn!("Invalid JSON content: {} - using None", err);
                None
            }
        };
        Ok(Self { raw: body, json })
    }

    pub fn timeout() -> Self {
        Self {
            raw: "timeout".into(),
            json: None,
        }
    }
}

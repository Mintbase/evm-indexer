use anyhow::Result;
use async_trait;
use eth::types::NftId;
use serde_json::Value;
pub mod homebrew;
mod util;

#[async_trait::async_trait]
pub trait MetadataFetching: Send + Sync {
    async fn get_nft_metadata(&self, token: NftId, uri: Option<String>) -> Result<Value>;
}

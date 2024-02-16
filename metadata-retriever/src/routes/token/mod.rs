use eth::types::NftId;
use futures::stream::{self, StreamExt};
use serde_json::Value;
pub mod metadata;
use crate::app::AppData;

use super::RequestHandler;
use async_trait;

#[async_trait::async_trait]
impl RequestHandler<(NftId, Option<String>)> for AppData {
    async fn process_request(&self, tokens: &[(NftId, Option<String>)]) -> String {
        let possible_content: Vec<Option<Value>> = stream::iter(tokens)
            .then(|(token, uri)| {
                let app_ref = self.clone();
                async move {
                    match app_ref
                        .metadata_fetcher
                        .get_nft_metadata(*token, uri.clone())
                        .await
                    {
                        Ok(metadata) => Some(metadata),
                        Err(err) => {
                            tracing::warn!(
                                "metadata for {token:?} not found ({err:?}). Using None"
                            );
                            None
                        }
                    }
                }
            })
            .collect()
            .await;

        let updates: Vec<(NftId, Value)> = tokens
            .iter()
            .zip(possible_content)
            .filter_map(|(token, content)| content.map(|value| ((token.0), value)))
            .collect();
        self.store
            .lock()
            .expect("failed to lock mutex")
            .insert_metadata_batch(&updates);
        // TODO - return status code or something.
        return "success".to_string();
    }
}

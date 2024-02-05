use eth::types::{Address, Message, NftId};
use reqwest::Client;

#[derive(Clone)]
pub struct PubSubApi {
    client: Client,
    url: String,
}

impl PubSubApi {
    pub fn from_env() -> Self {
        Self {
            client: Client::new(),
            url: std::env::var("PUBSUB_URL")
                .unwrap_or("http://localhost:8080/pubsub_callback".to_string()),
        }
    }

    async fn post_request(&self, payload: serde_json::Value) -> Result<(), reqwest::Error> {
        self.client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;
        Ok(())
    }

    pub async fn token_request(&self, tokens: &[(NftId, Sting)]) -> Result<(), reqwest::Error> {
        let messages: Vec<_> = token_ids
            .iter()
            .map(|&NftId { address, token_id }| Message::Token { address, token_id, token_uri })
            .collect();
        self.post_request(serde_json::to_value(&messages).expect("Failed to serialize to JSON"))
            .await
    }

    pub async fn contract_request(&self, addresses: &[Address]) -> Result<(), reqwest::Error> {
        let messages: Vec<_> = addresses
            .iter()
            .map(|&address| Message::Contract { address })
            .collect();
        self.post_request(serde_json::to_value(&messages).expect("Failed to serialize to JSON"))
            .await
    }
}

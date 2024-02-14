use anyhow::Result;
use eth::types::Message;
use google_cloud_googleapis::pubsub::v1::PubsubMessage;
use google_cloud_pubsub::{
    client::{Client, ClientConfig},
    publisher::Publisher,
};

pub struct PubSubClient {
    publisher: Publisher,
}

impl PubSubClient {
    pub fn new(client: Client, topic_id: &str) -> Self {
        // Create a publisher for a specific topic
        Self {
            publisher: client.topic(topic_id).new_publisher(None),
        }
    }
    pub async fn local_emulator() -> PubSubClient {
        std::env::set_var("PUBSUB_EMULATOR_HOST", "localhost:8681");
        let config = ClientConfig::default();
        let client = Client::new(config).await.unwrap();
        PubSubClient::new(client, "test-topic")
    }
    pub async fn from_env() -> Result<Self> {
        // Client constructor requires one of
        //  - GOOGLE_APPLICATION_CREDENTIALS or
        //  - GOOGLE_APPLICATION_CREDENTIALS_JSON
        let config = ClientConfig::default().with_auth().await?;
        let client = Client::new(config).await?;
        let topic_id = std::env::var("PUBSUB_TOPIC_ID").expect("PUBSUB_TOPIC_ID must be set");
        Ok(Self::new(client, &topic_id))
    }

    pub async fn post_message(&self, message: Message) -> Result<()> {
        let awaiter = self.publisher.publish(Self::message_from(&message)).await;
        match awaiter.get().await {
            Ok(_success) => (),
            Err(failure) => tracing::error!("failed publish for {:?} with {}", message, failure),
        }
        Ok(())
    }

    pub async fn post_batch(&self, messages: &[Message]) -> Result<()> {
        let message_vec: Vec<_> = messages.iter().map(Self::message_from).collect();
        tracing::info!("posting {} messages to metadata fetcher", message_vec.len());
        let awaiter_vec = self.publisher.publish_bulk(message_vec).await;

        let results = futures::future::join_all(awaiter_vec.into_iter().map(|a| a.get())).await;
        // Haven't decided yet if we are going to batch log the errors or just log as we go.
        let errors: Vec<_> = messages
            .iter()
            .zip(results.into_iter())
            .filter_map(|(message, result)| {
                match result {
                    Ok(_) => {
                        None
                        // Handle success, if necessary (e.g., logging, metrics)
                    }
                    Err(err) => {
                        tracing::error!("failed publish for {:?} with {}", message, err);
                        Some((message, err))
                    }
                }
            })
            .collect();
        tracing::info!("posted messages with {} errors", errors.len());
        Ok(())
    }

    pub(crate) fn message_from<T: serde::Serialize>(val: &T) -> PubsubMessage {
        let input = serde_json::to_string(val).expect("val is JSON serializable");
        PubsubMessage {
            data: input.into(),
            ..Default::default() // attributes: Default::default(),
                                 // message_id: "".to_string(),
                                 // publish_time: None,
                                 // ordering_key: "".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eth::types::{Address, Message};
    use google_cloud_pubsub::client::{Client, ClientConfig};
    use std::str::FromStr;
    async fn test_client() -> PubSubClient {
        std::env::set_var("PUBSUB_EMULATOR_HOST", "localhost:8681");
        let config = ClientConfig::default();
        let client = Client::new(config).await.unwrap();

        PubSubClient::new(client, "test-topic")
    }

    #[tokio::test]
    async fn mock_publish() {
        let ps_client = test_client().await;
        let contract_payload = Message::Contract {
            address: Address::from_str("0x966731DFD9B9925DD105FF465687F5AA8F54EE9F").unwrap(),
        };

        let result = ps_client.post_message(contract_payload).await;
        assert!(result.is_ok())
    }

    #[tokio::test]
    #[ignore = "requires GOOGLE_APPLICATION_CREDENTIALS & PUBSUB_TOPIC_ID"]
    async fn real_publish() {
        let ps_client = PubSubClient::from_env().await.unwrap();

        let contract_payload = Message::Contract {
            address: Address::from_str("0x966731DFD9B9925DD105FF465687F5AA8F54EE9F").unwrap(),
        };

        let result = ps_client.post_message(contract_payload).await;
        assert!(result.is_ok())
    }
}

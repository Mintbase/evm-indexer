use anyhow::Result;
use eth::types::Message;
use google_cloud_googleapis::pubsub::v1::PubsubMessage;
use google_cloud_pubsub::publisher::Awaiter;
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
    pub async fn from_env() -> Result<Self> {
        // Client constructor requires one of
        //  - GOOGLE_APPLICATION_CREDENTIALS or
        //  - GOOGLE_APPLICATION_CREDENTIALS_JSON
        let config = ClientConfig::default().with_auth().await?;
        let client = Client::new(config).await?;
        let topic_id = std::env::var("PUBSUB_TOPIC_ID").expect("PUBSUB_TOPIC must be set");
        Ok(Self::new(client, &topic_id))
    }

    pub async fn post_message(&self, message: Message) -> Result<Awaiter> {
        Ok(self.publisher.publish(Self::message_from(&message)).await)
    }

    pub async fn post_batch(&self, messages: Vec<Message>) -> Result<Vec<Awaiter>> {
        let message_vec = messages.iter().map(Self::message_from).collect();
        Ok(self.publisher.publish_bulk(message_vec).await)
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

        let awaiter = ps_client.post_message(contract_payload).await.unwrap();
        let result = awaiter.get().await;
        println!("Result {result:?}");
        assert!(result.is_ok())
    }

    #[tokio::test]
    #[ignore = "requires GOOGLE_APPLICATION_CREDENTIALS & PUBSUB_TOPIC_ID"]
    async fn real_publish() {
        let ps_client = PubSubClient::from_env().await.unwrap();

        let contract_payload = Message::Contract {
            address: Address::from_str("0x966731DFD9B9925DD105FF465687F5AA8F54EE9F").unwrap(),
        };

        let awaiter = ps_client.post_message(contract_payload).await.unwrap();
        let result = awaiter.get().await;
        assert!(result.is_ok())
    }
}

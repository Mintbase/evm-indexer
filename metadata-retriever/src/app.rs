use crate::routes::token::metadata::homebrew::Homebrew;
use crate::{
    config::Config,
    routes::{
        contract::abi::{AbiFetching, EtherscanApi},
        token::metadata::{alchemy::AlchemyApi, MetadataFetching},
    },
};
use anyhow::Result;
use data_store::store::DataStore;
use google_cloud_pubsub::{
    client::{Client, ClientConfig},
    subscription::{Subscription, SubscriptionConfig},
};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppData {
    pub store: Arc<Mutex<DataStore>>,
    pub abi_fetcher: Arc<dyn AbiFetching>,
    pub metadata_fetcher: Arc<dyn MetadataFetching>,
    _subscription: Subscription,
}

impl AppData {
    pub async fn new(subscriber: Subscription, config: Config) -> Self {
        let metadata_fetcher: Arc<dyn MetadataFetching> = match config.alchemy_key {
            Some(key) => {
                tracing::debug!("Using AlchemyAPI for metadata retrieval");
                Arc::new(AlchemyApi::new(key))
            }
            None => {
                tracing::debug!("Using Homebrew for metadata retrieval");
                Arc::new(Homebrew {})
            }
        };
        Self {
            store: Arc::new(Mutex::new(
                DataStore::new(&config.store_url, &config.db_schema).expect("Data Store required"),
            )),
            abi_fetcher: Arc::new(EtherscanApi::new(&config.etherscan_key)),
            metadata_fetcher,
            _subscription: subscriber,
        }
    }
}

// TODO - this is a test subscription!
pub async fn build_subscription(config: ClientConfig) -> Result<Subscription> {
    // Create pubsub client.
    let client = Client::new(config).await?;

    // Get the topic to subscribe to.
    let topic = client.topic("test-topic");

    // Create subscription
    // If subscription name does not contain a "/", then the project is taken from client above. Otherwise, the
    // name will be treated as a fully qualified resource name
    let config = SubscriptionConfig {
        // Enable message ordering if needed (https://cloud.google.com/pubsub/docs/ordering)
        // enable_message_ordering: true,
        ..Default::default()
    };

    // Create subscription
    let subscription = client.subscription("test-subscription");
    if !subscription.exists(None).await? {
        subscription
            .create(topic.fully_qualified_name(), config, None)
            .await?;
    }
    Ok(subscription)
}

use crate::{
    config::Config,
    routes::{
        contract::abi::{AbiFetching, EtherscanApi},
        token::metadata::{homebrew::Homebrew, MetadataFetching},
    },
};
use data_store::store::DataStore;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppData {
    pub store: Arc<Mutex<DataStore>>,
    pub abi_fetcher: Arc<dyn AbiFetching>,
    pub metadata_fetcher: Arc<dyn MetadataFetching>,
}

impl AppData {
    pub async fn new(config: Config) -> Self {
        // TODO - support for Alchemy Fetching: https://github.com/Mintbase/evm-indexer/issues/138
        let metadata_fetcher: Arc<dyn MetadataFetching> = Arc::new(Homebrew {});
        Self {
            store: Arc::new(Mutex::new(
                DataStore::new(&config.store_url, &config.store_schema)
                    .expect("Data Store required"),
            )),
            abi_fetcher: Arc::new(EtherscanApi::new(&config.etherscan_key)),
            metadata_fetcher,
        }
    }
}

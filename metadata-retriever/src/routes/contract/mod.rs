use data_store::models::ContractAbi;
use eth::types::Address;
use futures::stream::{self, StreamExt};

use crate::app::AppData;

use super::RequestHandler;
use async_trait;

pub mod abi;

#[async_trait::async_trait]
impl RequestHandler<Address> for AppData {
    async fn process_request(&self, addresses: &[Address]) -> String {
        let abis: Vec<_> = stream::iter(addresses)
            .then(|address| {
                let app_ref = self.clone();
                async move {
                    match app_ref.abi_fetcher.get_contract_abi(*address).await {
                        Ok(possible_abi) => match possible_abi {
                            Some(abi) => {
                                tracing::debug!("Found contract abi for {address}");
                                Some((*address, ContractAbi::from(abi)))
                            }
                            None => None,
                        },
                        Err(err) => {
                            tracing::warn!("Failed to get abi for {address}: {err:?}");
                            None
                        }
                    }
                }
            })
            .collect::<Vec<_>>()
            .await
            .iter()
            .filter_map(|option| option.as_ref().cloned())
            .collect();

        self.store
            .lock()
            .expect("failed to lock mutex")
            .insert_contract_abis(&abis);

        return format!("Processed request for {} contracts", addresses.len());
    }
}

use crate::types::{Address, BlockData, ContractDetails, NftId, TxDetails};
use anyhow::{anyhow, Error, Result};
use async_trait::async_trait;
use ethers::{
    middleware::Middleware,
    prelude::abigen,
    providers::{Http, Provider},
    utils::hex,
};
use futures::future::{join, join_all};
use std::{collections::HashMap, fmt::Debug, sync::Arc, time::Duration};

use super::EthNodeReading;

abigen!(ERC721Metadata, "./src/abis/ERC721Metadata.json");

fn erc721_contract_at_address(
    address: Address,
    provider: Arc<Provider<Http>>,
) -> ERC721Metadata<Provider<Http>> {
    ERC721Metadata::new(ethers::types::Address::from(address.0 .0), provider)
}

#[async_trait::async_trait]
trait RetryGet<T: Send> {
    async fn try_get(&self) -> Result<T>;
    fn is_retryable_error(error: &anyhow::Error) -> bool {
        let error_string = error.to_string();
        if error_string.contains("Contract call reverted with data:") {
            let hex_string = error_string
                .split_whitespace()
                .last()
                .expect("known error pattern");
            let bytes = hex::decode(hex_string).expect("Failed to decode hex");
            let decoded_message = String::from_utf8_lossy(&bytes);
            tracing::warn!("Contract call reverted with message: {}", decoded_message);
            return false;
        }
        true
    }
    // so this shouldn't only retry from one provider, but from multiple, esp. at the top of the chain
    async fn retry_get(&self, max_retries: u32, wait_secs: u64) -> Result<T> {
        let mut retries = 0;
        loop {
            match self.try_get().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    retries += 1;
                    if !Self::is_retryable_error(&err) || retries >= max_retries {
                        return Err(err);
                    } else {
                        tracing::debug!(
                            "failed rpc request attempt {} with error {} - trying again in {} seconds",
                            retries,
                            err,
                            wait_secs
                        );
                        tokio::time::sleep(Duration::from_secs(wait_secs)).await;
                    }
                }
            }
        }
    }
}

struct GetBlock {
    provider: Arc<Provider<Http>>,
    block: u64,
}

#[async_trait::async_trait]
impl RetryGet<Option<BlockData>> for GetBlock {
    async fn try_get(&self) -> Result<Option<BlockData>> {
        let res: Option<ethers::types::Block<ethers::types::Transaction>> =
            self.provider.get_block_with_txs(self.block).await?;

        Ok(match res {
            Some(ethers_block) => {
                let transactions = ethers_block
                    .transactions
                    .into_iter()
                    .map(|tx| {
                        (
                            tx.transaction_index.expect("should exist").as_u64(),
                            TxDetails::from(tx),
                        )
                    })
                    .collect();
                Some(BlockData {
                    // Could also use client_response for this, but its optional.
                    number: self.block,
                    time: ethers_block.timestamp.as_u64(),
                    transactions,
                })
            }
            None => None,
        })
    }
}

struct GetBlockReceipts {
    provider: Arc<Provider<Http>>,
    block: u64,
}

#[async_trait::async_trait]
impl RetryGet<HashMap<u64, TxDetails>> for GetBlockReceipts {
    async fn try_get(&self) -> Result<HashMap<u64, TxDetails>> {
        let block = self.block;
        let receipts = self.provider.get_block_receipts(block).await?;
        Ok(receipts
            .into_iter()
            .map(|r| (r.transaction_index.0[0], r.into()))
            .collect())
    }
}

struct GetErc721Uri {
    provider: Arc<Provider<Http>>,
    token: NftId,
}

#[async_trait::async_trait]
impl RetryGet<String> for GetErc721Uri {
    async fn try_get(&self) -> Result<String> {
        let contract = erc721_contract_at_address(self.token.address, self.provider.clone());
        contract
            .token_uri(ethers::types::U256::from_big_endian(
                &self.token.token_id.0.to_be_bytes(),
            ))
            .call()
            .await
            // Remove Null Bytes: Postgres can't handle them.
            .map(|uri| uri.replace('\0', ""))
            .map_err(|err| anyhow!(err.to_string()))
    }
}

struct GetName {
    provider: Arc<Provider<Http>>,
    address: Address,
}

#[async_trait::async_trait]
impl RetryGet<String> for GetName {
    async fn try_get(&self) -> Result<String> {
        let contract = erc721_contract_at_address(self.address, self.provider.clone());
        contract
            .name()
            .call()
            .await
            // Remove Null Bytes: Postgres can't handle them.
            .map(|uri| uri.replace('\0', ""))
            .map_err(|err| anyhow!(err.to_string()))
    }
}

struct GetSymbol {
    provider: Arc<Provider<Http>>,
    address: Address,
}

#[async_trait::async_trait]
impl RetryGet<String> for GetSymbol {
    async fn try_get(&self) -> Result<String> {
        let contract = erc721_contract_at_address(self.address, self.provider.clone());
        contract
            .symbol()
            .call()
            .await
            // Remove Null Bytes: Postgres can't handle them.
            .map(|uri| uri.replace('\0', ""))
            .map_err(|err| anyhow!(err.to_string()))
    }
}

pub struct Client {
    provider: Arc<Provider<Http>>,
}

#[async_trait]
impl EthNodeReading for Client {
    async fn get_contract_details(
        &self,
        addresses: &[Address],
    ) -> HashMap<Address, ContractDetails> {
        tracing::debug!("Preparing {} Contract Details Requests", addresses.len());
        let name_futures = addresses.iter().cloned().map(|a| self.get_name(a));
        let symbol_futures = addresses.iter().cloned().map(|a| self.get_symbol(a));

        let (names, symbols) = join(join_all(name_futures), join_all(symbol_futures)).await;
        tracing::debug!("Complete {} Contract Details Requests", addresses.len());

        addresses
            .iter()
            .zip(names.into_iter().zip(symbols))
            .map(|(&address, (name, symbol))| (address, ContractDetails { name, symbol }))
            .collect()
    }

    async fn get_uris(&self, token_ids: &[NftId]) -> HashMap<NftId, Option<String>> {
        tracing::info!("Preparing {} tokenUri Requests", token_ids.len());
        let futures = token_ids
            .iter()
            .cloned()
            .map(|token| self.get_erc721_uri(token));

        let uris = join_all(futures).await;

        token_ids
            .iter()
            .zip(uris)
            .map(|(&id, uri_result)| {
                (
                    id,
                    match uri_result {
                        Ok(val) => Some(val),
                        Err(err) => {
                            tracing::warn!("failed to decode token_uri for {:?}: {:?}", id, err);
                            None
                        }
                    },
                )
            })
            .collect()
    }

    async fn get_blocks_for_range(&self, start: u64, end: u64) -> Result<HashMap<u64, BlockData>> {
        let futures = (start..end).map(|block: u64| self.get_block(block));

        let (possible_blocks, errors) = Self::unpack_results(join_all(futures).await);
        if !errors.is_empty() {
            return Err(anyhow!(
                "failed to retrieve {} blocks {:?}",
                errors.len(),
                errors
            ));
        }

        Ok(possible_blocks
            .into_iter()
            .filter_map(|possible_block| {
                possible_block.map(|block_data| (block_data.number, block_data))
            })
            .collect())
    }
}

impl Client {
    pub fn new(url: &str) -> Result<Self> {
        Ok(Self {
            provider: Arc::new(Provider::<Http>::try_from(url)?),
        })
    }

    fn unpack_results<T: Debug>(results: Vec<Result<T, Error>>) -> (Vec<T>, Vec<Error>) {
        let (oks, errors): (Vec<_>, Vec<_>) = results.into_iter().partition(Result::is_ok);
        let oks: Vec<_> = oks.into_iter().map(Result::unwrap).collect();
        let errors: Vec<Error> = errors.into_iter().map(Result::unwrap_err).collect();
        (oks, errors)
    }

    pub async fn get_block(&self, block: u64) -> Result<Option<BlockData>> {
        GetBlock {
            provider: self.provider.clone(),
            block,
        }
        .retry_get(3, 1)
        .await
    }

    pub async fn get_erc721_uri(&self, token: NftId) -> Result<String> {
        GetErc721Uri {
            provider: self.provider.clone(),
            token,
        }
        .retry_get(3, 1)
        .await
    }

    async fn get_name(&self, address: Address) -> Option<String> {
        GetName {
            provider: self.provider.clone(),
            address,
        }
        .retry_get(3, 1)
        .await
        .ok()
    }

    async fn get_symbol(&self, address: Address) -> Option<String> {
        GetSymbol {
            provider: self.provider.clone(),
            address,
        }
        .retry_get(3, 1)
        .await
        .ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::U256;
    use diesel::internal::derives::multiconnection::chrono::NaiveDateTime;
    use maplit::hashmap;
    use std::str::FromStr;
    use tracing_test::traced_test;

    static FREE_ETH_RPC: &str = "https://rpc.ankr.com/eth"; // Also supports

    fn test_client() -> Client {
        Client::new(FREE_ETH_RPC).expect("Needed for test")
    }

    #[tokio::test]
    async fn get_block() {
        let eth_client = test_client();

        // Some
        let number = 10_000_000;
        // Second unwrap asserts block.is_some()!
        let block = eth_client.get_block(number).await.unwrap().unwrap();
        assert_eq!(block.time, 1588598533);
        assert_eq!(block.number, number);
        assert_eq!(block.transactions.len(), 103);
        // Check that: https://etherscan.io/block/10000000
        assert_eq!(
            block.db_time(),
            NaiveDateTime::from_str("2020-05-04T13:22:13").unwrap()
        );

        // None
        assert!(eth_client
            .get_block(i64::MAX as u64)
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn get_erc721_uri() {
        let eth_client = test_client();
        let ens_token = NftId {
            address: Address::from_str("0x57F1887A8BF19B14FC0DF6FD9B2ACC9AF147EA85").unwrap(),
            token_id: U256::from_dec_str(
                "64671196571681841248190411691641946869002480279128285790058847953168666315",
            )
            .unwrap(),
        };

        assert_eq!(
            eth_client
                .get_erc721_uri(ens_token)
                .await
                .unwrap_err()
                .to_string(),
            "Contract call reverted with data: 0x".to_string()
        );

        let bored_ape = NftId {
            address: Address::from_str("0x2EE6AF0DFF3A1CE3F7E3414C52C48FD50D73691E").unwrap(),
            token_id: U256::from(16),
        };

        assert!(eth_client.get_erc721_uri(bored_ape).await.is_ok());

        let mla_field_agent = NftId {
            address: Address::from_str("0x7A41E410BB784D9875FA14F2D7D2FA825466CDAE").unwrap(),
            token_id: U256::from(3490),
        };

        assert_eq!(
            eth_client
                .get_erc721_uri(mla_field_agent)
                .await
                .unwrap_err()
                .to_string(),
            "Contract call reverted with data: 0x08c379a00000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002f4552433732314d657461646174613a2055524920717565727920666f72206e6f6e6578697374656e7420746f6b656e0000000000000000000000000000000000".to_string()
        );
    }

    #[tokio::test]
    async fn get_contract_details() {
        let eth_client = test_client();
        let ens_contract = Address::from_str("0x57F1887A8BF19B14FC0DF6FD9B2ACC9AF147EA85").unwrap();
        let bored_ape_contract =
            Address::from_str("0x2EE6AF0DFF3A1CE3F7E3414C52C48FD50D73691E").unwrap();
        let mla_field_agent =
            Address::from_str("0x7A41E410BB784D9875FA14F2D7D2FA825466CDAE").unwrap();
        assert_eq!(
            eth_client
                .get_contract_details(&[ens_contract, bored_ape_contract, mla_field_agent])
                .await,
            hashmap! {
                ens_contract => ContractDetails{ name: None, symbol: None },
                bored_ape_contract => ContractDetails {
                    name: Some("Bored Ape Yacht Club".to_string()),
                    symbol: Some("BAYC".to_string()),
                },
                mla_field_agent => ContractDetails {
                    name: Some("Meta Labs Field Agents".to_string()),
                    symbol: Some("MLA1".to_string()),
                }
            }
        );
    }
    #[tokio::test]
    #[traced_test]
    async fn test_non_retryable_error() {
        let eth_client = test_client();
        let ens_contract = Address::from_str("0x57F1887A8BF19B14FC0DF6FD9B2ACC9AF147EA85").unwrap();
        eth_client.get_contract_details(&[ens_contract]).await;

        let warn_message = "Contract call reverted with message:";
        // Ensure that certain strings are or aren't logged
        assert!(logs_contain(warn_message));
        assert!(!logs_contain("failed rpc request attempt"));

        // Ensure that the string `logged` is logged exactly twice
        logs_assert(|lines: &[&str]| {
            match lines
                .iter()
                .filter(|line| line.contains(warn_message))
                .count()
            {
                2 => Ok(()),
                n => Err(format!("Expected two matching logs, but found {}", n)),
            }
        });
    }
}

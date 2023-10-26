use crate::types::{Address, Bytes32, NftId};
use anyhow::{anyhow, Result};
use ethers::{
    middleware::Middleware,
    prelude::abigen,
    providers::{Http, Provider},
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

abigen!(ERC721Metadata, "./src/abis/ERC721Metadata.json");

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct TxDetails {
    pub hash: Bytes32,
    pub from: Address,
    pub to: Option<Address>,
}

impl From<ethers::types::TransactionReceipt> for TxDetails {
    fn from(value: ethers::types::TransactionReceipt) -> Self {
        TxDetails {
            hash: Bytes32(value.transaction_hash),
            from: Address(value.from),
            to: value.to.map(Address::from),
        }
    }
}

impl From<ethers::types::Transaction> for TxDetails {
    fn from(value: ethers::types::Transaction) -> Self {
        TxDetails {
            hash: Bytes32(value.hash),
            from: Address(value.from),
            to: value.to.map(Address::from),
        }
    }
}

pub struct Client {
    provider: Arc<Provider<Http>>,
}

impl Client {
    pub fn new(url: &str) -> Result<Self> {
        Ok(Self {
            provider: Arc::new(Provider::<Http>::try_from(url)?),
        })
    }

    // TODO (Cost Optimization): on the number of `indices`.
    //  Example: QuickNode API credits for
    //  - eth_getBlockReceipts                    is 59 while
    //  - eth_getTransactionByBlockNumberAndIndex is 2
    //  So when indices.len() < 59/2 its cheaper to get them individually.

    // TODO - make a blocks table for timestamps.
    //  let block_time = eth_client
    //      .get_block(block)
    //      .await?
    //      .expect("block {block} not found")
    //      .timestamp;
    pub async fn get_block_receipts(
        &self,
        block: u64,
        indices: HashSet<u64>,
    ) -> Result<HashMap<u64, TxDetails>> {
        // First try eth_getBlockReceipts
        // This method is only supported by a few node providers: Erigon (e.g. QuickNode, Alchemy, Ankr).
        // But not Infura for example
        match self.provider.get_block_receipts(block).await {
            Ok(receipts) => Ok(receipts
                .into_iter()
                .filter_map(|r| {
                    let index = r.transaction_index.0[0];
                    if indices.contains(&index) {
                        Some((r.transaction_index.0[0], r.into()))
                    } else {
                        // Otherwise, we aren't interested.
                        None
                    }
                })
                .collect()),
            Err(_) => {
                // Uses: eth_getTransactionByBlockNumberAndIndex (Supported by all nodes)
                // Likely that the provider does not support the first method.
                let mut result = HashMap::new();
                let mut handles = vec![];

                for index in indices {
                    // TODO - Call eth_getBlockTransactionCountByNumber and don't request when index > that.
                    let eth_client_clone = self.provider.clone();
                    let handle = tokio::spawn(async move {
                        let tx = eth_client_clone
                            .get_transaction_by_block_and_index(block, index.into())
                            .await;

                        (index, tx)
                    });

                    handles.push(handle);
                }
                for handle in handles {
                    let (index, tx) = handle.await.expect("Task panicked");
                    match tx? {
                        Some(receipt) => {
                            result.insert(index, receipt.into());
                        }
                        None => {
                            tracing::warn!("transaction at block {block}, index {index} not found");
                        }
                    }
                }
                Ok(result)
            }
        }
    }

    pub async fn get_erc721_uri(&self, token: &NftId) -> Result<String> {
        let contract = ERC721Metadata::new(token.address, self.provider.clone());
        contract
            .token_uri(token.token_id.0)
            .call()
            .await
            // Remove Null Bytes: Postgres can't handle them.
            .map(|uri| uri.replace('\0', ""))
            .map_err(|err| anyhow!(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use maplit::{hashmap, hashset};

    static FREE_ETH_RPC: &str = "https://rpc.ankr.com/eth"; // Also supports

    fn test_client() -> Client {
        Client::new(FREE_ETH_RPC).expect("Needed for test")
    }

    #[tokio::test]
    async fn get_receipts_free() {
        let eth_client = test_client();
        assert_eq!(
            eth_client
                .get_block_receipts(1_000_000, hashset! {0, 1})
                .await
                .unwrap(),
            hashmap! {
                1 => TxDetails {
                    hash: Bytes32::from_str("0xe9e91f1ee4b56c0df2e9f06c2b8c27c6076195a88a7b8537ba8313d80e6f124e").unwrap(),
                    from: Address::from_str("0x32be343b94f860124dc4fee278fdcbd38c102d88").unwrap(),
                    to: Some(Address::from_str("0xdf190dc7190dfba737d7777a163445b7fff16133").unwrap())
                },
                0 => TxDetails {
                    hash: Bytes32::from_str("0xea1093d492a1dcb1bef708f771a99a96ff05dcab81ca76c31940300177fcf49f").unwrap(),
                    from: Address::from_str("0x39fa8c5f2793459d6622857e7d9fbb4bd91766d3").unwrap(),
                    to: Some(Address::from_str("0xc083e9947cf02b8ffc7d3090ae9aea72df98fd47").unwrap())
                }
            }
        );

        // Perhaps we should fail if the requested index doesn't exist.
        assert_eq!(
            // This transaction index doesn't exist!
            eth_client
                .get_block_receipts(1_000_000, hashset! {2})
                .await
                .unwrap(),
            HashMap::new()
        );
    }

    #[tokio::test]
    #[ignore]
    async fn get_many_receipts() {
        let eth_client = test_client();
        let indices = (0..500).collect::<HashSet<_>>();
        let result = eth_client.get_block_receipts(10_000_000, indices).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().keys().len(), 103);
    }
}

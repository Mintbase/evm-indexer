use anyhow::Result;
use ethers::{
    middleware::Middleware,
    providers::{Http, Provider},
    types::{Address, H256},
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct TxDetails {
    pub hash: H256,
    pub from: Address,
    pub to: Option<Address>,
}

impl From<ethers::types::TransactionReceipt> for TxDetails {
    fn from(value: ethers::types::TransactionReceipt) -> Self {
        TxDetails {
            hash: value.transaction_hash,
            from: value.from,
            to: value.to,
        }
    }
}

impl From<ethers::types::Transaction> for TxDetails {
    fn from(value: ethers::types::Transaction) -> Self {
        TxDetails {
            hash: value.hash,
            from: value.from,
            to: value.to,
        }
    }
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
    eth_client: &Provider<Http>,
    block: u64,
    indices: HashSet<u64>,
) -> Result<HashMap<u64, TxDetails>> {
    // First try eth_getBlockReceipts
    // This method is only supported by a few node providers: Erigon (e.g. QuickNode, Alchemy, Ankr).
    // But not Infura for example
    match eth_client.get_block_receipts(block).await {
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
                let eth_client_clone = eth_client.clone();
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use maplit::{hashmap, hashset};

    static FREE_ETH_RPC: &str = "https://rpc.ankr.com/eth"; // Also supports

    fn test_provider() -> Provider<Http> {
        Provider::<Http>::try_from(FREE_ETH_RPC).expect("Needed for test")
    }

    #[tokio::test]
    async fn get_receipts_free() {
        let eth_client = test_provider();
        assert_eq!(
            get_block_receipts(&eth_client, 1_000_000, hashset! {0, 1})
                .await
                .unwrap(),
            hashmap! {
                1 => TxDetails {
                    hash: H256::from_str("0xe9e91f1ee4b56c0df2e9f06c2b8c27c6076195a88a7b8537ba8313d80e6f124e").unwrap(),
                    from: Address::from_str("0x32be343b94f860124dc4fee278fdcbd38c102d88").unwrap(),
                    to: Some(Address::from_str("0xdf190dc7190dfba737d7777a163445b7fff16133").unwrap())
                },
                0 => TxDetails {
                    hash: H256::from_str("0xea1093d492a1dcb1bef708f771a99a96ff05dcab81ca76c31940300177fcf49f").unwrap(),
                    from: Address::from_str("0x39fa8c5f2793459d6622857e7d9fbb4bd91766d3").unwrap(),
                    to: Some(Address::from_str("0xc083e9947cf02b8ffc7d3090ae9aea72df98fd47").unwrap())
                }
            }
        );

        // Perhaps we should fail if the requested index doesn't exist.
        assert_eq!(
            // This transaction index doesn't exist!
            get_block_receipts(&eth_client, 1_000_000, hashset! {2})
                .await
                .unwrap(),
            HashMap::new()
        );
    }

    #[tokio::test]
    #[ignore]
    async fn get_many_receipts() {
        let eth_client = test_provider();
        let indices = (0..500).collect::<HashSet<_>>();
        let result = get_block_receipts(&eth_client, 10_000_000, indices).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().keys().len(), 103);
    }
}

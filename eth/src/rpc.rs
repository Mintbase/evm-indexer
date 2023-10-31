use crate::types::{Address, Bytes32, NftId};
use anyhow::{anyhow, Result};
use diesel::internal::derives::multiconnection::chrono::NaiveDateTime;
use ethers::{
    middleware::Middleware,
    prelude::abigen,
    providers::{Http, Provider},
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
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

#[derive(Debug, PartialEq)]
pub struct BlockData {
    /// Block Number
    pub number: u64,
    /// Unix timestamp as 64-bit integer
    pub time: u64,
}

impl BlockData {
    pub fn db_time(&self) -> NaiveDateTime {
        NaiveDateTime::from_timestamp_opt(self.time.try_into().expect("no crazy times"), 0)
            .expect("No crazy times plz")
    }
}

#[async_trait::async_trait]
trait RetryGet<T: Send> {
    async fn try_get(&self) -> Result<T>;

    // so this shouldn't only retry from one provider, but from multiple, esp. at the top of the chain
    async fn retry_get(&self, max_retries: u32, wait_secs: u64) -> Result<T> {
        let mut retries = 0;
        loop {
            match self.try_get().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    retries += 1;
                    if retries >= max_retries {
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
        let res: Option<ethers::types::Block<ethers::types::H256>> =
            self.provider.get_block(self.block).await?;

        Ok(match res {
            Some(ethers_block) => Some(BlockData {
                // Could also use client_response for this, but its optional.
                number: self.block,
                time: ethers_block.timestamp.as_u64(),
            }),
            None => None,
        })
    }
}

struct GetBlockReceipts {
    provider: Arc<Provider<Http>>,
    block: u64,
    indices: HashSet<u64>,
}

#[async_trait::async_trait]
impl RetryGet<HashMap<u64, TxDetails>> for GetBlockReceipts {
    // TODO (Cost Optimization): on the number of `indices`.
    //  Example: QuickNode API credits for
    //  - eth_getBlockReceipts                    is 59 while
    //  - eth_getTransactionByBlockNumberAndIndex is 2
    //  So when indices.len() < 59/2 its cheaper to get them individually.
    async fn try_get(&self) -> Result<HashMap<u64, TxDetails>> {
        let block = self.block;
        // First try eth_getBlockReceipts
        // This method is only supported by a few node providers: Erigon (e.g. QuickNode, Alchemy, Ankr).
        // But not Infura for example
        match self.provider.get_block_receipts(block).await {
            Ok(receipts) => Ok(receipts
                .into_iter()
                .filter_map(|r| {
                    let index = r.transaction_index.0[0];
                    if self.indices.contains(&index) {
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

                for index in self.indices.iter().copied() {
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
                            tracing::warn!(
                                "transaction at block {}, index {} not found",
                                self.block,
                                index
                            );
                        }
                    }
                }
                Ok(result)
            }
        }
    }
}

struct GetErc721Uri {
    provider: Arc<Provider<Http>>,
    token: NftId,
}

#[async_trait::async_trait]
impl RetryGet<String> for GetErc721Uri {
    async fn try_get(&self) -> Result<String> {
        let contract = ERC721Metadata::new(self.token.address, self.provider.clone());
        contract
            .token_uri(self.token.token_id.0)
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
        let contract = ERC721Metadata::new(self.address, self.provider.clone());
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
        let contract = ERC721Metadata::new(self.address, self.provider.clone());
        contract
            .symbol()
            .call()
            .await
            // Remove Null Bytes: Postgres can't handle them.
            .map(|uri| uri.replace('\0', ""))
            .map_err(|err| anyhow!(err.to_string()))
    }
}
#[derive(PartialEq, Debug)]
pub struct ContractDetails {
    pub name: Option<String>,
    pub symbol: Option<String>,
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

    pub async fn get_block(&self, block: u64) -> Result<Option<BlockData>> {
        GetBlock {
            provider: self.provider.clone(),
            block,
        }
        .retry_get(3, 1)
        .await
    }

    pub async fn get_block_receipts(
        &self,
        block: u64,
        indices: HashSet<u64>,
    ) -> Result<HashMap<u64, TxDetails>> {
        GetBlockReceipts {
            provider: self.provider.clone(),
            block,
            indices,
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
        .try_get()
        .await
        .ok()
    }

    async fn get_symbol(&self, address: Address) -> Option<String> {
        GetSymbol {
            provider: self.provider.clone(),
            address,
        }
        .try_get()
        .await
        .ok()
    }

    pub async fn get_contract_details(&self, address: Address) -> ContractDetails {
        ContractDetails {
            // TODO - fetch these simultaneously!
            name: self.get_name(address).await,
            symbol: self.get_symbol(address).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::types::U256;
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
    async fn get_block() {
        let eth_client = test_client();

        // Some
        let number = 10_000_000;
        let block = eth_client.get_block(number).await.unwrap();
        assert_eq!(
            block,
            Some(BlockData {
                number,
                time: 1588598533
            })
        );
        // Check that: https://etherscan.io/block/10000000
        assert_eq!(
            block.unwrap().db_time(),
            NaiveDateTime::from_str("2020-05-04T13:22:13").unwrap()
        );

        // None
        assert!(eth_client
            .get_block(i64::MAX as u64)
            .await
            .unwrap()
            .is_none());
    }

    #[test]
    fn impl_block() {
        let block = BlockData {
            number: 10_000_000,
            time: 1588598533,
        };
        assert_eq!(
            block.db_time(),
            NaiveDateTime::from_str("2020-05-04T13:22:13").unwrap()
        )
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

        // TODO - the above Err is a human readable message:
        // #[derive(Debug, Deserialize)]
        // struct Struct {
        //     ERC721Metadata: String,
        //     uri: String,
        // }
        // let hex_string = "08c379a00000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002f4552433732314d657461646174613a2055524920717565727920666f72206e6f6e6578697374656e7420746f6b656e0000000000000000000000000000000000";
        // let bytes = hex::decode(hex_string).expect("Failed to decode hex");
        // let result: MyStruct = serde_json::from_slice(&bytes).expect("Failed to deserialize JSON");
        // {
        //   "ERC721Metadata": "URI query for nonexistent token",
        //   "uri": "/"
        // }
    }

    #[tokio::test]
    async fn get_contract_details() {
        let eth_client = test_client();
        let ens_contract = Address::from_str("0x57F1887A8BF19B14FC0DF6FD9B2ACC9AF147EA85").unwrap();
        assert_eq!(
            eth_client.get_contract_details(ens_contract).await,
            ContractDetails {
                name: None,
                symbol: None,
            }
        );
        let bored_ape_contract =
            Address::from_str("0x2EE6AF0DFF3A1CE3F7E3414C52C48FD50D73691E").unwrap();
        assert_eq!(
            eth_client.get_contract_details(bored_ape_contract).await,
            ContractDetails {
                name: Some("Bored Ape Yacht Club".to_string()),
                symbol: Some("BAYC".to_string()),
            }
        );
        let mla_field_agent =
            Address::from_str("0x7A41E410BB784D9875FA14F2D7D2FA825466CDAE").unwrap();
        assert_eq!(
            eth_client.get_contract_details(mla_field_agent).await,
            ContractDetails {
                name: Some("Meta Labs Field Agents".to_string()),
                symbol: Some("MLA1".to_string()),
            }
        );
    }
}

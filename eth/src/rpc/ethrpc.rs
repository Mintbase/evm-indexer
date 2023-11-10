use crate::types::{Address, BlockData, ContractDetails, NftId};
use anyhow::Result;
use async_trait::async_trait;
use ethrpc;
use ethrpc::{eth, http::reqwest::Url, types::TransactionCall, types::*};
use futures::future::{join, join_all};
use solabi::{decode::Decode, encode::Encode, selector, FunctionEncoder};
use std::collections::HashMap;

use super::EthNodeReading;

const NAME: FunctionEncoder<(), (String,)> = FunctionEncoder::new(selector!("name()"));
const SYMBOL: FunctionEncoder<(), (String,)> = FunctionEncoder::new(selector!("symbol()"));
const TOKEN_URI: FunctionEncoder<(U256,), (String,)> =
    FunctionEncoder::new(selector!("tokenURI(uint256)"));
pub struct Client {
    provider: ethrpc::http::Buffered,
}

#[async_trait]
impl EthNodeReading for Client {
    async fn get_blocks_for_range(&self, start: u64, end: u64) -> Result<HashMap<u64, BlockData>> {
        let futures = (start..end).map(|block: u64| {
            self.provider.call(
                eth::GetBlockByNumber,
                (BlockSpec::Number(U256::from(block)), Hydrated::No),
            )
        });

        let blocks = join_all(futures).await;

        let result = blocks
            .into_iter()
            .filter_map(|possible_block| match possible_block.ok()? {
                Some(block) => {
                    let number = block.number.as_u64();
                    Some((
                        number,
                        BlockData {
                            number,
                            time: block.timestamp.as_u64(),
                        },
                    ))
                }
                // Note that Blocks are only None when they don't exist, this rpc is existing data.
                None => None,
            })
            .collect();

        Ok(result)
    }

    async fn get_uris(&self, token_ids: Vec<&NftId>) -> HashMap<NftId, Option<String>> {
        tracing::info!("Preparing {} tokenUri Requests", token_ids.len());
        let futures = token_ids.iter().cloned().map(|token| {
            self.provider
                .call(eth::Call, (Self::uri_call(token), BlockId::default()))
        });

        let uris = join_all(futures).await;

        token_ids
            .into_iter()
            .zip(uris)
            .map(|(id, uri_result)| {
                (
                    *id,
                    match Self::decode_function_result_string(&uri_result, TOKEN_URI) {
                        Ok(val) => val,
                        Err(err) => {
                            tracing::warn!("failed to decode token_uri for {:?}: {:?}", id, err);
                            None
                        }
                    },
                )
            })
            .collect()
    }

    async fn get_contract_details(
        &self,
        addresses: Vec<Address>,
    ) -> HashMap<Address, ContractDetails> {
        tracing::debug!("Preparing {} Contract Details Requests", addresses.len());
        let name_futures = addresses.iter().cloned().map(|addr| {
            self.provider
                .call(eth::Call, (Self::name_call(addr), BlockId::default()))
        });

        let symbol_futures = addresses.iter().cloned().map(|addr| {
            self.provider
                .call(eth::Call, (Self::symbol_call(addr), BlockId::default()))
        });

        let (names, symbols) = join(join_all(name_futures), join_all(symbol_futures)).await;
        tracing::debug!("Complete {} Contract Details Requests", addresses.len());
        addresses
            .into_iter()
            .zip(names.iter().zip(symbols))
            .map(|(address, (name_result, symbol_result))| {
                let name = match Self::decode_function_result_string(name_result, NAME) {
                    Ok(val) => val,
                    Err(err) => {
                        tracing::warn!("failed to decode name for {:?}: {:?}", address, err);
                        None
                    }
                };
                let symbol = match Self::decode_function_result_string(&symbol_result, SYMBOL) {
                    Ok(val) => val,
                    Err(err) => {
                        tracing::warn!("failed to decode symbol for {:?}: {:?}", address, err);
                        None
                    }
                };
                (address, ContractDetails { name, symbol })
            })
            .collect()
    }
}

impl Client {
    pub fn new(url: &str) -> Result<Self> {
        Ok(Self {
            provider: ethrpc::http::Client::new(Url::parse(url)?).buffered(Default::default()),
        })
    }

    fn uri_call(token: &NftId) -> TransactionCall {
        TransactionCall {
            to: Some(token.address.0),
            input: Some(TOKEN_URI.encode_params(&(token.token_id.0,))),
            ..Default::default()
        }
    }

    fn name_call(address: Address) -> TransactionCall {
        TransactionCall {
            to: Some(address.0),
            input: Some(NAME.encode_params(&())),
            ..Default::default()
        }
    }

    fn symbol_call(address: Address) -> TransactionCall {
        TransactionCall {
            to: Some(address.0),
            input: Some(SYMBOL.encode_params(&())),
            ..Default::default()
        }
    }

    fn decode_function_result_string<T>(
        res: &Result<Vec<u8>, ethrpc::http::Error>,
        encoder: FunctionEncoder<T, (String,)>,
    ) -> Result<Option<String>>
    where
        T: Encode + Decode,
    {
        Ok(match res {
            Ok(bytes) => Some(encoder.decode_returns(bytes)?.0.replace('\0', "")),
            Err(err) => {
                tracing::warn!("got {:?}", err.to_string());
                None
            }
        })
    }

    // pub async fn get_block_receipts(
    //     &self,
    //     block: u64,
    //     indices: HashSet<u64>,
    // ) -> Result<HashMap<u64, TxDetails>> {
    //     unimplemented!()
    //     // GetBlockReceipts {
    //     //     provider: self.provider.clone(),
    //     //     block,
    //     //     indices,
    //     // }
    //     // .retry_get(3, 1)
    //     // .await
    // }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::types::U256;
    use maplit::hashmap;
    use std::str::FromStr;

    static FREE_ETH_RPC: &str = "https://rpc.ankr.com/eth";

    fn test_client() -> Client {
        Client::new(FREE_ETH_RPC).expect("Needed for test")
    }

    #[tokio::test]
    async fn get_contract_details() {
        let eth_client = test_client();

        let addresses = [
            "0x966731DFD9B9925DD105FF465687F5AA8F54EE9F",
            "0xD945F759D422AE30A6166838317B937DE08380E3",
            "0xBA30E5F9BB24CAA003E9F2F0497AD287FDF95623",
            "0x5E30AAF41B0DFF94672C8DAF941A1FEC5B8B6AAA",
            "0x3BF42951001BB7CB3CC303068FE87DEBF696EE3D",
            "0x26BADF693F2B103B021C670C852262B379BBBE8A",
            "0xBC4EA4F07F4F897772C3FAD8AAF327973254B72B",
            "0x97ED92E744C10FDD5D403A756239C4069E415E79",
            "0x9D0DE41434C14932D058AD6938FDA6601C720D8E",
            "0xCAACE84B015330C0AB4BD003F6FA0B84EC6C64AC",
        ]
        .map(|s| Address::from_str(s).unwrap())
        .to_vec();
        let details = eth_client.get_contract_details(addresses.clone()).await;

        let expected = addresses
            .into_iter()
            .zip([
                ContractDetails {
                    name: Some("Hero".into()),
                    symbol: Some("HERO".into()),
                },
                ContractDetails {
                    name: Some("Zora API Genesis Hackathon".into()),
                    symbol: Some("ZRPG".into()),
                },
                ContractDetails {
                    name: Some("BoredApeKennelClub".into()),
                    symbol: Some("BAKC".into()),
                },
                ContractDetails {
                    name: Some("kai".into()),
                    symbol: Some("KAI".into()),
                },
                ContractDetails {
                    name: Some("CrashTestJoyride".into()),
                    symbol: Some("CTJR".into()),
                },
                ContractDetails {
                    name: Some("Illuminati".into()),
                    symbol: Some("Truth".into()),
                },
                ContractDetails {
                    name: Some("Light Baths: Waves".into()),
                    symbol: Some("LIGHTWAV".into()),
                },
                ContractDetails {
                    name: Some("White Rabbit Producer Pass".into()),
                    symbol: Some("WRPP".into()),
                },
                ContractDetails {
                    name: Some("Zombie Zebras Comic Issue 2 Cover".into()),
                    symbol: Some("ZZC02C".into()),
                },
                ContractDetails {
                    name: Some("Flower Fam".into()),
                    symbol: Some("FF".into()),
                },
            ])
            .collect();

        assert_eq!(details, expected);
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
        let bored_ape = NftId {
            address: Address::from_str("0x2EE6AF0DFF3A1CE3F7E3414C52C48FD50D73691E").unwrap(),
            token_id: U256::from(16),
        };
        let mla_field_agent = NftId {
            address: Address::from_str("0x7A41E410BB784D9875FA14F2D7D2FA825466CDAE").unwrap(),
            token_id: U256::from(3490),
        };

        let null_bytes = NftId {
            address: Address::from([
                207, 58, 101, 134, 77, 251, 109, 74, 234, 170, 147, 221, 230, 106, 211, 222, 178,
                39, 195, 227,
            ]),
            token_id: U256::from(10063),
        };
        let token_ids = [ens_token, bored_ape, mla_field_agent, null_bytes];

        let uris = eth_client.get_uris(token_ids.iter().collect()).await;

        assert_eq!(
            uris,
            hashmap! {
                ens_token => None,
                mla_field_agent => None,
                bored_ape => Some("ipfs://QmYZNgUhb2AgqU1xGPrdY8SDKuQngfSqSeGwz5bNQD4pZk/metadata.json".to_string()),
                // Note that this is a BAD URL (because of the last 6 characters)!
                null_bytes => Some("https://5h5jydmla4qvcjvmdgcgnnkdhy0ddrod.lambda-url.us-east-2.on.aws/?id=10063&data=".into())
            }
        );
    }
}

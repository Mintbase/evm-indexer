use crate::types::{Address, BlockData, ContractDetails, NftId, TxDetails};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use ethrpc::http::Error;
use ethrpc::{
    eth,
    http::{buffered::Configuration, reqwest::Url, Error as EthRpcError},
    types::TransactionCall,
    types::*,
};
use futures::future::{join, join_all};
use solabi::{decode::Decode, encode::Encode, selector, FunctionEncoder};
use std::{collections::HashMap, fmt::Debug};

use super::EthNodeReading;

const NAME: FunctionEncoder<(), (String,)> = FunctionEncoder::new(selector!("name()"));
const SYMBOL: FunctionEncoder<(), (String,)> = FunctionEncoder::new(selector!("symbol()"));
const TOKEN_URI: FunctionEncoder<(U256,), (String,)> =
    FunctionEncoder::new(selector!("tokenURI(uint256)"));
pub struct Client {
    provider: ethrpc::http::Buffered,
}

fn handle_error(error: EthRpcError, context: &str) {
    match error {
        Error::Json(err) => {
            panic!("Json Error {}", err);
        }
        Error::Http(err) => {
            panic!("Http Error {}", err);
        }
        Error::Status(code, message) => {
            panic!("Status Error with code {code} and message {}", message);
        }
        Error::Rpc(err) => {
            if !err.message.contains("execution reverted") {
                tracing::warn!("request failed: {context} with {err:?}");
            }
            // Contract function does not exist or no longer returns a value for given input.
        }
        Error::Batch(err) => {
            panic!("Batch Error {}", err);
        }
    }
}

#[async_trait]
impl EthNodeReading for Client {
    async fn get_blocks_for_range(&self, start: u64, end: u64) -> Result<HashMap<u64, BlockData>> {
        let futures = (start..end).map(|block: u64| {
            self.provider.call(
                eth::GetBlockByNumber,
                (BlockSpec::Number(U256::from(block)), Hydrated::Yes),
            )
        });

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
            .flatten()
            .map(|block| {
                let number = block.number.as_u64();
                let transactions = match block.transactions {
                    BlockTransactions::Full(txs) => txs,
                    BlockTransactions::Hash(hashes) => match hashes.len() {
                        // This happens when a block has no transactions
                        0 => vec![],
                        _ => unreachable!("expected Full for Hydrated block={}", number),
                    },
                };
                (
                    number,
                    BlockData {
                        number,
                        time: block.timestamp.as_u64(),
                        transactions: transactions
                            .into_iter()
                            .map(|tx| (tx.transaction_index().as_u64(), TxDetails::from(tx)))
                            .collect(),
                    },
                )
            })
            .collect())
    }

    async fn get_uris(&self, token_ids: &[NftId]) -> HashMap<NftId, Option<String>> {
        tracing::info!("preparing {} tokenUri requests", token_ids.len());
        let futures = token_ids.iter().map(|token| {
            self.provider
                .call(eth::Call, (Self::uri_call(token), BlockId::default()))
        });

        let uri_results = join_all(futures).await;
        tracing::debug!("completed tokenUri requests");
        token_ids
            .iter()
            .zip(uri_results)
            .map(|(&id, uri_result)| {
                let uri = match uri_result {
                    Ok(bytes) => Self::decode_function_result_string(bytes, TOKEN_URI),
                    Err(err) => {
                        handle_error(err, &format!("tokenUri for {id:?}"));
                        None
                    }
                };
                (id, uri)
            })
            .collect()
    }

    async fn get_contract_details(
        &self,
        addresses: &[Address],
    ) -> HashMap<Address, ContractDetails> {
        tracing::info!("preparing {} contract details requests", addresses.len());
        let name_futures = addresses.iter().cloned().map(|addr| {
            self.provider
                .call(eth::Call, (Self::name_call(addr), BlockId::default()))
        });

        let symbol_futures = addresses.iter().cloned().map(|addr| {
            self.provider
                .call(eth::Call, (Self::symbol_call(addr), BlockId::default()))
        });

        let (names, symbols) = join(join_all(name_futures), join_all(symbol_futures)).await;
        tracing::debug!("complete {} contract details requests", addresses.len());

        addresses
            .iter()
            .zip(names.into_iter().zip(symbols))
            .map(|(&address, (name_result, symbol_result))| {
                let name = match name_result {
                    Ok(name) => Self::decode_function_result_string(name, NAME),
                    Err(err) => {
                        handle_error(err, &format!("name for {address}"));
                        None
                    }
                };
                let symbol = match symbol_result {
                    Ok(symbol) => Self::decode_function_result_string(symbol, SYMBOL),
                    Err(err) => {
                        handle_error(err, &format!("symbol for {address}"));
                        None
                    }
                };
                (
                    address,
                    ContractDetails {
                        address,
                        name,
                        symbol,
                    },
                )
            })
            .collect()
    }
}

impl Client {
    pub fn new(url: &str) -> Result<Self> {
        Ok(Self {
            provider: ethrpc::http::Client::new(Url::parse(url)?)
                .buffered(Configuration::default()),
        })
    }

    fn unpack_results<T: Debug>(results: Vec<Result<T, Error>>) -> (Vec<T>, Vec<Error>) {
        let (oks, errors): (Vec<_>, Vec<_>) = results.into_iter().partition(Result::is_ok);
        let oks: Vec<_> = oks.into_iter().map(Result::unwrap).collect();
        let errors: Vec<ethrpc::http::Error> = errors.into_iter().map(Result::unwrap_err).collect();
        (oks, errors)
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
        res: Vec<u8>,
        encoder: FunctionEncoder<T, (String,)>,
    ) -> Option<String>
    where
        T: Encode + Decode,
    {
        match encoder.decode_returns(&res) {
            Ok(decoded_string) => Some(decoded_string.0.replace('\0', "")),
            Err(err) => {
                if !res.is_empty() {
                    // Only log if result is non-empty
                    tracing::warn!("failed to decode bytes {:?} with {}", res, err);
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::types::U256;
    use ethrpc::jsonrpc;
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
        let details = eth_client.get_contract_details(&addresses.clone()).await;

        let expected = addresses
            .clone()
            .into_iter()
            .zip([
                ContractDetails {
                    address: addresses[0],
                    name: Some("Hero".into()),
                    symbol: Some("HERO".into()),
                },
                ContractDetails {
                    address: addresses[1],
                    name: Some("Zora API Genesis Hackathon".into()),
                    symbol: Some("ZRPG".into()),
                },
                ContractDetails {
                    address: addresses[2],
                    name: Some("BoredApeKennelClub".into()),
                    symbol: Some("BAKC".into()),
                },
                ContractDetails {
                    address: addresses[3],
                    name: Some("kai".into()),
                    symbol: Some("KAI".into()),
                },
                ContractDetails {
                    address: addresses[4],
                    name: Some("CrashTestJoyride".into()),
                    symbol: Some("CTJR".into()),
                },
                ContractDetails {
                    address: addresses[5],
                    name: Some("Illuminati".into()),
                    symbol: Some("Truth".into()),
                },
                ContractDetails {
                    address: addresses[6],
                    name: Some("Light Baths: Waves".into()),
                    symbol: Some("LIGHTWAV".into()),
                },
                ContractDetails {
                    address: addresses[7],
                    name: Some("White Rabbit Producer Pass".into()),
                    symbol: Some("WRPP".into()),
                },
                ContractDetails {
                    address: addresses[8],
                    name: Some("Zombie Zebras Comic Issue 2 Cover".into()),
                    symbol: Some("ZZC02C".into()),
                },
                ContractDetails {
                    address: addresses[9],
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

        let uris = eth_client.get_uris(&token_ids).await;

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

    #[test]
    fn error_handling() {
        let inner_error = jsonrpc::Error::custom("execution reverted");
        assert!(inner_error.message.contains("execution reverted"));
        let rpc_error = EthRpcError::Rpc(inner_error);
        handle_error(rpc_error, "test");
    }
}

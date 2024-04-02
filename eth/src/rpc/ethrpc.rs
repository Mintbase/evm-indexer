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
use futures::future::join_all;
use futures::FutureExt;
use solabi::{decode::Decode, encode::Encode, selector, FunctionEncoder};
use std::future::Future;
use std::time::Duration;
use std::{collections::HashMap, fmt::Debug};

use super::EthNodeReading;

const NAME: FunctionEncoder<(), (String,)> = FunctionEncoder::new(selector!("name()"));
const SYMBOL: FunctionEncoder<(), (String,)> = FunctionEncoder::new(selector!("symbol()"));
const TOKEN_URI: FunctionEncoder<(U256,), (String,)> =
    FunctionEncoder::new(selector!("tokenURI(uint256)"));
pub struct Client {
    provider: ethrpc::http::Buffered,
}

/// Runs futures generated from an iterator of items,
/// ensuring that the order of results matches the order of items.
///
/// `iter`: An iterator over items for which futures will be generated and executed.
/// `f`: A closure that takes an item from the iterator and returns a future.
///
/// Returns a `Vec` of results, sorted by the original index to match the order of items.
async fn run_futures_in_order<I, F, Fut, T, E>(iter: I, mut f: F) -> Vec<Result<T, E>>
where
    I: IntoIterator,
    I::Item: Send,
    F: FnMut(I::Item) -> Fut,
    Fut: Future<Output = Result<T, E>> + Send,
    T: Send,
    E: Send,
{
    let futures: Vec<_> = iter
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            // Apply the function to each item to create a future and pair it with its index
            let future = f(item).map(move |result| (index, result));
            // Box the future to ensure it has a static size
            future.boxed()
        })
        .collect();

    // Wait for all futures to complete
    let mut results: Vec<(usize, Result<T, E>)> = join_all(futures).await;

    // Sort the results by index
    results.sort_by_key(|(index, _)| *index);

    // Map the sorted results to only include the result, in the correct order
    results.into_iter().map(|(_, result)| result).collect()
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
            panic!("Status Error with code {} and message {}", code, message);
        }
        Error::Rpc(err) => {
            let known_rpc_errors = [
                // Contract does not have attempted functionality
                // or function exists and some assertion failed.
                "execution reverted",
                // Contract method is unable to respond to the given input.
                "out of gas",
            ];
            if !known_rpc_errors.iter().any(|e| err.message.contains(e)) {
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
        let results = run_futures_in_order(token_ids, |token| {
            self.provider
                .call(eth::Call, (Self::uri_call(token), BlockId::default()))
        })
        .await;
        tracing::debug!("completed tokenUri requests");
        token_ids
            .iter()
            .zip(results)
            .map(|(&id, uri_result)| {
                let uri = match uri_result {
                    Ok(bytes) => Self::decode_function_result_string(bytes, TOKEN_URI),
                    Err(err) => {
                        handle_error(err, &format!("tokenUri for {id}"));
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
        let names = run_futures_in_order(addresses, |addr| {
            self.provider
                .call(eth::Call, (Self::name_call(addr), BlockId::default()))
        })
        .await;

        let symbols = run_futures_in_order(addresses, |addr| {
            self.provider
                .call(eth::Call, (Self::symbol_call(addr), BlockId::default()))
        })
        .await;

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
    pub fn new(url: &str, batch_delay: u64) -> Result<Self> {
        Ok(Self {
            provider: ethrpc::http::Client::new(Url::parse(url)?).buffered(Configuration {
                delay: Duration::from_millis(batch_delay),
                max_size: 2,
                ..Default::default()
            }),
        })
    }

    fn unpack_results<T: Debug>(results: Vec<Result<T, Error>>) -> (Vec<T>, Vec<Error>) {
        let (oks, errors): (Vec<_>, Vec<_>) = results.into_iter().partition(Result::is_ok);
        let oks: Vec<_> = oks.into_iter().map(Result::unwrap).collect();
        let errors: Vec<Error> = errors.into_iter().map(Result::unwrap_err).collect();
        (oks, errors)
    }

    fn uri_call(token: &NftId) -> TransactionCall {
        TransactionCall {
            to: Some(token.address.0),
            input: Some(TOKEN_URI.encode_params(&(token.token_id.0,))),
            ..Default::default()
        }
    }

    fn name_call(address: &Address) -> TransactionCall {
        TransactionCall {
            to: Some(address.0),
            input: Some(NAME.encode_params(&())),
            ..Default::default()
        }
    }

    fn symbol_call(address: &Address) -> TransactionCall {
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
    use ethrpc::jsonrpc;
    use maplit::hashmap;
    use std::str::FromStr;
    use tokio::time::{sleep, Duration};

    static FREE_ETH_RPC: &str = "https://rpc.ankr.com/eth";

    fn test_client() -> Client {
        Client::new(FREE_ETH_RPC, 0).expect("Needed for test")
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
        let ens_token = NftId::from_str("0x57F1887A8BF19B14FC0DF6FD9B2ACC9AF147EA85/64671196571681841248190411691641946869002480279128285790058847953168666315").unwrap();
        let bored_ape = NftId::from_str("0x2EE6AF0DFF3A1CE3F7E3414C52C48FD50D73691E/16").unwrap();
        let mla_field_agent =
            NftId::from_str("0x7A41E410BB784D9875FA14F2D7D2FA825466CDAE/3490").unwrap();
        let null_bytes =
            NftId::from_str("0xcf3a65864DFB6d4aEAaa93Dde66ad3deb227c3E3/10063").unwrap();

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

    // Helper function for a mocked async operation
    async fn async_operation(input: usize, delay_ms: u64) -> Result<String, ()> {
        sleep(Duration::from_millis(delay_ms)).await;
        Ok(format!("Result {}", input))
    }

    #[tokio::test]
    async fn test_run_futures_in_order() {
        let inputs = vec![2, 3, 1]; // Intentionally out of order
        let delays = vec![300, 100, 200]; // Different delays to ensure tasks finish out of input order

        let futures = inputs
            .into_iter()
            .zip(delays.into_iter())
            .map(|(input, delay)| (input, async move { async_operation(input, delay).await }));

        let results = run_futures_in_order(futures, |(_, future)| future).await;

        let expected: Vec<Result<String, ()>> = vec![
            Ok("Result 2".to_string()),
            Ok("Result 3".to_string()),
            Ok("Result 1".to_string()),
        ];
        assert_eq!(results, expected);
    }

    #[tokio::test]
    async fn test_handles_errors() {
        async fn async_op_may_fail(input: usize) -> Result<String, &'static str> {
            if input % 2 == 0 {
                Ok(format!("Even {}", input))
            } else {
                Err("Odd number error")
            }
        }

        let inputs = vec![2, 4, 3]; // Includes a value that will cause an error

        let results = run_futures_in_order(inputs, async_op_may_fail).await;

        let expected = vec![
            Ok("Even 2".to_string()),
            Ok("Even 4".to_string()),
            Err("Odd number error"),
        ];
        assert_eq!(results, expected);
    }
}

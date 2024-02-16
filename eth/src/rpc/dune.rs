use anyhow::{anyhow, Result};
use std::fs::File;
use std::{collections::HashMap, str::FromStr};

use async_trait::async_trait;
use csv::Writer;
use duners::{client::DuneClient, parameters::Parameter};
use serde::{Deserialize, Serialize};

use crate::types::{Address, BlockData, Bytes32, ContractDetails, NftId, TxDetails};

use super::EthNodeReading;

#[derive(Clone, Deserialize, Debug, PartialEq, Serialize)]
pub struct BlockTransaction {
    pub block_time: u64,
    pub block_number: u64,
    pub index: u64,
    pub hash: String,
    pub from: String,
    pub to: Option<String>,
}

impl BlockTransaction {
    fn write_collection_to_csv<I>(iter: I, file_path: &str) -> Result<(), csv::Error>
    where
        I: Iterator<Item = BlockTransaction>,
    {
        // TODO - avoid overwriting files and check first
        //  shouldn't even execute the query if file exists.
        let file = File::create(file_path)?;
        let mut writer = Writer::from_writer(file);

        for item in iter {
            writer.serialize(&item)?;
        }

        writer.flush()?;
        Ok(())
    }
}

#[async_trait]
impl EthNodeReading for DuneClient {
    async fn get_contract_details(
        &self,
        _addresses: &[Address],
    ) -> HashMap<Address, ContractDetails> {
        unimplemented!("Sorry!")
    }

    async fn get_uris(&self, _token_ids: &[NftId]) -> HashMap<NftId, Option<String>> {
        unimplemented!("Sorry!")
    }

    async fn get_blocks_for_range(&self, start: u64, end: u64) -> Result<HashMap<u64, BlockData>> {
        let file_path = format!("block_tx_{}_{}.csv", start, end);
        // TODO - check if file exists and warn/don't run query.
        tracing::debug!("Querying blocks for range {start} - {end}");
        let block_transactions = self
            .refresh::<BlockTransaction>(
                3238193,
                Some(vec![
                    Parameter::number("Start", &start.to_string()),
                    Parameter::number("Width", &(end - start).to_string()),
                ]),
                Some(1),
            )
            .await
            .map_err(|err| anyhow!("dune tx query: {:?}", err))?
            .get_rows();
        tracing::debug!("Got {} results", block_transactions.len());

        BlockTransaction::write_collection_to_csv(block_transactions.iter().cloned(), &file_path)?;

        let block_data_map: HashMap<u64, BlockData> =
            block_transactions
                .into_iter()
                .fold(HashMap::new(), |mut acc, tx| {
                    // Extract block_number from the transaction
                    let block_number = tx.block_number;

                    // Create a BlockData if it doesn't exist in the accumulator
                    let block_data = acc.entry(block_number).or_insert_with(|| BlockData {
                        number: block_number,
                        time: tx.block_time,
                        transactions: HashMap::new(),
                    });

                    // Add the transaction to the BlockData's transactions HashMap
                    block_data.transactions.insert(
                        tx.index,
                        TxDetails {
                            hash: Bytes32::from_str(&tx.hash).expect("dune hash"),
                            from: Address::from_str(&tx.from).expect("dune address"),
                            to: tx
                                .to
                                .map(|address| Address::from_str(&address).expect("parse Address")),
                        },
                    );

                    acc
                });

        Ok(block_data_map)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    #[ignore]
    async fn try_client() {
        // This test demonstrates that we can index the first Million Ethereum Blocks in 70 Seconds.
        let dune = DuneClient::from_env();
        let start = 15_000_000;
        let result = dune.get_blocks_for_range(start, start + 100).await;
        assert!(result.is_ok());
    }

    #[test]
    fn serialize() {
        // This test demonstrates that we can index the first Million Ethereum Blocks in 70 Seconds.
        let sample_records = vec![
            BlockTransaction {
                block_time: 0,
                block_number: 0,
                index: 0,
                hash: "0x1bb01678429765366f7b3956018765618d9290136f47ce08ce8168058ae6c5e5"
                    .to_string(),
                from: "0x300bef96a6cb272ee1847bf75177168b8b97556b".to_string(),
                to: None,
            },
            BlockTransaction {
                block_time: 1,
                block_number: 2,
                index: 3,
                hash: "0xd83a0928f99dcbde78d725c41d47385ed03c845d7dcd82cccd09766090442168"
                    .to_string(),
                from: "0x9b69609d429c7acef0b9d7fd9fd413771f7f8521".to_string(),
                to: Some("0xe8f1a89ae62e64c1547ed28bf84c279b76a93072".to_string()),
            },
        ];

        for record in &sample_records {
            let json_string = serde_json::to_string(record).unwrap();
            println!("{}", json_string);
        }
    }
}

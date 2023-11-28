use anyhow::{anyhow, Result};
use std::{collections::HashMap, str::FromStr};

use async_trait::async_trait;
use duners::{client::DuneClient, parameters::Parameter};
use serde::Deserialize;

use crate::types::{Address, BlockData, Bytes32, ContractDetails, NftId, TxDetails};

use super::EthNodeReading;

#[derive(Deserialize, Debug, PartialEq)]
pub struct Block {
    pub number: u64,
    pub time: u64,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct Transaction {
    pub block_number: u64,
    pub index: u64,
    pub hash: String,
    pub from: String,
    pub to: Option<String>,
}

#[async_trait]
impl EthNodeReading for DuneClient {
    async fn get_contract_details(
        &self,
        _addresses: Vec<Address>,
    ) -> HashMap<Address, ContractDetails> {
        unimplemented!("Sorry!")
    }

    async fn get_uris(&self, _token_ids: Vec<NftId>) -> HashMap<NftId, Option<String>> {
        unimplemented!("Sorry!")
    }

    async fn get_blocks_for_range(&self, start: u64, end: u64) -> Result<HashMap<u64, BlockData>> {
        let blocks = self
            .refresh::<Block>(
                3238189,
                Some(vec![
                    Parameter::number("Start", &start.to_string()),
                    Parameter::number("Width", &(end - start).to_string()),
                ]),
                Some(1),
            )
            .await
            .map_err(|err| anyhow!(format!("dune block query: {:?}", err)))?
            .get_rows();

        let transactions = self
            .refresh::<Transaction>(
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
        let mut tx_map: HashMap<u64, _> = HashMap::new();
        for tx in transactions {
            tx_map
                .entry(tx.block_number)
                .or_insert(HashMap::new())
                .insert(
                    tx.index,
                    TxDetails {
                        hash: Bytes32::from_str(&tx.hash)?,
                        from: Address::from_str(&tx.from)?,
                        to: tx
                            .to
                            .map(|address| Address::from_str(&address).expect("parse Address")),
                    },
                );
        }
        Ok(blocks
            .into_iter()
            .map(|block| {
                (
                    block.number,
                    BlockData {
                        number: block.number,
                        time: block.time,
                        transactions: tx_map.remove(&block.number).unwrap_or_default(),
                    },
                )
            })
            .collect())
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
        let result = dune.get_blocks_for_range(1, 1_000_000).await;
        assert!(result.is_ok());
    }
}

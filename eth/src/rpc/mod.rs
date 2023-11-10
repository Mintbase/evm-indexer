pub mod ethers;
pub mod ethrpc;
use crate::types::{Address, BlockData, ContractDetails, NftId};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait EthNodeReading {
    async fn get_contract_details(
        &self,
        addresses: Vec<Address>,
    ) -> HashMap<Address, ContractDetails>;

    async fn get_uris(&self, token_ids: Vec<&NftId>) -> HashMap<NftId, Option<String>>;

    async fn get_blocks_for_range(&self, start: u64, end: u64) -> Result<HashMap<u64, BlockData>>;

    // This is waiting for: https://github.com/nlordell/ethrpc-rs/pull/3
    // async fn get_receipts_for_range(
    //     &self,
    //     start: u64,
    //     end: u64,
    // ) -> Result<HashMap<u64, HashMap<u64, TxDetails>>>;
}

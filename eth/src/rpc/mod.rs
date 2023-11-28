pub mod dune;
pub mod ethers;
pub mod ethrpc;
use crate::types::{Address, BlockData, ContractDetails, NftId};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait EthNodeReading: Send + Sync {
    async fn get_contract_details(
        &self,
        addresses: &[Address],
    ) -> HashMap<Address, ContractDetails>;

    async fn get_uris(&self, token_ids: &[NftId]) -> HashMap<NftId, Option<String>>;

    async fn get_blocks_for_range(&self, start: u64, end: u64) -> Result<HashMap<u64, BlockData>>;

    async fn get_uris_and_contract_details(
        &self,
        tokens: &[NftId],
        addresses: &[Address],
    ) -> (
        HashMap<NftId, Option<String>>,
        HashMap<Address, ContractDetails>,
    ) {
        futures::future::join(self.get_uris(tokens), self.get_contract_details(addresses)).await
    }
}

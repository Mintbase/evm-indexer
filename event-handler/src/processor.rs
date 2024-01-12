use crate::{
    config::{ChainDataSource, HandlerConfig},
    handlers::EventHandler,
    update_cache::UpdateCache,
};
use anyhow::{Context, Result};
use data_store::{
    models::{TokenContract, Transaction},
    store::DataStore,
};
use eth::{rpc::ethrpc::Client as EthRpcClient, rpc::EthNodeReading, types::BlockData};
use event_retriever::db_reader::{
    diesel::{BlockRange, EventSource},
    models::*,
};
use std::{collections::HashMap, sync::Arc};

pub struct EventProcessor {
    /// Source of events for processing
    source: EventSource,
    /// Location of existing stored content
    pub store: DataStore,
    /// A memory store updates.
    pub updates: UpdateCache,
    /// Web3 Provider
    eth_client: Arc<dyn EthNodeReading>,
    /// Runtime configuration parameters
    config: HandlerConfig,
}

impl EventProcessor {
    pub fn new(
        source_url: &str,
        store_url: &str,
        eth_rpc: &str,
        config: HandlerConfig,
    ) -> Result<Self> {
        Ok(Self {
            source: EventSource::new(source_url).context("init EventSource")?,
            store: DataStore::new(store_url).context("init DataStore")?,
            updates: UpdateCache::default(),
            eth_client: Arc::new(EthRpcClient::new(eth_rpc).context("init EthRpcClient")?),
            config,
        })
    }

    pub async fn run(&mut self, start_from: i64) -> Result<()> {
        let mut current_block = start_from;
        loop {
            // TODO - (after reorg handling) Replace with get_indexed_block (finalized is safe)
            //  https://github.com/Mintbase/evm-indexer/issues/104
            let max_block = self.source.get_finalized_block();

            if current_block >= max_block {
                // Exit when reached or exceeded the max_block
                break;
            }

            let end_block = current_block + self.config.page_size - 1;
            let block_range = BlockRange {
                start: current_block,
                end: end_block.min(max_block), // Ensure we don't exceed max_block
            };

            self.process_events_for_block_range(block_range).await?;

            // Update current_block for the next iteration
            let processed_block = self.store.get_processed_block();
            if processed_block <= current_block {
                // Ensure progress, or break the loop
                break;
            }
            current_block = processed_block + 1;
        }

        Ok(())
    }

    fn check_for_contract(&mut self, event: &EventBase) {
        let address = event.contract_address;
        if self.updates.contracts.contains_key(&address)
            || self.store.load_contract(address).is_some()
        {
            return;
        }

        self.updates
            .contracts
            .insert(address, TokenContract::from_event_base(event));
    }
    async fn load_chain_data(&mut self, range: BlockRange) -> Result<HashMap<u64, BlockData>> {
        let block_info = match self.config.chain_data_source {
            ChainDataSource::Database => {
                tracing::info!("retrieving block and transaction data from arak");
                self.source.get_blocks_for_range(range)?
            }
            ChainDataSource::Node => {
                tracing::info!("retrieving block and transaction data from node");
                self.eth_client
                    .get_blocks_for_range(range.start as u64, range.end as u64)
                    .await?
            }
        };
        self.updates
            .transactions
            .extend(
                block_info
                    .clone()
                    .into_iter()
                    .flat_map(|(block, block_data)| {
                        block_data
                            .transactions
                            .into_iter()
                            .map(move |(idx, data)| Transaction::new(block, idx, data))
                    }),
            );

        self.updates.blocks.extend(block_info.clone().into_values());
        Ok(block_info)
    }

    async fn get_missing_node_data(&mut self) {
        // TODO - (after metadata-retrieving) this functionality will be replaced by metadata-retriever.
        //  https://github.com/Mintbase/evm-indexer/issues/105
        let (mut missing_uris, mut contract_details) = self
            .eth_client
            .get_uris_and_contract_details(
                self.updates
                    .nfts
                    .clone()
                    .into_iter()
                    // Without additional specification here this will retry to fetch things
                    // We can prevent this by perhaps by filtering also for range.start < mint_block
                    .filter(|(_, token)| token.token_uri.is_none())
                    .map(|(id, _)| id)
                    .collect::<Vec<_>>()
                    .as_slice(),
                self.updates
                    .contracts
                    .keys()
                    .copied()
                    .collect::<Vec<_>>()
                    .as_slice(),
            )
            .await;
        tracing::info!(
            "retrieved missing node data for {} contracts and {} tokens",
            contract_details.len(),
            missing_uris.len()
        );
        for (id, possible_uri) in missing_uris.drain() {
            if let Some(uri) = possible_uri {
                self.updates.nfts.get_mut(&id).expect("known").token_uri = Some(uri);
            }
        }

        for (address, details) in contract_details.drain() {
            let contract = self
                .updates
                .contracts
                .get_mut(&address)
                .expect("known to exist");
            contract.name = details.name;
            contract.symbol = details.symbol;
        }
    }

    async fn process_events_for_block_range(&mut self, range: BlockRange) -> Result<()> {
        tracing::info!("processing events for {:?}", range);
        let event_map = self.source.get_events_for_block_range(range)?;
        let mut block_data = self.load_chain_data(range).await?;
        for (block, block_events) in event_map.into_iter() {
            let tx_data = block_data
                .remove(&block)
                .unwrap_or_else(|| panic!("Missing block {} in {:?}", block, range))
                .transactions;
            for ((tx_index, _), tx_events) in block_events {
                let tx = tx_data.get(&tx_index).expect("receipt known to exist!");
                for NftEvent { base, meta } in tx_events.into_iter() {
                    self.check_for_contract(&base);
                    match meta {
                        EventMeta::Erc721Approval(a) => self.handle_event(base, a, tx),
                        EventMeta::Erc721Transfer(t) => self.handle_event(base, t, tx),
                        EventMeta::Erc1155TransferBatch(mut batch) => {
                            // Squash the event to avoid unintentional replay protection errors.
                            batch.squash();
                            for (id, value) in batch.ids.into_iter().zip(batch.values.into_iter()) {
                                self.handle_event(
                                    base,
                                    Erc1155TransferSingle {
                                        operator: batch.operator,
                                        from: batch.from,
                                        to: batch.to,
                                        id,
                                        value,
                                    },
                                    tx,
                                )
                            }
                        }
                        EventMeta::Erc1155TransferSingle(t) => self.handle_event(base, t, tx),
                        EventMeta::Erc1155Uri(e) => self.handle_event(base, e, tx),
                        EventMeta::ApprovalForAll(a) => self.handle_event(base, a, tx),
                    };
                }
            }
        }
        tracing::debug!("events processed, retrieving missing node data");
        // TODO (Once we have metadata-retrieval)
        //  this will have to happen AFTER updates.
        //  The service expects records to exist attempting to update.
        // Retrieve missing data from node.
        match self.config.fetch_metadata {
            true => self.get_missing_node_data().await,
            // This is a placeholder for metadata retrieving invocation.
            // Make pubsub post here.
            false => (),
        }

        // Drain cache and write to store
        self.updates.write(&mut self.store).await;
        tracing::info!("completed event processing for {:?}", range);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event_retriever::db_reader::diesel::BlockRange;
    use tracing_test::traced_test;
    static TEST_SOURCE_URL: &str = "postgresql://postgres:postgres@localhost:5432/arak";
    static TEST_STORE_URL: &str = "postgresql://postgres:postgres@localhost:5432/store";
    static TEST_ETH_RPC: &str = "https://rpc.ankr.com/eth";

    fn test_processor() -> EventProcessor {
        EventProcessor::new(
            TEST_SOURCE_URL,
            TEST_STORE_URL,
            TEST_ETH_RPC,
            HandlerConfig {
                chain_data_source: ChainDataSource::Database,
                page_size: 100,
                fetch_metadata: false,
            },
        )
        .unwrap()
    }

    #[tokio::test]
    #[traced_test]
    async fn event_processing() {
        let mut handler = test_processor();
        let block = std::cmp::max(handler.store.get_processed_block() + 1, 15_000_000);
        let range = BlockRange {
            start: block,
            end: block + 5,
        };
        let result = handler.process_events_for_block_range(range).await;
        match result {
            Ok(_) => assert_eq!(handler.store.get_processed_block(), range.end - 1),
            Err(err) => panic!("{}", err.to_string()),
        }
    }

    #[tokio::test]
    #[ignore = "end-to-end test"]
    #[traced_test]
    async fn test_run() {
        let mut handler = test_processor();
        let start_from = std::cmp::max(handler.store.get_processed_block() + 1, 15_000_000);
        let result = handler.run(start_from).await;
        assert!(result.is_ok());
    }
}

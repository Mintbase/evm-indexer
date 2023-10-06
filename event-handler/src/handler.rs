use crate::store::DataStore;
use anyhow::{Context, Result};
use event_retriever::db_reader::diesel::EventSource;
use event_retriever::db_reader::models::*;
pub struct EventHandler {
    /// source of events processing
    source: EventSource,
    // Location of existing stored content
    store: DataStore,
}

impl EventHandler {
    pub fn new(source_url: &str, store_url: &str) -> Result<Self> {
        Ok(Self {
            source: EventSource::new(source_url).context("init EventSource")?,
            store: DataStore::new(store_url).context("init DataStore")?,
        })
    }
    pub fn process_events_for_block(mut self, block_number: i64) -> Result<()> {
        let events = self.source.get_events_for_block(block_number)?;
        tracing::debug!("Retrieved {} events", events.len());
        let _ = events.into_iter().map(|NftEvent { base, meta }| {
            match meta {
                EventMeta::ApprovalForAll(a) => Self::handle_approval_for_all(base, a),
                EventMeta::Erc1155TransferBatch(batch) => {
                    for (id, value) in batch.ids.into_iter().zip(batch.values.into_iter()) {
                        Self::handle_erc1155_transfer(
                            base,
                            Erc1155TransferSingle {
                                operator: batch.operator,
                                from: batch.from,
                                to: batch.to,
                                id,
                                value,
                            },
                        )
                    }
                }
                EventMeta::Erc1155TransferSingle(t) => Self::handle_erc1155_transfer(base, t),
                EventMeta::Erc1155Uri(uri) => Self::handle_erc1155_uri(base, uri),
                EventMeta::Erc721Approval(a) => Self::handle_erc721_approval(base, a),
                EventMeta::Erc721Transfer(transfer) => Self::handle_erc721_transfer(base, transfer),
            };
        });

        Ok(())
    }

    fn handle_approval_for_all(base: EventBase, approval: ApprovalForAll) {
        let ApprovalForAll {
            owner,
            operator,
            approved,
        } = approval;
        let log_word = match approved {
            true => "approved",
            false => "revoked",
        };
        tracing::debug!(
            "{:?} {log_word} {:?} as operator of all their {}",
            owner,
            operator,
            base.contract_address
        )
    }

    fn handle_erc1155_transfer(base: EventBase, transfer: Erc1155TransferSingle) {
        // This will be the place where we handle minting, burning, and generic transfer.
        // Some sample code used to process such an event for a subgraph can be found here:
        // https://github.com/verynifty/NFT-subgraph/blob/0c128f0aa126abf7ba19eaac2ad87e98fa9710df/src/mappings-erc-1155.ts#L46-L70
        tracing::debug!("Processing {:?} of {:?}", transfer, base.contract_address);
    }

    fn handle_erc721_approval(base: EventBase, approval: Erc721Approval) {
        tracing::debug!("Processing {:?} of {:?}", approval, base.contract_address);
    }

    fn handle_erc721_transfer(base: EventBase, transfer: Erc721Transfer) {
        // Note that these may also include Erc20 Transfers (and we will have to handle that).
        tracing::debug!("Processing {:?} of {:?}", transfer, base.contract_address);
    }

    fn handle_erc1155_uri(base: EventBase, uri: Erc1155Uri) {
        tracing::debug!("Processing {:?} of {:?}", uri, base.contract_address);
    }
}

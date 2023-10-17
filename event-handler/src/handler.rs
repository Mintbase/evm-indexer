use crate::{models::NftId, store::DataStore};
use anyhow::{Context, Result};
use event_retriever::db_reader::{
    diesel::{BlockRange, EventSource},
    models::*,
};

pub struct EventHandler {
    /// Source of events for processing
    source: EventSource,
    /// Location of existing stored content
    store: DataStore,
}

impl EventHandler {
    pub fn new(source_url: &str, store_url: &str) -> Result<Self> {
        Ok(Self {
            source: EventSource::new(source_url).context("init EventSource")?,
            store: DataStore::new(store_url).context("init DataStore")?,
        })
    }
    pub fn process_events_for_block_range(&mut self, range: BlockRange) -> Result<()> {
        let events = self.source.get_events_for_block_range(range)?;
        tracing::debug!("Retrieved {} events for {:?}", events.len(), range);
        for NftEvent { base, meta } in events.into_iter() {
            // TODO - fetch transaction hashes for block.
            //  eth_getTransactionByBlockNumberAndIndex OR
            //  eth_getBlockByNumber (with true flag for hashes)
            match meta {
                EventMeta::Erc721Approval(a) => self.handle_erc721_approval(base, a)?,
                _ => continue,
            };
        }
        Ok(())
    }
    fn handle_erc721_approval(
        &mut self,
        base: EventBase,
        approval: Erc721Approval,
    ) -> Result<usize> {
        tracing::debug!("Processing {:?} of {:?}", approval, base.contract_address);
        let nft_id = NftId {
            address: base.contract_address,
            token_id: approval.id,
        };
        match self.store.set_approval(&nft_id, approval.approved) {
            Ok(affected_rows) => Ok(affected_rows),
            Err(err) => {
                tracing::warn!("{}", err.to_string());
                Ok(0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::NftId;
    use event_retriever::db_reader::diesel::BlockRange;
    use shared::eth::Address;
    use shared::eth::U256;
    use tracing::Level;
    static TEST_SOURCE_URL: &str = "postgresql://postgres:postgres@localhost:5432/arak";
    static TEST_STORE_URL: &str = "postgresql://postgres:postgres@localhost:5432/store";

    fn test_handler() -> EventHandler {
        let mut handler = EventHandler::new(TEST_SOURCE_URL, TEST_STORE_URL).unwrap();
        handler.store.clear_tables();
        handler
    }

    #[test]
    fn event_processing() {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(Level::DEBUG)
            .finish();

        let mut handler = EventHandler::new(TEST_SOURCE_URL, TEST_STORE_URL).unwrap();
        let block = 15_000_000;
        tracing::subscriber::with_default(subscriber, || {
            assert!(handler
                .process_events_for_block_range(BlockRange {
                    start: block,
                    end: block + 10,
                })
                .is_ok())
        });
        // TODO - construct a sequence of events and actually check the Store State is as expected here.
    }

    #[test]
    fn erc721_approval_handler() {
        let mut handler = test_handler();
        let contract_address = Address::from(1);
        let token_id = U256::from(123);
        let token = NftId {
            address: contract_address,
            token_id,
        };
        let base = EventBase {
            block_number: 1,
            log_index: 2,
            transaction_index: 3,
            contract_address,
        };
        let approval = Erc721Approval {
            owner: Address::from(2),
            approved: Address::from(3),
            id: token.token_id,
        };
        // No approval yet.
        assert_eq!(handler.handle_erc721_approval(base, approval).unwrap(), 0);
        let _ = handler.store.load_or_initialize_nft(&base, &token);
        assert_eq!(handler.handle_erc721_approval(base, approval).unwrap(), 1);
        let nft = handler.store.load_nft(&token).unwrap().unwrap();
        assert_eq!(
            Address::expect_from(nft.approved.unwrap()),
            approval.approved
        );
        let _ = handler.handle_erc721_approval(
            base,
            Erc721Approval {
                owner: Address::from(2),
                approved: Address::zero(),
                id: token.token_id,
            },
        );

        let nft = handler.store.load_nft(&token).unwrap().unwrap();
        assert_eq!(nft.approved, None);
    }
}

use crate::{models::NftId, store::DataStore};
use anyhow::{Context, Result};
use event_retriever::db_reader::{
    diesel::{BlockRange, EventSource},
    models::*,
};
use shared::eth::Address;

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
                EventMeta::Erc721Transfer(t) => self.handle_erc721_transfer(base, t)?,
                _ => continue,
            };
        }
        Ok(())
    }
    fn handle_erc721_approval(&mut self, base: EventBase, approval: Erc721Approval) -> Result<()> {
        tracing::debug!("Processing {:?} of {:?}", approval, base.contract_address);
        let nft_id = NftId {
            address: base.contract_address,
            token_id: approval.id,
        };
        match self.store.set_approval(&nft_id, approval.approved) {
            Ok(_) => (),
            Err(err) => tracing::warn!("{}", err.to_string()),
        }
        Ok(())
    }

    fn handle_erc721_transfer(&mut self, base: EventBase, transfer: Erc721Transfer) -> Result<()> {
        // Note that these may also include Erc20 Transfers (and we will have to handle that).
        tracing::debug!("Processing {:?} of {:?}", transfer, base.contract_address);
        let (nft_id, _contract, mut nft) = self
            .store
            .load_id_contract_token(&base, transfer.token_id)?;
        // TODO - get Uri, creator, save TxReceipt (at least hash)
        let EventBase {
            block_number,
            transaction_index,
            ..
        } = base;
        let block = block_number.try_into().expect("i64 block");
        let tx_index = transaction_index.try_into().expect("i64 block");

        if transfer.to == Address::zero() {
            // burn token
            nft.burn_block = Some(block);
            nft.burn_tx = Some(tx_index);
        }
        if transfer.from == Address::zero() {
            // Mint: This case is already handled by load_or_initialize
        }
        // TODO - set minter (with tx.from)
        nft.owner = transfer.to.0.as_bytes().to_vec();
        nft.last_transfer_block = Some(block);
        nft.last_transfer_tx = Some(tx_index);
        // TODO - fetch and set json. Maybe in load_or_initialize
        self.store.save_nft(&nft)?;
        // Approvals are unset on transfer.
        // TODO - This could probably also just be set on the field directly.
        // nft.approved = None;
        self.store.clear_approval(&nft_id)?;
        Ok(())
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
    #[ignore]
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
        assert!(handler.handle_erc721_approval(base, approval).is_ok());
        let nft = handler.store.load_or_initialize_nft(&base, &token).unwrap();
        assert_eq!(nft.approved, None); // This also implies the first approval was not set.
        assert!(handler.handle_erc721_approval(base, approval).is_ok());
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

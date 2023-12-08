use crate::handler::EventHandler;
use crate::processor::EventProcessor;
use eth::types::{Address, NftId, TxDetails};
use event_retriever::db_reader::models::{Erc721Transfer, EventBase};

impl EventHandler<Erc721Transfer> for EventProcessor {
    fn handle_event<E>(&mut self, base: EventBase, transfer: Erc721Transfer, tx: &TxDetails) {
        {
            let nft_id = NftId {
                address: base.contract_address,
                token_id: transfer.token_id,
            };

            let mut nft = match self.updates.nfts.remove(&nft_id) {
                Some(nft) => nft,
                None => self.store.load_or_initialize_nft(&base, &nft_id, tx),
            };
            if nft.event_applied(&base) {
                tracing::warn!(
                    "skipping attempt to replay event {:?} at tx {:?} on {:?}",
                    base,
                    tx.hash,
                    nft_id
                );
                // Put the nft back in cache!
                self.updates.nfts.insert(nft_id, nft);
                return;
            }
            let EventBase {
                block_number,
                transaction_index,
                log_index,
                ..
            } = base;
            // TODO - Maybe we should just leave Event Base fields as i64...
            let block = block_number.try_into().expect("i64 block");
            let tx_index = transaction_index.try_into().expect("i64 tx_index");
            let log_index = log_index.try_into().expect("i64 log index");

            if transfer.to == Address::zero() {
                // burn token
                nft.burn_block = Some(block);
                nft.burn_tx = Some(tx_index);
            }
            if transfer.from == Address::zero() {
                // Mint: This case is already handled by load_or_initialize
            }
            nft.owner = transfer.to;
            nft.last_update_block = block;
            nft.last_update_tx = base.transaction_index as i64;
            nft.last_update_log_index = log_index;
            nft.last_transfer_block = Some(block);
            nft.last_transfer_tx = Some(tx_index);
            // TODO - fetch and set json. Maybe in load_or_initialize
            // Approvals are unset on transfer.
            nft.approved = None;
            self.updates.nfts.insert(nft_id, nft);
        }
    }
}

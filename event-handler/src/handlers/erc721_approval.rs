use crate::handler::EventHandler;
use crate::processor::EventProcessor;
use eth::types::{Address, NftId, TxDetails};
use event_retriever::db_reader::models::{Erc721Approval, EventBase};

impl EventHandler<Erc721Approval> for EventProcessor {
    fn handle_event<E>(&mut self, base: EventBase, approval: Erc721Approval, tx: &TxDetails) {
        let nft_id = NftId {
            address: base.contract_address,
            token_id: approval.id,
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
        nft.approved = if approval.approved == Address::zero() {
            None
        } else {
            Some(approval.approved.into())
        };
        nft.last_update_block = base.block_number as i64;
        nft.last_update_tx = base.transaction_index as i64;
        nft.last_update_log_index = base.log_index as i64;
        self.updates.nfts.insert(nft_id, nft);
    }
}

use crate::handler::EventHandler;
use crate::processor::EventProcessor;
use data_store::models::{ApprovalForAll, ApprovalId};
use eth::types::TxDetails;
use event_retriever::db_reader::models::{Erc1155Uri, EventBase};

impl EventHandler<ApprovalForAll> for EventProcessor {
    fn handle_event<E>(&mut self, base: EventBase, event: ApprovalForAll, tx: &TxDetails) {
        let approval_id = ApprovalId {
            contract_address: base.contract_address,
            owner: event.owner,
        };

        let mut approval = match self.updates.approval_for_alls.remove(&approval_id) {
            Some(nft) => nft,
            None => self.store.load_or_initialize_approval(&approval_id),
        };
        if approval.event_applied(&base) {
            tracing::warn!(
                "skipping attempt to replay event {:?} at tx {:?} on {:?}",
                base,
                tx.hash,
                approval_id
            );
            // Put the nft back in cache!
            self.updates.approval_for_alls.insert(approval_id, approval);
            return;
        }
        let EventBase {
            block_number,
            log_index,
            ..
        } = base;
        let block = block_number.try_into().expect("i64 block");
        let log_index = log_index.try_into().expect("i64 log index");

        approval.last_update_block = block;
        approval.last_update_log_index = log_index;

        approval.approved = event.approved;
        approval.operator = event.operator;

        self.updates.approval_for_alls.insert(approval_id, approval);
    }
}

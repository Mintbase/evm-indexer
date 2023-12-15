use crate::handlers::EventHandler;
use crate::processor::EventProcessor;
use data_store::models::ApprovalId;
use eth::types::TxDetails;
use event_retriever::db_reader::models::{ApprovalForAll, EventBase};

impl EventHandler<ApprovalForAll> for EventProcessor {
    fn handle_event(&mut self, base: EventBase, event: ApprovalForAll, tx: &TxDetails) {
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

        approval.last_update_block = base.block_number as i64;
        approval.last_update_log_index = base.log_index as i64;

        approval.approved = event.approved;
        approval.operator = event.operator;

        self.updates.approval_for_alls.insert(approval_id, approval);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::test_util::{setup_data, SetupData};
    use data_store::models::ApprovalForAll as StoreApproval;
    use eth::types::Address;

    #[tokio::test]
    async fn approval_for_all() {
        let SetupData {
            mut handler,
            token_id: _,
            token: _,
            mut base,
            tx,
        } = setup_data();

        let owner = Address::from(3);
        let operator = Address::from(4);
        let approval_id = ApprovalId {
            contract_address: base.contract_address,
            owner,
        };

        handler.handle_event(
            base,
            ApprovalForAll {
                owner,
                operator,
                approved: true,
            },
            &tx,
        );

        assert_eq!(
            handler.updates.approval_for_alls.get(&approval_id).unwrap(),
            &StoreApproval {
                contract_address: base.contract_address,
                owner,
                operator,
                approved: true,
                last_update_block: base.block_number as i64,
                last_update_log_index: base.log_index as i64,
            },
            "true approval"
        );

        // Increment event index (to reuse) and set approval event to false.
        base.block_number += 1;
        handler.handle_event(
            base,
            ApprovalForAll {
                owner,
                operator,
                approved: false,
            },
            &tx,
        );

        assert_eq!(
            handler.updates.approval_for_alls.get(&approval_id).unwrap(),
            &StoreApproval {
                contract_address: base.contract_address,
                owner,
                operator,
                approved: false,
                last_update_block: base.block_number as i64,
                last_update_log_index: base.log_index as i64,
            },
            "false approval"
        );

        // Replay protection -- failed attempt to change to true
        handler.handle_event(
            base,
            ApprovalForAll {
                owner,
                operator,
                approved: true,
            },
            &tx,
        );
        assert_eq!(
            handler.updates.approval_for_alls.get(&approval_id).unwrap(),
            &StoreApproval {
                contract_address: base.contract_address,
                owner,
                operator,
                approved: false,
                last_update_block: base.block_number as i64,
                last_update_log_index: base.log_index as i64,
            },
            "idempotency"
        );
    }
}

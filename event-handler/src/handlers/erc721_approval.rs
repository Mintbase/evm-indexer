use crate::handlers::EventHandler;
use crate::processor::EventProcessor;
use eth::types::{Address, NftId, TxDetails};
use event_retriever::db_reader::models::{Erc721Approval, EventBase};

impl EventHandler<Erc721Approval> for EventProcessor {
    fn handle_event(&mut self, base: EventBase, approval: Erc721Approval, tx: &TxDetails) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::test_util::{setup_data, SetupData};
    use eth::types::Address;

    // These tests shouldn't need to be async, but the handler struct contains async fields.
    #[tokio::test]
    async fn erc721_approval_handler() {
        let SetupData {
            mut handler,
            token_id: _,
            token,
            mut base,
            tx,
        } = setup_data();

        let approved = Address::from(3);
        let first_approval = Erc721Approval {
            owner: Address::from(2),
            approved,
            id: token.token_id,
        };
        // Approval before token existence (handled the way the event said)
        handler.handle_event(base, first_approval, &tx);
        let nft = handler.updates.nfts.get(&token).unwrap();
        assert_eq!(
            Address::from(nft.clone().approved.unwrap()),
            approved,
            "first approval"
        );
        base.block_number += 1; // reuse incremented base.
        handler.handle_event(
            base,
            Erc721Approval {
                owner: Address::from(2),
                approved: Address::zero(),
                id: token.token_id,
            },
            &tx,
        );
        let nft = handler.updates.nfts.get(&token).unwrap();
        assert_eq!(nft.approved, None, "second approval");

        // Idempotency: Try to replay the first approval
        base.block_number -= 1;
        handler.handle_event(base, first_approval, &tx);
        // Approval not applied.
        assert_eq!(
            handler.updates.nfts.get(&token).unwrap().approved,
            None,
            "idempotency"
        );
    }
}

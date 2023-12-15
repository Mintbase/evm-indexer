use crate::handlers::EventHandler;
use crate::processor::EventProcessor;
use eth::types::{Address, NftId, TxDetails};
use event_retriever::db_reader::models::{Erc721Transfer, EventBase};

impl EventHandler<Erc721Transfer> for EventProcessor {
    fn handle_event(&mut self, base: EventBase, transfer: Erc721Transfer, tx: &TxDetails) {
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
            // TODO - Maybe we should just leave Event Base fields as i64...
            //  https://github.com/Mintbase/evm-indexer/issues/103
            let block = base.block_number as i64;
            let tx_index = base.transaction_index as i64;
            let log_index = base.log_index as i64;

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
            nft.approved = None;
            self.updates.nfts.insert(nft_id, nft);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::test_util::{setup_data, SetupData};
    use data_store::models::Nft;
    use eth::types::Address;
    #[tokio::test]
    async fn erc721_transfer_handler() {
        let SetupData {
            mut handler,
            token_id,
            token,
            base,
            tx,
        } = setup_data();
        let from = Address::from(2);
        let to = Address::from(3);
        let first_transfer = Erc721Transfer { from, to, token_id };
        handler.handle_event(base, first_transfer.clone(), &tx);

        assert_eq!(
            handler.updates.nfts.get(&token).unwrap(),
            &Nft {
                contract_address: base.contract_address,
                token_id: token_id.into(),
                token_uri: None,
                owner: to,
                metadata_id: None,
                last_update_block: base.block_number as i64,
                last_update_tx: base.transaction_index as i64,
                last_update_log_index: base.log_index as i64,
                last_transfer_block: Some(base.block_number as i64),
                last_transfer_tx: Some(base.transaction_index as i64),
                mint_block: base.block_number as i64,
                mint_tx: base.transaction_index as i64,
                burn_block: None,
                burn_tx: None,
                minter: tx.from,
                approved: None,
            },
            "first transfer"
        );
        let base_2 = EventBase {
            block_number: 4,
            log_index: 5,
            transaction_index: 6,
            contract_address: base.contract_address,
        };
        // Transfer back
        handler.handle_event(
            base_2,
            Erc721Transfer {
                from: to,
                to: from,
                token_id,
            },
            &tx,
        );

        assert_eq!(
            handler.updates.nfts.get(&token).unwrap(),
            &Nft {
                contract_address: base.contract_address,
                token_id: token_id.into(),
                token_uri: None,
                owner: from,
                metadata_id: None,
                last_update_block: base_2.block_number as i64,
                last_update_tx: base_2.transaction_index as i64,
                last_update_log_index: base_2.log_index as i64,
                last_transfer_block: Some(base_2.block_number as i64),
                last_transfer_tx: Some(base_2.transaction_index as i64),
                mint_block: base.block_number as i64,
                mint_tx: base.transaction_index as i64,
                burn_block: None,
                burn_tx: None,
                minter: tx.from,
                approved: None,
            },
            "transfer back"
        );

        // Burn Token
        let base_3 = EventBase {
            block_number: 7,
            log_index: 8,
            transaction_index: 9,
            contract_address: base.contract_address,
        };
        handler.handle_event(
            base_3,
            Erc721Transfer {
                from,
                to: Address::zero(),
                token_id,
            },
            &tx,
        );
        assert_eq!(
            handler.updates.nfts.get(&token).unwrap(),
            &Nft {
                contract_address: base.contract_address,
                token_id: token_id.into(),
                token_uri: None,
                owner: Address::zero(),
                metadata_id: None,
                last_update_block: base_3.block_number as i64,
                last_update_tx: base_3.transaction_index as i64,
                last_update_log_index: base_3.log_index as i64,
                last_transfer_block: Some(base_3.block_number as i64),
                last_transfer_tx: Some(base_3.transaction_index as i64),
                mint_block: base.block_number as i64,
                mint_tx: base.transaction_index as i64,
                burn_block: Some(base_3.block_number as i64),
                burn_tx: Some(base_3.transaction_index as i64),
                minter: tx.from,
                approved: None,
            },
            "burn transfer"
        );

        // Idempotency: try to replay earlier transfers
        handler.handle_event(base, first_transfer, &tx);
        assert_eq!(
            handler.updates.nfts.get(&token).unwrap().owner,
            Address::zero(),
            "idempotency"
        )
    }
}

use crate::handlers::EventHandler;
use crate::processor::EventProcessor;
use eth::types::TxDetails;
use event_retriever::db_reader::models::{Erc1155Uri, EventBase};

impl EventHandler<Erc1155Uri> for EventProcessor {
    fn handle_event(&mut self, base: EventBase, uri: Erc1155Uri, tx: &TxDetails) {
        let mut token = match self.before_erc1155_event(base, uri.id, tx) {
            Some(token) => token,
            None => return,
        };
        token.token_uri = Some(uri.value);
        self.updates.multi_tokens.insert(token.id(), token);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::test_util::{setup_data, SetupData};
    use data_store::models::Erc1155;

    #[tokio::test]
    async fn erc1155_uri() {
        let SetupData {
            mut handler,
            token_id: id,
            token,
            base,
            tx,
        } = setup_data();
        let value = "https://my-website.com".to_string();

        handler.handle_event(
            base,
            Erc1155Uri {
                id,
                value: value.clone(),
            },
            &tx,
        );

        assert_eq!(
            handler.updates.multi_tokens.get(&token).unwrap(),
            &Erc1155 {
                contract_address: base.contract_address,
                token_id: id.into(),
                token_uri: Some(value.clone()),
                // Note that we did not mint first so this transferred value
                // is not realized in the total supply
                total_supply: 0.into(),
                last_update_block: base.block_number as i64,
                last_update_tx: base.transaction_index as i64,
                last_update_log_index: base.log_index as i64,
                mint_block: base.block_number as i64,
                mint_tx: base.transaction_index as i64,
                creator_address: tx.from,
            },
            "uri update"
        );
        // Replay protection (inherited from token)
        let new_value = "Different website".to_string();
        handler.handle_event(
            base,
            Erc1155Uri {
                id,
                value: new_value.clone(),
            },
            &tx,
        );
        assert_eq!(
            handler.updates.multi_tokens.get(&token).unwrap(),
            &Erc1155 {
                contract_address: base.contract_address,
                token_id: id.into(),
                token_uri: Some(value),
                // Note that we did not mint first so this transferred value
                // is not realized in the total supply
                total_supply: 0.into(),
                last_update_block: base.block_number as i64,
                last_update_tx: base.transaction_index as i64,
                last_update_log_index: base.log_index as i64,
                mint_block: base.block_number as i64,
                mint_tx: base.transaction_index as i64,
                creator_address: tx.from,
            },
            "idempotency"
        );
    }
}

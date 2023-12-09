use crate::handlers::EventHandler;
use crate::processor::EventProcessor;
use eth::types::{Address, TxDetails};
use event_retriever::db_reader::models::{Erc1155TransferSingle, EventBase};

impl EventHandler<Erc1155TransferSingle> for EventProcessor {
    fn handle_event(&mut self, base: EventBase, transfer: Erc1155TransferSingle, tx: &TxDetails) {
        let mut token = match self.before_erc1155_event(base, transfer.id, tx) {
            Some(token) => token,
            None => return,
        };

        let from = transfer.from;
        let to = transfer.to;

        // Supply related updates.
        if to == Address::zero() {
            token.decrease_supply(transfer.value);
        }
        if from == Address::zero() {
            token.increase_supply(transfer.value);
        }

        // Ownership updates
        let contract = base.contract_address;
        if from != Address::zero() {
            let mut sender =
                match self
                    .updates
                    .multi_token_owners
                    .remove(&(token.id(), contract, from))
                {
                    Some(owner) => owner,
                    None => self
                        .store
                        .load_or_initialize_erc1155_owner(&base, &token.id(), from),
                };
            sender.decrease_balance(transfer.value);
            self.updates
                .multi_token_owners
                .insert((token.id(), contract, from), sender);
        }
        let mut recipient =
            match self
                .updates
                .multi_token_owners
                .remove(&(token.id(), contract, to))
            {
                Some(owner) => owner,
                None => self
                    .store
                    .load_or_initialize_erc1155_owner(&base, &token.id(), to),
            };
        recipient.increase_balance(transfer.value);
        self.updates
            .multi_token_owners
            .insert((token.id(), contract, to), recipient);

        self.updates.multi_tokens.insert(token.id(), token);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::test_util::{setup_data, SetupData};
    use data_store::models::{Erc1155, Erc1155Owner};
    use eth::types::{Address, U256};

    #[tokio::test]
    async fn erc1155_transfer_handler() {
        let SetupData {
            mut handler,
            token_id: id,
            token,
            base,
            tx,
        } = setup_data();
        let from = Address::from(2);
        let to = Address::from(3);
        let value = U256::from(456789);
        let first_transfer = Erc1155TransferSingle {
            operator: Default::default(),
            from,
            to,
            id,
            value,
        };
        handler.handle_event(base, first_transfer.clone(), &tx);

        assert_eq!(
            handler.updates.multi_tokens.get(&token).unwrap(),
            &Erc1155 {
                contract_address: base.contract_address,
                token_id: id.into(),
                token_uri: None,
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
            "first transfer"
        );

        assert_eq!(
            handler
                .updates
                .multi_token_owners
                .get(&(token, base.contract_address, to))
                .unwrap(),
            &Erc1155Owner {
                contract_address: base.contract_address,
                token_id: id.into(),
                owner: to,
                balance: value.into(),
            },
            "first transfer recipient balance"
        );

        assert_eq!(
            handler
                .updates
                .multi_token_owners
                .get(&(token, base.contract_address, from))
                .unwrap(),
            &Erc1155Owner {
                contract_address: base.contract_address,
                token_id: id.into(),
                owner: from,
                // Negative balance because they sent before ever receiving!
                balance: (-456789).into(),
            },
            "first transfer sender balance"
        );

        let base_2 = EventBase {
            block_number: 4,
            log_index: 5,
            transaction_index: 6,
            contract_address: base.contract_address,
        };
        // Transfer Balance back
        handler.handle_event(
            base_2,
            Erc1155TransferSingle {
                from: to,
                to: from,
                id,
                operator: Address::zero(),
                value,
            },
            &tx,
        );

        assert_eq!(
            handler.updates.multi_tokens.get(&token).unwrap(),
            &Erc1155 {
                contract_address: base.contract_address,
                token_id: id.into(),
                token_uri: None,
                total_supply: 0.into(),
                last_update_block: base_2.block_number as i64,
                last_update_tx: base_2.transaction_index as i64,
                last_update_log_index: base_2.log_index as i64,
                mint_block: base.block_number as i64,
                mint_tx: base.transaction_index as i64,
                creator_address: tx.from,
            },
            "transfer back"
        );
        assert_eq!(
            handler
                .updates
                .multi_token_owners
                .get(&(token, base.contract_address, to))
                .unwrap(),
            &Erc1155Owner {
                contract_address: base.contract_address,
                token_id: id.into(),
                owner: to,
                balance: 0.into(),
            },
            "second transfer (back) recipient balance"
        );
        assert_eq!(
            handler
                .updates
                .multi_token_owners
                .get(&(token, base.contract_address, from))
                .unwrap(),
            &Erc1155Owner {
                contract_address: base.contract_address,
                token_id: id.into(),
                owner: from,
                // Negative balance because they sent before ever receiving!
                balance: 0.into(),
            },
            "second transfer (back) sender balance"
        );

        // Mint:
        let mint_base = EventBase {
            block_number: 5,
            log_index: 5,
            transaction_index: 6,
            contract_address: base.contract_address,
        };
        let mint_transfer = Erc1155TransferSingle {
            from: Address::zero(),
            to,
            id,
            operator: Address::zero(),
            value,
        };
        handler.handle_event(mint_base, mint_transfer, &tx);

        assert_eq!(
            handler.updates.multi_tokens.get(&token).unwrap(),
            &Erc1155 {
                contract_address: base.contract_address,
                token_id: id.into(),
                token_uri: None,
                total_supply: value.into(),
                last_update_block: mint_base.block_number as i64,
                last_update_tx: mint_base.transaction_index as i64,
                last_update_log_index: mint_base.log_index as i64,
                mint_block: base.block_number as i64,
                mint_tx: base.transaction_index as i64,
                creator_address: tx.from,
            },
            "mint"
        );
        assert_eq!(
            handler
                .updates
                .multi_token_owners
                .get(&(token, base.contract_address, to))
                .unwrap(),
            &Erc1155Owner {
                contract_address: base.contract_address,
                token_id: id.into(),
                owner: to,
                balance: value.into(),
            },
            "mint recipient balance"
        );

        // Idempotency: try to replay mint event
        handler.handle_event(base, first_transfer, &tx);
        assert_eq!(
            handler
                .updates
                .multi_tokens
                .get(&token)
                .unwrap()
                .total_supply,
            value.into(),
            "idempotency"
        );

        // Burn Token
        let base_4 = EventBase {
            block_number: 7,
            log_index: 8,
            transaction_index: 9,
            contract_address: base.contract_address,
        };
        handler.handle_event(
            base_4,
            Erc1155TransferSingle {
                from: to,
                to: Address::zero(),
                id,
                operator: Address::zero(),
                value,
            },
            &tx,
        );
        assert_eq!(
            handler.updates.multi_tokens.get(&token).unwrap(),
            &Erc1155 {
                contract_address: base.contract_address,
                token_id: id.into(),
                token_uri: None,
                total_supply: 0.into(),
                last_update_block: base_4.block_number as i64,
                last_update_tx: base_4.transaction_index as i64,
                last_update_log_index: base_4.log_index as i64,
                mint_block: base.block_number as i64,
                mint_tx: base.transaction_index as i64,
                creator_address: tx.from,
            },
            "burn"
        );
        assert_eq!(
            handler
                .updates
                .multi_token_owners
                .get(&(token, base.contract_address, to))
                .unwrap(),
            &Erc1155Owner {
                contract_address: base.contract_address,
                token_id: id.into(),
                owner: to,
                balance: 0.into(),
            },
            "burner balance"
        );
    }
}

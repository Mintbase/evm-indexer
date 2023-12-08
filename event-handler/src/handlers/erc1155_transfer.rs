use crate::handler::EventHandler;
use crate::processor::EventProcessor;
use data_store::models::Erc1155;
use eth::types::{Address, TxDetails};
use event_retriever::db_reader::models::{Erc1155TransferSingle, EventBase};

impl EventHandler<Erc1155TransferSingle> for EventProcessor {
    fn handle_event<E>(
        &mut self,
        base: EventBase,
        transfer: Erc1155TransferSingle,
        tx: &TxDetails,
    ) {
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

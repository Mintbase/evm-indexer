use crate::handler::EventHandler;
use crate::processor::EventProcessor;
use eth::types::TxDetails;
use event_retriever::db_reader::models::{Erc1155Uri, EventBase};

impl EventHandler<Erc1155Uri> for EventProcessor {
    fn handle_event<E>(&mut self, base: EventBase, uri: Erc1155Uri, tx: &TxDetails) {
        let mut token = match self.before_erc1155_event(base, uri.id, tx) {
            Some(token) => token,
            None => return,
        };
        token.token_uri = Some(uri.value);
        self.updates.multi_tokens.insert(token.id(), token);
    }
}

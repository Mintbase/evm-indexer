use data_store::models::{ApprovalId, Erc1155};
use eth::types::{Address, NftId, TxDetails, U256};
use event_retriever::db_reader::models::{
    ApprovalForAll, Erc1155TransferSingle, Erc1155Uri, Erc721Approval, Erc721Transfer, EventBase,
};

use crate::processor::EventProcessor;

pub trait EventHandler {
    fn handle_erc721_approval(&mut self, base: EventBase, approval: Erc721Approval, tx: &TxDetails);
    fn handle_erc721_transfer(&mut self, base: EventBase, transfer: Erc721Transfer, tx: &TxDetails);
    fn handle_erc1155_transfer(
        &mut self,
        base: EventBase,
        transfer: Erc1155TransferSingle,
        tx: &TxDetails,
    );
    fn handle_erc1155_uri(&mut self, base: EventBase, uri: Erc1155Uri, tx: &TxDetails);
    fn handle_approval_for_all(&mut self, base: EventBase, event: ApprovalForAll, tx: &TxDetails);
    // Maybe Someday we can have something like one of these:
    // fn handle_event<T: EventMeta>(&mut self, base: EventBase, event: T, tx: &TxDetails);
    // fn handle_event<T: NftEvent>(&mut self, event: T, tx: &TxDetails);
}

impl EventHandler for EventProcessor {
    fn handle_erc721_approval(
        &mut self,
        base: EventBase,
        approval: Erc721Approval,
        tx: &TxDetails,
    ) {
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

    fn handle_erc721_transfer(
        &mut self,
        base: EventBase,
        transfer: Erc721Transfer,
        tx: &TxDetails,
    ) {
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

    fn handle_erc1155_transfer(
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

    fn handle_erc1155_uri(&mut self, base: EventBase, uri: Erc1155Uri, tx: &TxDetails) {
        let mut token = match self.before_erc1155_event(base, uri.id, tx) {
            Some(token) => token,
            None => return,
        };
        token.token_uri = Some(uri.value);
        self.updates.multi_tokens.insert(token.id(), token);
    }

    fn handle_approval_for_all(&mut self, base: EventBase, event: ApprovalForAll, tx: &TxDetails) {
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

impl EventProcessor {
    fn before_erc1155_event(
        &mut self,
        base: EventBase,
        id: U256,
        tx: &TxDetails,
    ) -> Option<Erc1155> {
        let nft_id = NftId {
            address: base.contract_address,
            token_id: id,
        };

        let mut token = match self.updates.multi_tokens.remove(&nft_id) {
            Some(nft) => nft,
            None => self.store.load_or_initialize_erc1155(&base, &nft_id, tx),
        };
        if token.event_applied(&base) {
            tracing::warn!(
                "skipping attempt to replay event {:?} at tx {:?} on {:?}",
                base,
                tx.hash,
                nft_id
            );
            // Put the nft back in cache!
            self.updates.multi_tokens.insert(nft_id, token);
            return None;
        }
        let EventBase {
            block_number,
            transaction_index,
            log_index,
            ..
        } = base;
        let block = block_number.try_into().expect("i64 block");
        let tx_index = transaction_index.try_into().expect("i64 tx_index");
        let log_index = log_index.try_into().expect("i64 log index");

        token.last_update_block = block;
        token.last_update_tx = tx_index;
        token.last_update_log_index = log_index;

        Some(token)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{ChainDataSource, HandlerConfig};

    use super::*;
    use data_store::models::{ApprovalForAll as StoreApproval, Erc1155Owner, Nft};
    use eth::types::{Address, Bytes32, NftId, U256};
    use std::str::FromStr;
    static TEST_SOURCE_URL: &str = "postgresql://postgres:postgres@localhost:5432/arak";
    static TEST_STORE_URL: &str = "postgresql://postgres:postgres@localhost:5432/store";
    static TEST_ETH_RPC: &str = "https://rpc.ankr.com/eth";

    fn test_processor() -> EventProcessor {
        EventProcessor::new(
            TEST_SOURCE_URL,
            TEST_STORE_URL,
            TEST_ETH_RPC,
            HandlerConfig {
                chain_data_source: ChainDataSource::Database,
                page_size: 10,
                fetch_metadata: false,
            },
        )
        .unwrap()
    }
    struct SetupData {
        handler: EventProcessor,
        // contract_address: Address,
        token_id: U256,
        token: NftId,
        base: EventBase,
        tx: TxDetails,
    }

    fn setup_data() -> SetupData {
        let handler = test_processor();
        let contract_address = Address::from(1);
        let token_id = U256::from(123);
        let token = NftId {
            address: contract_address,
            token_id,
        };
        let base = EventBase {
            block_number: 1,
            log_index: 2,
            transaction_index: 3,
            contract_address,
        };
        let tx = TxDetails {
            hash: Bytes32::from_str(
                "0xe9e91f1ee4b56c0df2e9f06c2b8c27c6076195a88a7b8537ba8313d80e6f124e",
            )
            .unwrap(),
            from: Address::from_str("0x32be343b94f860124dc4fee278fdcbd38c102d88").unwrap(),
            to: Some(Address::from_str("0xdf190dc7190dfba737d7777a163445b7fff16133").unwrap()),
        };
        SetupData {
            handler,
            token_id,
            token,
            base,
            tx,
        }
    }

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
        handler.handle_erc721_approval(base, first_approval, &tx);
        let nft = handler.updates.nfts.get(&token).unwrap();
        assert_eq!(
            Address::from(nft.clone().approved.unwrap()),
            approved,
            "first approval"
        );
        base.block_number += 1; // reuse incremented base.
        handler.handle_erc721_approval(
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
        handler.handle_erc721_approval(base, first_approval, &tx);
        // Approval not applied.
        assert_eq!(
            handler.updates.nfts.get(&token).unwrap().approved,
            None,
            "idempotency"
        );
    }

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
        handler.handle_erc721_transfer(base, first_transfer.clone(), &tx);

        assert_eq!(
            handler.updates.nfts.get(&token).unwrap(),
            &Nft {
                contract_address: base.contract_address,
                token_id: token_id.into(),
                token_uri: None,
                owner: to,
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
        handler.handle_erc721_transfer(
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
        handler.handle_erc721_transfer(
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
        handler.handle_erc721_transfer(base, first_transfer, &tx);
        assert_eq!(
            handler.updates.nfts.get(&token).unwrap().owner,
            Address::zero(),
            "idempotency"
        )
    }

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
        handler.handle_erc1155_transfer(base, first_transfer.clone(), &tx);

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
        handler.handle_erc1155_transfer(
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
        handler.handle_erc1155_transfer(mint_base, mint_transfer, &tx);

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
        handler.handle_erc1155_transfer(base, first_transfer, &tx);
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
        handler.handle_erc1155_transfer(
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

        handler.handle_erc1155_uri(
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
        handler.handle_erc1155_uri(
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

        handler.handle_approval_for_all(
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
        handler.handle_approval_for_all(
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
        handler.handle_approval_for_all(
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

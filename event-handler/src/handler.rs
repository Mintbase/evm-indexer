use crate::models::Nft;
use crate::{models::NftApproval, store::DataStore};
use anyhow::{Context, Result};
use ethers::types::Address;
use event_retriever::db_reader::diesel::BlockRange;
use event_retriever::db_reader::{diesel::EventSource, models::*};

const ZERO_ADDRESS: Address = Address::zero();

pub struct EventHandler {
    /// source of events processing
    source: EventSource,
    // Location of existing stored content
    store: DataStore,
}

impl EventHandler {
    pub fn new(source_url: &str, store_url: &str) -> Result<Self> {
        Ok(Self {
            source: EventSource::new(source_url).context("init EventSource")?,
            store: DataStore::new(store_url).context("init DataStore")?,
        })
    }
    pub fn process_events_for_block_range(&mut self, range: BlockRange) -> Result<()> {
        let events = self.source.get_events_for_block_range(range)?;
        tracing::debug!("Retrieved {} events for {:?}", events.len(), range);
        for NftEvent { base, meta } in events.into_iter() {
            // TODO - fetch transaction hashes for block.
            //  eth_getTransactionByBlockNumberAndIndex OR
            //  eth_getBlockByNumber (with true flag for hashes)
            match meta {
                EventMeta::ApprovalForAll(a) => Self::handle_approval_for_all(base, a),
                EventMeta::Erc1155TransferBatch(batch) => {
                    for (id, value) in batch.ids.into_iter().zip(batch.values.into_iter()) {
                        self.handle_erc1155_transfer(
                            base,
                            Erc1155TransferSingle {
                                operator: batch.operator,
                                from: batch.from,
                                to: batch.to,
                                id,
                                value,
                            },
                        )
                    }
                }
                EventMeta::Erc1155TransferSingle(t) => self.handle_erc1155_transfer(base, t),
                EventMeta::Erc1155Uri(uri) => self.handle_erc1155_uri(base, uri),
                EventMeta::Erc721Approval(a) => self.handle_erc721_approval(base, a),
                EventMeta::Erc721Transfer(transfer) => self.handle_erc721_transfer(base, transfer),
            };
        }
        Ok(())
    }

    fn handle_approval_for_all(base: EventBase, approval: ApprovalForAll) {
        let ApprovalForAll {
            owner,
            operator,
            approved,
        } = approval;
        let log_word = match approved {
            true => "approved",
            false => "revoked",
        };
        tracing::debug!(
            "{:?} {log_word} {:?} as operator of all their {}",
            owner,
            operator,
            base.contract_address
        )
    }

    fn handle_erc1155_transfer(&mut self, base: EventBase, transfer: Erc1155TransferSingle) {
        // This will be the place where we handle minting, burning, and generic transfer.
        // Some sample code used to process such an event for a subgraph can be found here:
        // https://github.com/verynifty/NFT-subgraph/blob/0c128f0aa126abf7ba19eaac2ad87e98fa9710df/src/mappings-erc-1155.ts#L46-L70
        tracing::debug!("Processing {:?} of {:?}", transfer, base.contract_address);
        let (_nft_id, _contract, nft) = self.store.load_id_contract_token(&base, transfer.id);
        // TODO - get Uri, creator, save TxReceipt (at least hash)
        let nft = self.generic_transfer(&base, nft, transfer.to, transfer.from);
        // TODO - fetch and set json. Maybe in load_or_initialize
        self.store.save_nft(&nft).expect("save Nft");
    }

    fn handle_erc721_approval(&mut self, base: EventBase, approval: Erc721Approval) {
        tracing::debug!("Processing {:?} of {:?}", approval, base.contract_address);
        let _ = self
            .store
            .set_approval(NftApproval::from_event(base.contract_address, approval));
    }

    fn handle_erc721_transfer(&mut self, base: EventBase, transfer: Erc721Transfer) {
        // Note that these may also include Erc20 Transfers (and we will have to handle that).
        tracing::debug!("Processing {:?} of {:?}", transfer, base.contract_address);
        let (nft_id, _contract, nft) = self.store.load_id_contract_token(&base, transfer.token_id);
        // TODO - get Uri, creator, save TxReceipt (at least hash)
        let nft = self.generic_transfer(&base, nft, transfer.to, transfer.from);
        // TODO - fetch and set json. Maybe in load_or_initialize
        self.store.save_nft(&nft).expect("save Nft");
        // Approvals are unset on transfer.
        self.store.clear_approval(nft_id).expect("clear approval");
    }

    fn generic_transfer(
        &mut self,
        base: &EventBase,
        mut nft: Nft,
        to: Address,
        from: Address,
    ) -> Nft {
        let EventBase {
            block_number,
            transaction_index,
            ..
        } = *base;
        let block = block_number.try_into().expect("i64 block");
        let tx_index = transaction_index.try_into().expect("i64 block");

        if to == ZERO_ADDRESS {
            // burn token
            nft.burn_block = Some(block);
            nft.burn_tx = Some(tx_index);
        }
        if from == ZERO_ADDRESS {
            // Mint: This case is already handled by load_or_initialize
        }
        // TODO - set minter (with tx.from)
        nft.owner = to.as_bytes().to_vec();
        nft.last_transfer_block = Some(block);
        nft.last_transfer_tx = Some(tx_index);
        nft
    }

    fn handle_erc1155_uri(&mut self, base: EventBase, uri: Erc1155Uri) {
        tracing::debug!("Processing {:?} of {:?}", uri, base.contract_address);
        let (_nft_id, _contract, mut nft) = self.store.load_id_contract_token(&base, uri.id);

        nft.update_json("uri".to_string(), uri.value);
        self.store.save_nft(&nft).expect("save nft on uri");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event_retriever::db_reader::diesel::BlockRange;

    static TEST_SOURCE_URL: &str = "postgresql://postgres:postgres@localhost:5432/postgres";
    static TEST_STORE_URL: &str = "postgresql://postgres:postgres@localhost:5432/store";
    #[test]
    fn event_processing() {
        let mut handler = EventHandler::new(TEST_SOURCE_URL, TEST_STORE_URL).unwrap();
        assert!(handler
            .process_events_for_block_range(BlockRange {
                start: 10_000_000,
                end: 11_000_000,
            })
            .is_ok());
    }
}

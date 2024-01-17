use data_store::models::ApprovalId;
use data_store::{
    models::{
        ApprovalForAll as StoreApproval, Erc1155, Erc1155Owner, Nft, TokenContract, Transaction,
    },
    store::DataStore,
};
use eth::types::{Address, BlockData, NftId};
use std::collections::{HashMap, HashSet};

#[derive(Default, Debug)]
pub struct UpdateCache {
    pub nfts: HashMap<NftId, Nft>,
    pub multi_tokens: HashMap<NftId, Erc1155>,
    /// (Token, Contract, Owner) -> Ownership
    pub multi_token_owners: HashMap<(NftId, Address, Address), Erc1155Owner>,
    pub approval_for_alls: HashMap<ApprovalId, StoreApproval>,
    pub contracts: HashMap<Address, TokenContract>,
    pub transactions: HashSet<Transaction>,
    pub blocks: HashSet<BlockData>,
}

impl UpdateCache {
    pub fn add_block_tx(&mut self, block: &BlockData, transaction: &Transaction) {
        self.transactions.insert(transaction.clone());
        self.blocks.insert(block.clone());
    }
    /// This method writes its records to the provided DataStore
    /// while relieving itself of its memory.
    pub async fn write(&mut self, db: &mut DataStore) {
        // TODO - It would be ideal if all db actions happened in a single commit
        //  so that failure to write any one of them results in no changes at all.
        //  this can be done with @databases typescript library so it should be possible here.
        //  https://github.com/Mintbase/evm-indexer/issues/106

        // Write and clear transactions
        if !self.transactions.is_empty() {
            db.save_transactions(std::mem::take(&mut self.transactions).into_iter().collect());
        }

        // Write and clear blocks
        if !self.blocks.is_empty() {
            db.save_blocks(std::mem::take(&mut self.blocks).into_iter().collect());
        }

        // Write and clear contracts
        if !self.contracts.is_empty() {
            db.save_contracts(std::mem::take(
                &mut self.contracts.drain().map(|(_, v)| v).collect(),
            ));
        }

        // Write and clear nfts
        if !self.nfts.is_empty() {
            db.save_nfts(std::mem::take(
                &mut self.nfts.drain().map(|(_, v)| v).collect(),
            ))
            .await;
        }

        // Write and clear erc1155s
        if !self.multi_tokens.is_empty() {
            db.save_erc1155s(std::mem::take(
                &mut self.multi_tokens.drain().map(|(_, v)| v).collect(),
            ))
            .await;
        }

        // Write and clear erc1155_owners
        if !self.multi_token_owners.is_empty() {
            db.save_erc1155_owners(std::mem::take(
                &mut self.multi_token_owners.drain().map(|(_, v)| v).collect(),
            ))
            .await;
        }

        // Write and clear approval_for_alls
        if !self.approval_for_alls.is_empty() {
            db.save_approval_for_alls(std::mem::take(
                &mut self.approval_for_alls.drain().map(|(_, v)| v).collect(),
            ))
            .await;
        }
    }
}

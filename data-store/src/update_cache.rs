use crate::models::{
    ApprovalForAll as StoreApproval, ApprovalId, Erc1155, Erc1155Owner, Nft, TokenContract,
    Transaction,
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

    pub fn is_empty(&self) -> bool {
        self.nfts.is_empty()
            && self.multi_tokens.is_empty()
            && self.multi_token_owners.is_empty()
            && self.approval_for_alls.is_empty()
            && self.contracts.is_empty()
            && self.transactions.is_empty()
            && self.blocks.is_empty()
    }
}

use crate::models::{
    ApprovalForAll as StoreApproval, ApprovalId, Erc1155, Erc1155Owner, Nft, TokenContract,
    Transaction,
};
use eth::types::{Address, BlockData, Message, NftId};
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

    pub fn build_messages(&self) -> Vec<Message> {
        // TODO - we need to be careful how and when we decide to try and fetch stuff.
        //  1. For tokens it could be when metadata_id is null and token_uri is not null.
        //  2. For contracts when abi_id is null.
        //  However, in both cases, we want to avoid retrying the same contract too many times.
        let erc721s: Vec<_> = self
            .nfts
            .iter()
            .filter(|(_, token)| token.metadata_id.is_none() && token.token_uri.is_some())
            .map(|(id, token)| Message::Token {
                address: id.address,
                token_id: id.token_id,
                token_uri: token.token_uri.clone(),
            })
            .collect();

        let erc1155s: Vec<_> = self
            .multi_tokens
            .iter()
            .filter(|(_, token)| token.metadata_id.is_none() && token.token_uri.is_some())
            .map(|(id, token)| Message::Token {
                address: id.address,
                token_id: id.token_id,
                token_uri: token.token_uri.clone(),
            })
            .collect();

        let contracts: Vec<_> = self
            .contracts
            .iter()
            .filter(|(_, contract)| contract.abi_id.is_none())
            .map(|(address, _)| Message::Contract { address: *address })
            .collect();

        erc721s
            .into_iter()
            .chain(erc1155s)
            .chain(contracts)
            .collect()
    }
}

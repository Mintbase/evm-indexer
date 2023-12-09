use data_store::models::ApprovalId;
use data_store::{
    models::{
        ApprovalForAll as StoreApproval, Erc1155, Erc1155Owner, Nft, TokenContract, Transaction,
    },
    store::DataStore,
};
use eth::types::{Address, BlockData, NftId};
use std::collections::HashMap;

#[derive(Default, Debug, PartialEq)]
pub struct UpdateCache {
    pub nfts: HashMap<NftId, Nft>,
    pub multi_tokens: HashMap<NftId, Erc1155>,
    /// (Token, Contract, Owner) -> Ownership
    pub multi_token_owners: HashMap<(NftId, Address, Address), Erc1155Owner>,
    pub approval_for_alls: HashMap<ApprovalId, StoreApproval>,
    pub contracts: HashMap<Address, TokenContract>,
    pub transactions: Vec<Transaction>,
    pub blocks: Vec<BlockData>,
}

impl UpdateCache {
    /// This method writes its records to the provided DataStore
    /// while relieving itself of its memory.
    pub async fn write(&mut self, db: &mut DataStore) {
        // TODO - It would be ideal if all db actions happened in a single commit
        //  so that failure to write any one of them results in no changes at all.
        //  this can be done with @databases typescript library so it should be possible here.

        // Write and clear transactions
        db.save_transactions(std::mem::take(&mut self.transactions));

        // Write and clear blocks
        db.save_blocks(std::mem::take(&mut self.blocks));

        // Write and clear contracts
        db.save_contracts(std::mem::take(
            &mut self.contracts.drain().map(|(_, v)| v).collect(),
        ));

        // Write and clear nfts
        db.save_nfts(std::mem::take(
            &mut self.nfts.drain().map(|(_, v)| v).collect(),
        ))
        .await;

        // Write and clear erc1155s
        db.save_erc1155s(std::mem::take(
            &mut self.multi_tokens.drain().map(|(_, v)| v).collect(),
        ))
        .await;

        // Write and clear erc1155_owners
        db.save_erc1155_owners(std::mem::take(
            &mut self.multi_token_owners.drain().map(|(_, v)| v).collect(),
        ))
        .await;

        // Write and clear approval_for_alls
        db.save_approval_for_alls(std::mem::take(
            &mut self.approval_for_alls.drain().map(|(_, v)| v).collect(),
        ))
        .await;
    }
}

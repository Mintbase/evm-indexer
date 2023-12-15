use crate::schema::*;
use bigdecimal::{BigDecimal, Zero};
use diesel::internal::derives::multiconnection::chrono::NaiveDateTime;
use diesel::{AsChangeset, Insertable, Queryable, Selectable};
use eth::types::{Address, BlockData, Bytes32, NftId, TxDetails, U256};
use event_retriever::db_reader::models::{ApprovalForAll as ApprovalEvent, EventBase};
use keccak_hash::keccak;
use serde::Serialize;
use serde_json::Value;

#[derive(Queryable, Selectable, Insertable, AsChangeset, Debug, Clone, PartialEq, Eq)]
#[diesel(table_name = approval_for_all)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ApprovalForAll {
    #[diesel(serialize_as = Vec<u8>)]
    pub contract_address: Address,
    #[diesel(serialize_as = Vec<u8>)]
    pub owner: Address,
    #[diesel(serialize_as = Vec<u8>)]
    pub operator: Address,
    pub approved: bool,
    pub last_update_block: i64,
    pub last_update_log_index: i64,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ApprovalId {
    pub contract_address: Address,
    pub owner: Address,
}

impl ApprovalForAll {
    pub fn new(base: &EventBase, event: ApprovalEvent) -> Self {
        Self {
            contract_address: base.contract_address,
            owner: event.owner,
            operator: event.operator,
            approved: event.approved,
            last_update_block: 0,
            last_update_log_index: 0,
        }
    }

    pub fn id(&self) -> ApprovalId {
        ApprovalId {
            contract_address: self.contract_address,
            owner: self.owner,
        }
    }

    pub fn event_applied(&self, base: &EventBase) -> bool {
        (base.block_number as i64, base.log_index as i64)
            <= (self.last_update_block, self.last_update_log_index)
    }
}

#[derive(Queryable, Selectable, Insertable, Serialize, Debug, Clone, PartialEq)]
#[diesel(table_name = contract_abis)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ContractAbi {
    #[diesel(serialize_as = Vec<u8>)]
    pub address: Address,
    pub abi: Option<Value>,
}

#[derive(Queryable, Selectable, Insertable, Serialize, Debug, Clone, PartialEq)]
#[diesel(table_name = nft_metadata)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NftMetadata {
    #[diesel(serialize_as = Vec<u8>)]
    pub uid: Bytes32,
    pub json: Value,
}

/// Evalutes the keccak hash of serde_json::Value
fn doc_hash(value: &Value) -> Bytes32 {
    Bytes32::from(keccak(value.to_string().as_bytes()).0)
}

impl NftMetadata {
    pub fn from(content: Value) -> Self {
        Self {
            uid: doc_hash(&content),
            json: content,
        }
    }
}

#[derive(Queryable, Selectable, Insertable, AsChangeset, Debug, PartialEq, Clone, Serialize)]
#[diesel(table_name = nfts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Nft {
    #[diesel(serialize_as = Vec<u8>)]
    pub contract_address: Address,
    pub token_id: BigDecimal,
    pub token_uri: Option<String>,
    #[diesel(serialize_as = Vec<u8>)]
    pub owner: Address,
    /// The keccak hash of the raw document.
    pub metadata_id: Option<Vec<u8>>,
    pub last_update_block: i64,
    pub last_update_tx: i64,
    pub last_update_log_index: i64,
    pub last_transfer_block: Option<i64>,
    pub last_transfer_tx: Option<i64>,
    pub mint_block: i64,
    pub mint_tx: i64,
    pub burn_block: Option<i64>,
    pub burn_tx: Option<i64>,
    #[diesel(serialize_as = Vec<u8>)]
    pub minter: Address,
    pub approved: Option<Vec<u8>>,
    // TODO - add content category / flag here.
    //  https://github.com/Mintbase/evm-indexer/issues/23
}

impl Nft {
    pub fn new(base: &EventBase, nft_id: &NftId, tx: &TxDetails) -> Self {
        Self {
            contract_address: nft_id.address,
            token_id: nft_id.token_id.into(),
            token_uri: None,
            owner: Address::zero(),
            metadata_id: None,
            last_update_block: 0,
            last_update_tx: 0,
            last_update_log_index: 0,
            last_transfer_block: None,
            last_transfer_tx: None,
            // Maybe its best if we set this when transfer comes from Zero.
            mint_block: base.block_number as i64,
            mint_tx: base.transaction_index as i64,
            burn_block: None,
            burn_tx: None,
            minter: tx.from,
            approved: None,
        }
    }

    pub fn id(&self) -> String {
        format!("{:?}/{}", self.contract_address, self.token_id)
    }

    pub fn event_applied(&self, base: &EventBase) -> bool {
        (base.block_number as i64, base.log_index as i64)
            <= (self.last_update_block, self.last_update_log_index)
    }
}

#[derive(Queryable, Selectable, Insertable, AsChangeset, Debug, PartialEq, Clone, Serialize)]
#[diesel(table_name = erc1155s)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Erc1155 {
    #[diesel(serialize_as = Vec<u8>)]
    pub contract_address: Address,
    pub token_id: BigDecimal,
    pub token_uri: Option<String>,
    /// Sum of over owners of all balances.
    pub total_supply: BigDecimal,
    /// Address of first minter.
    #[diesel(serialize_as = Vec<u8>)]
    pub creator_address: Address,
    /// The keccak hash of the raw document.
    pub metadata_id: Option<Vec<u8>>,
    /// Block when token was first minted (i.e. transfer from zero).
    pub mint_block: i64,
    /// Transaction index of first mint.
    pub mint_tx: i64,
    /// record keeping fields.
    pub last_update_block: i64,
    pub last_update_tx: i64,
    pub last_update_log_index: i64,
}

impl Erc1155 {
    pub fn new(base: &EventBase, nft_id: &NftId, tx: &TxDetails) -> Self {
        Self {
            contract_address: nft_id.address,
            token_id: nft_id.token_id.into(),
            token_uri: None,
            total_supply: BigDecimal::zero(),
            metadata_id: None,
            last_update_block: 0,
            last_update_tx: 0,
            last_update_log_index: 0,
            mint_block: base.block_number as i64,
            mint_tx: base.transaction_index as i64,
            creator_address: tx.from,
        }
    }

    pub fn id(&self) -> NftId {
        NftId {
            address: self.contract_address,
            token_id: self.token_id.clone().into(),
        }
    }
    pub fn event_applied(&self, base: &EventBase) -> bool {
        (base.block_number as i64, base.log_index as i64)
            <= (self.last_update_block, self.last_update_log_index)
    }

    pub fn increase_supply(&mut self, amount: U256) {
        self.total_supply += BigDecimal::from(amount);
    }

    pub fn decrease_supply(&mut self, amount: U256) {
        self.total_supply -= BigDecimal::from(amount);
    }
}

#[derive(Queryable, Selectable, Insertable, AsChangeset, Debug, PartialEq, Clone)]
#[diesel(table_name = erc1155_owners)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Erc1155Owner {
    #[diesel(serialize_as = Vec<u8>)]
    pub contract_address: Address,
    pub token_id: BigDecimal,
    #[diesel(serialize_as = Vec<u8>)]
    pub owner: Address,
    pub balance: BigDecimal,
}

impl Erc1155Owner {
    pub fn increase_balance(&mut self, amount: U256) {
        self.balance += BigDecimal::from(amount);
    }

    pub fn decrease_balance(&mut self, amount: U256) {
        self.balance -= BigDecimal::from(amount);
    }
}

#[derive(Queryable, Selectable, Insertable, AsChangeset, PartialEq, Debug, Clone)]
#[diesel(table_name = token_contracts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TokenContract {
    #[diesel(serialize_as = Vec<u8>)]
    pub address: Address,
    // token_type: TokenType,
    pub name: Option<String>,
    pub symbol: Option<String>,
    created_block: i64,
    created_tx_index: i64,
    /// This is generally non-null for Erc1155s.
    base_uri: Option<String>,
    // content_flags -> Nullable<Array<Nullable<ContentFlag>>>,
    // content_category -> Nullable<Array<Nullable<ContentCategory>>>
}

impl TokenContract {
    pub fn from_event_base(event: &EventBase) -> Self {
        Self {
            address: event.contract_address,
            // These are populated externally and asynchronously.
            name: None,
            symbol: None,
            // assume that the first time a contract is seen is the created block
            created_block: event.block_number as i64,
            created_tx_index: event.transaction_index as i64,
            // TODO - try-fetch from Node or Remove
            //  (remove) https://github.com/Mintbase/evm-indexer/issues/101
            //  (try-fetch) https://github.com/Mintbase/evm-indexer/issues/26
            base_uri: None,
        }
    }
}

#[derive(Queryable, Selectable, Insertable, AsChangeset, Clone, Debug, PartialEq)]
#[diesel(table_name = transactions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Transaction {
    block_number: i64,
    index: i64,
    #[diesel(serialize_as = Vec<u8>)]
    hash: Bytes32,
    #[diesel(serialize_as = Vec<u8>)]
    from: Address,
    to: Option<Vec<u8>>,
}

impl Transaction {
    pub fn new(block: u64, index: u64, details: TxDetails) -> Self {
        Self {
            block_number: block as i64,
            index: index as i64,
            hash: details.hash,
            from: details.from,
            to: details.to.map(Address::into),
        }
    }
}

#[derive(Queryable, Selectable, Insertable, AsChangeset, Clone)]
#[diesel(table_name = blocks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Block {
    number: i64,
    time: NaiveDateTime,
}

impl Block {
    pub fn new(block: &BlockData) -> Self {
        Self {
            number: block.number as i64,
            time: block.db_time(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use eth::types::{Bytes32, U256};

    #[test]
    fn token_contract_impls() {
        let contract_address = Address::from(1);
        let base = EventBase {
            block_number: 1,
            log_index: 2,
            transaction_index: 3,
            contract_address,
        };

        assert_eq!(
            TokenContract::from_event_base(&base),
            TokenContract {
                address: base.contract_address,
                name: None,
                symbol: None,
                created_block: base.block_number.try_into().unwrap(),
                created_tx_index: base.transaction_index.try_into().unwrap(),
                base_uri: None,
            }
        )
    }

    #[test]
    fn nft_impls() {
        let contract_address = Address::from(1);
        let base = EventBase {
            block_number: 1,
            log_index: 2,
            transaction_index: 3,
            contract_address,
        };
        let nft_id = NftId {
            address: contract_address,
            token_id: U256::from(123),
        };
        let from = Address::from(1);
        let tx = TxDetails {
            hash: Bytes32::from(1),
            from,
            to: Some(Address::from(2)),
        };
        let nft = Nft::new(&base, &nft_id, &tx);
        assert_eq!(
            nft,
            Nft {
                contract_address: nft_id.address,
                token_id: nft_id.token_id.into(),
                token_uri: None,
                owner: Address::zero(),
                metadata_id: None,
                last_update_block: 0,
                last_update_tx: 0,
                last_update_log_index: 0,
                last_transfer_block: None,
                last_transfer_tx: None,
                // Maybe its best if we set this when transfer comes from Zero.
                mint_block: base.block_number.try_into().unwrap(),
                mint_tx: base.transaction_index.try_into().unwrap(),
                burn_block: None,
                burn_tx: None,
                minter: from,
                approved: None,
            }
        );

        assert!(nft.event_applied(&EventBase {
            block_number: 0,
            log_index: 0,
            transaction_index: 0,
            contract_address
        }));

        assert!(!nft.event_applied(&EventBase {
            block_number: 1,
            log_index: 0,
            transaction_index: 0,
            contract_address
        }));

        assert!(!nft.event_applied(&EventBase {
            block_number: 0,
            log_index: 1,
            transaction_index: 0,
            contract_address
        }));
    }

    #[test]
    fn document_hash() {
        let document = serde_json::json!("My JSON document!");
        assert_eq!(
            doc_hash(&document),
            Bytes32::from_str("0x657fdc1e2d28600a3951bb0b06f4a9672311ca0a4faffae1f2b5904d8b38f12f")
                .unwrap()
        )
    }
}

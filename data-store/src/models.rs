use crate::schema::*;
use bigdecimal::{BigDecimal, Zero};
use diesel::internal::derives::multiconnection::chrono::NaiveDateTime;
use diesel::{AsChangeset, Insertable, Queryable, Selectable};
use eth::types::{Address, BlockData, Bytes32, NftId, TxDetails, U256};
use event_retriever::db_reader::models::{ApprovalForAll as ApprovalEvent, EventBase};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;

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

struct JsonDoc {
    hash: Vec<u8>,
    value: Value,
}

impl JsonDoc {
    fn new(value: Value) -> Self {
        let content_string = value.to_string().replace('\0', "");
        let stripped_content: Value =
            serde_json::from_str::<Value>(&content_string).expect("was Value before");
        Self {
            hash: doc_hash(&stripped_content),
            value: stripped_content,
        }
    }
}

#[derive(Queryable, Selectable, Insertable, Serialize, Debug, Clone, PartialEq)]
#[diesel(table_name = contract_abis)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ContractAbi {
    pub uid: Vec<u8>,
    pub abi: Value,
}

impl ContractAbi {
    pub fn from(content: Value) -> Self {
        let json = JsonDoc::new(content);
        Self {
            uid: json.hash,
            abi: json.value,
        }
    }
}

#[derive(Queryable, Selectable, Insertable, Serialize, Debug, Clone, PartialEq)]
#[diesel(table_name = nft_metadata)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NftMetadata {
    pub uid: Vec<u8>,
    pub raw: Option<String>,
    pub json: Option<Value>,
}

/// Evaluates the md5 hash of serde_json::Value
/// Used for JSON documents like NFTMetadata & ContractABI.
fn doc_hash(value: &Value) -> Vec<u8> {
    md5::compute(value.to_string().as_bytes()).0.to_vec()
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
    /// The md5-hash of the raw document (if available)
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

    pub fn is_fetch_worthy(&self, avoid_list: &HashSet<Address>, retry_blocks: &i64) -> bool {
        let filter_criteria = [
            self.token_uri.is_none(),
            self.last_update_block - self.mint_block < *retry_blocks,
            !avoid_list.contains(&self.contract_address),
        ];
        filter_criteria.iter().all(|x| *x)
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
    /// The md5-hash of the raw document (if available).
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
    /// The md5-hash of the raw document (if available).
    pub abi_id: Option<Vec<u8>>, // content_flags -> Nullable<Array<Nullable<ContentFlag>>>,
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
            abi_id: None,
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

impl Eq for Transaction {}

// Implement Hash based solely on the 'number' field
impl std::hash::Hash for Transaction {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl Transaction {
    pub fn new(block: u64, index: u64, details: &TxDetails) -> Self {
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
                abi_id: None
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
            vec![124, 98, 30, 175, 161, 39, 233, 146, 42, 191, 65, 112, 29, 74, 42, 204]
        )
    }

    #[test]
    fn fetch_worthy_token_filter() {
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

        let mut avoid_list = HashSet::new();
        assert!(nft.token_uri.is_none());
        assert_eq!(nft.last_update_block - nft.last_update_block, 0);

        // Starts as fetch-worthy with empty avoid list.
        assert!(nft.is_fetch_worthy(&avoid_list, &1));

        // Now avoid fetching for this token's contract address.
        avoid_list.insert(contract_address);
        assert!(!nft.is_fetch_worthy(&avoid_list, &1));
    }
}

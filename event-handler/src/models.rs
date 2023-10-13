use crate::schema::*;
use bigdecimal::BigDecimal;
use diesel::internal::derives::multiconnection::chrono::NaiveDateTime;
use diesel::{AsChangeset, Insertable, Queryable, Selectable};
use event_retriever::db_reader::models::EventBase;
use serde_json::Value;
use shared::eth::{Address, U256};

#[derive(Debug)]
pub struct NftId {
    pub address: Address,
    pub token_id: U256,
}

impl NftId {
    pub fn db_address(&self) -> Vec<u8> {
        self.address.into()
    }

    pub fn db_id(&self) -> BigDecimal {
        self.token_id.into()
    }
}

#[derive(Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = approval_for_all)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ApprovalForAll {
    contract_address: Vec<u8>,
    owner: Vec<u8>,
    operator: Vec<u8>,
    approved: bool,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = contract_abis)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct ContractAbi {
    address: Vec<u8>,
    abi: Option<Value>,
}

#[derive(Queryable, Selectable, Insertable, AsChangeset, Debug, PartialEq)]
#[diesel(table_name = nfts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Nft {
    contract_address: Vec<u8>,
    token_id: BigDecimal,
    owner: Vec<u8>,
    last_transfer_block: Option<i64>,
    last_transfer_tx: Option<i64>,
    mint_block: i64,
    mint_tx: i64,
    burn_block: Option<i64>,
    burn_tx: Option<i64>,
    minter: Vec<u8>,
    approved: Option<Vec<u8>>,
    json: Option<Value>,
    // TODO - add content category / flag here.
}

impl Nft {
    pub fn build_from(base: &EventBase, nft_id: &NftId) -> Self {
        Self {
            contract_address: nft_id.address.into(),
            token_id: nft_id.token_id.into(),
            owner: vec![],
            last_transfer_block: None,
            last_transfer_tx: None,
            // Maybe its best if we set this when transfer comes from Zero.
            mint_block: base.block_number.try_into().expect("i64 block_number"),
            mint_tx: base
                .transaction_index
                .try_into()
                .expect("i64 transaction_index"),
            burn_block: None,
            burn_tx: None,
            // TODO - Use tx.from here
            minter: vec![],
            approved: None,
            json: None,
        }
    }
}

#[derive(Queryable, Selectable, Insertable, AsChangeset, PartialEq, Debug)]
#[diesel(table_name = token_contracts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TokenContract {
    pub address: Vec<u8>,
    // token_type: TokenType,
    name: Option<String>,
    symbol: Option<String>,
    decimals: Option<i16>,
    token_uri: Option<String>,
    created_block: i64,
    created_tx_index: i64,
    // content_flags -> Nullable<Array<Nullable<ContentFlag>>>,
    // content_category -> Nullable<Array<Nullable<ContentCategory>>>
}

impl TokenContract {
    pub fn from_event_base(event: &EventBase) -> Self {
        Self {
            address: event.contract_address.into(),
            // TODO - find these an put them.
            name: None,
            symbol: None,
            decimals: None,
            // TODO - this should be base_url
            token_uri: None,
            // assume that the first time a contract is seen is the created block
            created_block: event.block_number.try_into().expect("u64 conversion"),
            created_tx_index: event.transaction_index.try_into().expect("u64 conversion"),
        }
    }
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = transactions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct Transaction {
    block_number: i64,
    index: i64,
    hash: Vec<u8>,
    block_time: NaiveDateTime,
}

#[cfg(test)]
mod tests {
    use super::*;

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
                address: base.contract_address.into(),
                name: None,
                symbol: None,
                decimals: None,
                token_uri: None,
                created_block: base.block_number.try_into().unwrap(),
                created_tx_index: base.transaction_index.try_into().unwrap(),
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

        assert_eq!(
            Nft::build_from(&base, &nft_id),
            Nft {
                contract_address: nft_id.address.into(),
                token_id: nft_id.token_id.into(),
                owner: vec![],
                last_transfer_block: None,
                last_transfer_tx: None,
                // Maybe its best if we set this when transfer comes from Zero.
                mint_block: base.block_number.try_into().unwrap(),
                mint_tx: base.transaction_index.try_into().unwrap(),
                burn_block: None,
                burn_tx: None,
                minter: vec![],
                approved: None,
                json: None,
            }
        )
    }
}

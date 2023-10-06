// @generated automatically by Diesel CLI.

pub mod sql_types {

    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "content_category"))]
    pub struct ContentCategory;

    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "content_flag"))]
    pub struct ContentFlag;

    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "token_type"))]
    pub struct TokenType;
}

diesel::table! {
    approval_for_all (contract_address, owner) {
        contract_address -> Bytea,
        owner -> Bytea,
        operator -> Bytea,
        approved -> Bool,
    }
}

diesel::table! {
    contract_abis (address) {
        address -> Bytea,
        abi -> Nullable<Jsonb>,
    }
}

diesel::table! {
    nft_approvals (contract_address, token_id) {
        contract_address -> Bytea,
        token_id -> Numeric,
        approved -> Bytea,
    }
}

diesel::table! {
    nfts (contract_address, token_id) {
        contract_address -> Bytea,
        token_id -> Numeric,
        owner -> Bytea,
        last_transfer_block -> Nullable<Int8>,
        last_transfer_tx -> Nullable<Int8>,
        mint_block -> Nullable<Int8>,
        mint_tx -> Nullable<Int8>,
        burn_block -> Nullable<Int8>,
        burn_tx -> Nullable<Int8>,
        minter -> Nullable<Bytea>,
        json -> Nullable<Jsonb>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::TokenType;
    use super::sql_types::ContentFlag;
    use super::sql_types::ContentCategory;

    token_contracts (address) {
        address -> Bytea,
        // token_type -> TokenType,
        name -> Nullable<Text>,
        symbol -> Nullable<Text>,
        decimals -> Nullable<Int2>,
        token_uri -> Nullable<Text>,
        created_block -> Int8,
        created_tx_index -> Int8,
        // content_flags -> Nullable<Array<Nullable<ContentFlag>>>,
        // content_category -> Nullable<Array<Nullable<ContentCategory>>>,
    }
}

diesel::table! {
    transactions (block_number, index) {
        block_number -> Int8,
        index -> Int8,
        hash -> Bytea,
        block_time -> Timestamp,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    approval_for_all,
    contract_abis,
    nft_approvals,
    nfts,
    token_contracts,
    transactions,
);

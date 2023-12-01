// @generated automatically by Diesel CLI.

// pub mod sql_types {
//
//     #[derive(diesel::sql_types::SqlType)]
//     #[diesel(postgres_type(name = "content_category"))]
//     pub struct ContentCategory;
//
//     #[derive(diesel::sql_types::SqlType)]
//     #[diesel(postgres_type(name = "content_flag"))]
//     pub struct ContentFlag;
//
//     #[derive(diesel::sql_types::SqlType)]
//     #[diesel(postgres_type(name = "token_type"))]
//     pub struct TokenType;
// }

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
    nfts (contract_address, token_id) {
        contract_address -> Bytea,
        token_id -> Numeric,
        token_uri -> Nullable<Text>,
        owner -> Bytea,
        last_update_block -> Int8,
        last_update_tx -> Int8,
        last_update_log_index -> Int8,
        last_transfer_block -> Nullable<Int8>,
        last_transfer_tx -> Nullable<Int8>,
        mint_block -> Int8,
        mint_tx -> Int8,
        burn_block -> Nullable<Int8>,
        burn_tx -> Nullable<Int8>,
        minter -> Bytea,
        approved -> Nullable<Bytea>,
    }
}

diesel::table! {
    // use diesel::sql_types::*;
    // use super::sql_types::TokenType;
    // use super::sql_types::ContentFlag;
    // use super::sql_types::ContentCategory;

    token_contracts (address) {
        address -> Bytea,
        // token_type -> TokenType,
        name -> Nullable<Text>,
        symbol -> Nullable<Text>,
        created_block -> Int8,
        created_tx_index -> Int8,
        base_uri -> Nullable<Text>,
        // content_flags -> Nullable<Array<Nullable<ContentFlag>>>,
        // content_category -> Nullable<Array<Nullable<ContentCategory>>>,
    }
}

diesel::table! {
    transactions (block_number, index) {
        block_number -> Int8,
        index -> Int8,
        hash -> Bytea,
        from -> Bytea,
        to -> Nullable<Bytea>,
    }
}

diesel::table! {
    blocks (number) {
        number -> Int8,
        time -> Timestamp,
    }
}

diesel::table! {
    erc1155s (contract_address, token_id) {
        contract_address -> Bytea,
        token_id -> Numeric,
        total_supply -> Numeric,
        creator_address -> Bytea,
        token_uri -> Nullable<Text>,
        mint_block -> BigInt,
        mint_tx -> BigInt,
    }
}

diesel::table! {
    erc1155_owners (contract_address, token_id, owner) {
        contract_address -> Bytea,
        token_id -> Numeric,
        owner -> Bytea,
        balance -> Numeric,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    approval_for_all,
    erc1155s,
    erc1155_owners,
    contract_abis,
    nfts,
    token_contracts,
    transactions,
    blocks
);

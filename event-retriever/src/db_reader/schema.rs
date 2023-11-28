// @generated automatically by Diesel CLI.

diesel::table! {
    _event_block (event) {
        event -> Text,
        indexed -> Int8,
        finalized -> Int8,
    }
}

diesel::table! {
    approval_for_all (block_number, log_index) {
        block_number -> Int8,
        log_index -> Int8,
        transaction_index -> Int8,
        address -> Bytea,
        owner_0 -> Bytea,
        operator_1 -> Bytea,
        approved_2 -> Bool,
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
    erc1155_transfer_batch (block_number, log_index) {
        block_number -> Int8,
        log_index -> Int8,
        transaction_index -> Int8,
        address -> Bytea,
        operator_0 -> Bytea,
        from_1 -> Bytea,
        to_2 -> Bytea,
    }
}

diesel::table! {
    erc1155_transfer_batch_ids_0 (block_number, log_index, array_index) {
        block_number -> Int8,
        log_index -> Int8,
        transaction_index -> Int8,
        address -> Bytea,
        array_index -> Int8,
        ids_0 -> Numeric,
    }
}

diesel::table! {
    erc1155_transfer_batch_values_1 (block_number, log_index, array_index) {
        block_number -> Int8,
        log_index -> Int8,
        transaction_index -> Int8,
        address -> Bytea,
        array_index -> Int8,
        values_0 -> Numeric,
    }
}

diesel::table! {
    erc1155_transfer_single (block_number, log_index) {
        block_number -> Int8,
        log_index -> Int8,
        transaction_index -> Int8,
        address -> Bytea,
        operator_0 -> Bytea,
        from_1 -> Bytea,
        to_2 -> Bytea,
        id_3 -> Numeric,
        value_4 -> Numeric,
    }
}

diesel::table! {
    erc1155_uri (block_number, log_index) {
        block_number -> Int8,
        log_index -> Int8,
        transaction_index -> Int8,
        address -> Bytea,
        value_0 -> Text,
        id_1 -> Numeric,
    }
}

diesel::table! {
    erc721_approval (block_number, log_index) {
        block_number -> Int8,
        log_index -> Int8,
        transaction_index -> Int8,
        address -> Bytea,
        owner_0 -> Bytea,
        approved_1 -> Bytea,
        tokenid_2 -> Numeric,
    }
}

diesel::table! {
    erc721_transfer (block_number, log_index) {
        block_number -> Int8,
        log_index -> Int8,
        transaction_index -> Int8,
        address -> Bytea,
        from_0 -> Bytea,
        to_1 -> Bytea,
        tokenid_2 -> Numeric,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    _event_block,
    approval_for_all,
    erc1155_transfer_batch,
    erc1155_transfer_batch_ids_0,
    erc1155_transfer_batch_values_1,
    erc1155_transfer_single,
    erc1155_uri,
    erc721_approval,
    erc721_transfer,
);

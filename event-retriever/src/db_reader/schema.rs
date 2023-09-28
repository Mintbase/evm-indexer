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
        owner_0 -> Nullable<Bytea>,
        operator_1 -> Nullable<Bytea>,
        approved_2 -> Nullable<Bool>,
    }
}

diesel::table! {
    erc1155_transfer_batch (block_number, log_index) {
        block_number -> Int8,
        log_index -> Int8,
        transaction_index -> Int8,
        address -> Bytea,
        operator_0 -> Nullable<Bytea>,
        from_1 -> Nullable<Bytea>,
        to_2 -> Nullable<Bytea>,
    }
}

diesel::table! {
    erc1155_transfer_batch_ids_0 (block_number, log_index, array_index) {
        block_number -> Int8,
        log_index -> Int8,
        transaction_index -> Int8,
        address -> Bytea,
        array_index -> Int8,
        ids_0 -> Nullable<Numeric>,
    }
}

diesel::table! {
    erc1155_transfer_batch_values_1 (block_number, log_index, array_index) {
        block_number -> Int8,
        log_index -> Int8,
        transaction_index -> Int8,
        address -> Bytea,
        array_index -> Int8,
        values_0 -> Nullable<Numeric>,
    }
}

diesel::table! {
    erc1155_transfer_single (block_number, log_index) {
        block_number -> Int8,
        log_index -> Int8,
        transaction_index -> Int8,
        address -> Bytea,
        operator_0 -> Nullable<Bytea>,
        from_1 -> Nullable<Bytea>,
        to_2 -> Nullable<Bytea>,
        id_3 -> Nullable<Numeric>,
        value_4 -> Nullable<Numeric>,
    }
}

diesel::table! {
    erc1155_uri (block_number, log_index) {
        block_number -> Int8,
        log_index -> Int8,
        transaction_index -> Int8,
        address -> Bytea,
        value_0 -> Nullable<Text>,
        id_1 -> Nullable<Numeric>,
    }
}

diesel::table! {
    erc721_approval (block_number, log_index) {
        block_number -> Int8,
        log_index -> Int8,
        transaction_index -> Int8,
        address -> Bytea,
        owner_0 -> Nullable<Bytea>,
        approved_1 -> Nullable<Bytea>,
        tokenid_2 -> Nullable<Numeric>,
    }
}

diesel::table! {
    erc721_transfer (block_number, log_index) {
        block_number -> Int8,
        log_index -> Int8,
        transaction_index -> Int8,
        address -> Bytea,
        from_0 -> Nullable<Bytea>,
        to_1 -> Nullable<Bytea>,
        tokenid_2 -> Nullable<Numeric>,
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

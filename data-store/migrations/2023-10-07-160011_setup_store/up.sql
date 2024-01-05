-- CREATE TYPE token_type AS ENUM ('erc20', 'erc721', 'erc1155', 'unknown');
-- CREATE TYPE content_flag AS ENUM ('nsfl', 'nsfw', 'illegal');
-- CREATE TYPE content_category AS ENUM ( 'sensitive', 'educational', 'art', 'history', 'interactive', 'limited', 'audio', 'video', 'charity');

CREATE TABLE transactions
(
    block_number int8  not null,
    index        int8  not null,
    hash         bytea not null,
    "from"       bytea not null,
    "to"         bytea,
    primary key (block_number, index)
);

CREATE TABLE token_contracts
(
    address          bytea primary key,
    -- token_type       token_type not null,
    name             text,
    symbol           text,
    -- This uniquely defines creation tx
    created_block    int8 not null,
    created_tx_index int8 not null,
    base_uri         text -- May be null for Erc721
--     content_flags    content_flag[],
--     content_category content_category[]
);

CREATE TABLE contract_abis
(
    address bytea primary key,
    abi     jsonb
);

CREATE TABLE nft_metadata
(
    uid  bytea primary key,
    json jsonb not null
);

CREATE TABLE nfts
(
    contract_address      bytea          not null,
    token_id              numeric(78, 0) not null,
    token_uri             text,
    owner                 bytea          not null,
    metadata_id           bytea,
    last_update_block     int8           not null,
    last_update_tx        int8           not null,
    last_update_log_index int8           not null,
    last_transfer_block   int8,
    last_transfer_tx      int8,
    -- Mint/Burn Info
    mint_block            int8           not null,
    mint_tx               int8           not null,
    burn_block            int8,
    burn_tx               int8,
    minter                bytea          not null,
    approved              bytea,
    primary key (contract_address, token_id)
);

CREATE INDEX nfts_metadata_ind ON nfts (metadata_id);

CREATE TABLE approval_for_all
(
    contract_address      bytea not null,
    owner                 bytea not null,
    operator              bytea not null,
    approved              bool  not null,
    -- Used for
    last_update_block     int8  not null,
    last_update_log_index int8  not null,
    -- this was semi-arbitrarily chosen, but makes some sense
    primary key (contract_address, owner)
);

CREATE TABLE blocks
(
    number int8 primary key,
    time   timestamp not null
);

CREATE TABLE erc1155s
(
    contract_address      bytea          not null,
    token_id              numeric(78, 0) not null,
    token_uri             text,
    total_supply          numeric(78, 0) not null,
    creator_address       bytea,
    metadata_id           bytea,
    mint_block            int8           not null,
    mint_tx               int8           not null,
    last_update_block     int8           not null,
    last_update_tx        int8           not null,
    last_update_log_index int8           not null,
    PRIMARY KEY (contract_address, token_id),
    FOREIGN KEY (contract_address) REFERENCES token_contracts (address)
);

CREATE INDEX erc1155_metadata_ind ON erc1155s (metadata_id);

CREATE TABLE erc1155_owners
(
    contract_address bytea          not null,
    token_id         numeric(78, 0) not null,
    owner            bytea          not null,
    balance          numeric(78, 0) not null,
    PRIMARY KEY (contract_address, token_id, owner),
    FOREIGN KEY (contract_address, token_id) REFERENCES erc1155s (contract_address, token_id)
);

-- CREATE TYPE token_type AS ENUM ('erc20', 'erc721', 'erc1155', 'unknown');
-- CREATE TYPE content_flag AS ENUM ('nsfl', 'nsfw', 'illegal');
-- CREATE TYPE content_category AS ENUM ( 'sensitive', 'educational', 'art', 'history', 'interactive', 'limited', 'audio', 'video', 'charity');

CREATE TABLE transactions
(
    block_number int8      not null,
    index        int8      not null,
    hash         bytea     not null,
    block_time   timestamp not null,
    primary key (block_number, index)
);

CREATE TABLE token_contracts
(
    address          bytea primary key,
    -- token_type       token_type not null,
    name             text,
    symbol           text,
    decimals         int2, -- Null for Nfts
    token_uri        text, -- Null for erc20
    -- This uniquely defines creation tx
    created_block    int8       not null,
    created_tx_index int8       not null
--     content_flags    content_flag[],
--     content_category content_category[]
);

CREATE TABLE contract_abis
(
    address bytea primary key,
    abi     jsonb
);

CREATE TABLE nfts
(
    contract_address    bytea          not null,
    token_id            numeric(78, 0) not null,
    owner               bytea          not null,
    -- Below all seems like metadata to me (should have own table)
    -- Last Transfer
    last_transfer_block int8,
    last_transfer_tx    int8,
    -- Mint/Burn Info
    mint_block          int8,
    mint_tx             int8,
    burn_block          int8,
    burn_tx             int8,
    minter              bytea, -- tx_from for transfer from 0
    -- Metadata
    json                jsonb,
    primary key (contract_address, token_id)
);

-- approvals are cleared on transfer.
CREATE TABLE nft_approvals
(
    contract_address bytea          not null,
    token_id         numeric(78, 0) not null,
    approved         bytea not null,
    primary key (contract_address, token_id)
);
CREATE TABLE approval_for_all
(
    contract_address bytea not null,
    owner            bytea not null,
    operator         bytea not null,
    approved         bool  not null,
    -- this was semi-arbitrarily chosen, but makes some sense
    primary key (contract_address, owner)
);

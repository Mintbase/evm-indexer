-- Note that this is not a complete reversal
-- If the services start populating the table with null valued `raw` data,
-- this operation can not be performed.
ALTER TABLE nft_metadata ALTER COLUMN raw DROP NOT NULL;

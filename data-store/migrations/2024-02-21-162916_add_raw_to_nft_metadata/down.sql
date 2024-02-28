-- Remove the `raw` column
ALTER TABLE nft_metadata DROP COLUMN raw;

-- Make the `json` field non-nullable
ALTER TABLE nft_metadata ALTER COLUMN json_field SET NOT NULL;

-- Add a new text column named `raw`
ALTER TABLE nft_metadata ADD COLUMN raw TEXT NOT NULL;

-- Make the existing `json` field nullable
ALTER TABLE nft_metadata ALTER COLUMN json DROP NOT NULL;

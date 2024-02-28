-- Add a new text column named `raw`
ALTER TABLE nft_metadata ADD COLUMN raw TEXT;
UPDATE nft_metadata SET raw = 'v0.0.3';
ALTER TABLE nft_metadata ALTER COLUMN raw SET NOT NULL;

-- Make the existing `json` field nullable
ALTER TABLE nft_metadata ALTER COLUMN json DROP NOT NULL;

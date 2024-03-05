
DELETE
FROM nft_metadata
WHERE raw IS NULL;

UPDATE nfts
SET metadata_id = NULL
where metadata_id IS NOT NULL
  AND metadata_id NOT IN (select uid from nft_metadata);

UPDATE erc1155s
SET metadata_id = NULL
where metadata_id IS NOT NULL
  AND metadata_id NOT IN (select uid from nft_metadata);

ALTER TABLE nft_metadata
    ALTER COLUMN raw SET NOT NULL;

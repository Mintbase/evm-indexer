use eth::types::{Address, NftId, U256};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum NftTokenType {
    ERC721,
    ERC1155,
    UnsupportedStandard,
    NotContract,
    Unknown,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum OpenSeaSafelistRequestStatus {
    /// Verified collection.
    Verified,
    /// Collections that are approved on open sea and can be found in search results.
    Approved,
    /// Collections that requested safe listing on OpenSea.
    Requested,
    /// Brand new collections.
    NotRequested,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OpenSeaCollectionMetadata {
    /// The floor price of the NFT.
    floor_price: Option<f64>,
    /// The name of the collection on OpenSea.
    collection_name: Option<String>,
    /// The approval status of the collection on OpenSea.
    safelist_request_status: Option<OpenSeaSafelistRequestStatus>,
    /// The image URL determined by OpenSea.
    image_url: Option<String>,
    /// The description of the collection on OpenSea.
    description: Option<String>,
    /// The homepage of the collection as determined by OpenSea.
    external_url: Option<String>,
    /// The Twitter handle of the collection.
    twitter_username: Option<String>,
    /// The Discord URL of the collection.
    discord_url: Option<String>,
    /// Timestamp of when the OpenSea metadata was last ingested by Alchemy.
    last_ingested_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NftContract {
    /// The address of the contract.
    address: Address,
    // /// The type of the token in the contract.
    token_type: NftTokenType,
    /// The name of the contract.
    name: Option<String>,
    /// The symbol of the contract.
    symbol: Option<String>,
    /// The number of NFTs in the contract as an integer string. This field is only
    /// available on ERC-721 contracts.
    total_supply: Option<U256>,
    /// OpenSea's metadata for the contract.
    open_sea: Option<OpenSeaCollectionMetadata>,
    /// The address that deployed the NFT contract.
    contract_deployer: Option<Address>,
    /// The block number the NFT contract deployed in.
    deployed_block_number: Option<u64>,
}

// TODO - some URL fields should not be string, but Url.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NftMetadata {
    /// Name of the NFT asset.
    name: Option<String>,
    /// A human-readable description of the NFT asset.
    description: Option<String>,
    /// URL to the NFT asset image.
    image: Option<String>,
    /// The image URL that appears along the top of the NFT asset page.
    /// This tends to be the highest resolution image.
    external_url: Option<String>,
    /// Background color of the NFT item. Usually defined as a 6 character hex string.
    background_color: Option<String>,
    /// The traits, attributes, and characteristics for the NFT asset.
    attributes: Option<Vec<serde_json::Value>>, // TODO - this value field should be maybe Any!
    /// The traits, attributes, and characteristics for the NFT asset.
    properties: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenUri {
    /// URI for the location of the NFT's original metadata blob (ex: the original IPFS link).
    raw: String,
    /// Public gateway URI for the raw URI. Generally offers better performance.
    gateway: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    /// URI for the location of the NFT's original metadata blob (ex: the original IPFS link).
    raw: String,
    /// Public gateway URI for the raw URI. Generally offers better performance.
    gateway: String,
    /// URL for a resized thumbnail of the NFT media asset.
    thumbnail: Option<String>,
    /// The media format (ex: jpg, gif, png) of the {@link gateway} and {@link thumbnail} assets.
    format: Option<String>,
    /// The size of the media asset in bytes.
    bytes: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum NftSpamClassification {
    Erc721TooManyOwners,
    Erc721TooManyTokens,
    Erc721DishonestTotalSupply,
    MostlyHoneyPotOwners,
    OwnedByMostHoneyPots,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpamInfo {
    is_spam: bool,
    /// A list of reasons why an NFT contract was marked as spam.
    classifications: Vec<NftSpamClassification>,
}
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AcquiredAt {
    /** Timestamp of the block at which an NFT was last acquired. */
    block_timestamp: Option<String>,
    /** Block number of the block at which an NFT was last acquired. */
    block_number: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NftContent {
    id: NftId,
    /// The NFT's underlying contract and relevant contract metadata.
    contract: NftContract,
    /// The NFT title.
    title: String,
    /// The NFT description.
    description: String,
    /// When the NFT was last updated in the blockchain. Represented in ISO-8601 format.
    time_last_updated: String,
    /// Holds an error message if there was an issue fetching metadata.
    metadata_error: Option<String>,
    /// The metadata fetched from the metadata URL specified by the NFT.
    /// None if unable to fetch.
    metadata: Option<NftMetadata>,
    /// URIs for accessing the NFT's metadata blob.
    token_uri: Option<TokenUri>,
    /// URIs for accessing the NFT's media assets.
    media: Vec<Media>,
    /// Detailed information on why an NFT was classified as spam.
    spam_info: Option<SpamInfo>,
    /// Time at which the NFT was most recently acquired by the user. Only
    /// available when specifying `orderBy: NftOrdering.TRANSFERTIME` in the
    /// request.
    acquired_at: Option<AcquiredAt>,
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn serialize() {
        let response_json = r#"{
          "collection": {
            "bannerImageUrl": "https://lh3.googleusercontent.com/GHhptRLebBOWOy8kfXpYCVqsqdes-1-6I_jbuRnGTHHW6TD63CtciH75Dotfu2u8v6EmkWt-tjhkFRVLxRUwgMfKqqy5W24AolJayeo=s2500",
            "externalUrl": null,
            "name": "World of Women",
            "slug": "world-of-women-nft"
          },
          "contract": {
            "address": "0xe785E82358879F061BC3dcAC6f0444462D4b5330",
            "contractDeployer": "0xc9b6321dc216D91E626E9BAA61b06B0E4d55bdb1",
            "deployedBlockNumber": 12907782,
            "isSpam": null,
            "name": "World Of Women",
            "openSeaMetadata": {
              "bannerImageUrl": "https://lh3.googleusercontent.com/GHhptRLebBOWOy8kfXpYCVqsqdes-1-6I_jbuRnGTHHW6TD63CtciH75Dotfu2u8v6EmkWt-tjhkFRVLxRUwgMfKqqy5W24AolJayeo=s2500",
              "collectionName": "World of Women",
              "collectionSlug": "world-of-women-nft",
              "description": "World of Women is a collection of 10,000 NFTs that gives you full access to our network of artists, creators, entrepreneurs, and executives who are championing diversity and equal opportunity on the blockchain.\r\n\r\nCreated and illustrated by Yam Karkai (@ykarkai), World of Women has made prominent appearances at Christie's, The New Yorker and Billboard.\r\n\r\nJoin us to receive exclusive access to NFT drops, experiences, and much more.\r\n\r\nThe Time is WoW.",
              "discordUrl": "https://discord.gg/worldofwomen",
              "externalUrl": null,
              "floorPrice": 0.897995,
              "imageUrl": "https://openseauserdata.com/files/8604de2d9aaec98dd389e3af1b1a14b6.gif",
              "lastIngestedAt": "2023-12-13T14:43:42.000Z",
              "safelistRequestStatus": "verified",
              "twitterUsername": "worldofwomennft"
            },
            "spamClassifications": [],
            "symbol": "WOW",
            "tokenType": "ERC721",
            "totalSupply": "10000"
          },
          "description": null,
          "image": {
            "cachedUrl": "https://nft-cdn.alchemy.com/eth-mainnet/9316855d8f60a32cd44aa71f07cd7dc1",
            "contentType": "image/png",
            "originalUrl": "https://ipfs.io/ipfs/QmUkdJKCsV8ixm2eDLJGosH8Bntwwx942YXxfuF9yXPBzi",
            "pngUrl": "https://res.cloudinary.com/alchemyapi/image/upload/convert-png/eth-mainnet/9316855d8f60a32cd44aa71f07cd7dc1",
            "size": 105117,
            "thumbnailUrl": "https://res.cloudinary.com/alchemyapi/image/upload/thumbnailv2/eth-mainnet/9316855d8f60a32cd44aa71f07cd7dc1"
          },
          "mint": {
            "blockNumber": null,
            "mintAddress": null,
            "timestamp": null,
            "transactionHash": null
          },
          "name": "WoW #44",
          "owners": null,
          "raw": {
            "error": null,
            "metadata": {
              "attributes": [
                {
                  "trait_type": "Background",
                  "value": "Green Orange"
                },
                {
                  "trait_type": "Skin Tone",
                  "value": "Medium Gold"
                },
                {
                  "trait_type": "Eyes",
                  "value": "Green To The Left"
                },
                {
                  "trait_type": "Facial Features",
                  "value": "Freckles"
                },
                {
                  "trait_type": "Hairstyle",
                  "value": "Boy Cut"
                },
                {
                  "trait_type": "Clothes",
                  "value": "Tunic"
                },
                {
                  "trait_type": "Earrings",
                  "value": "Spikes"
                },
                {
                  "trait_type": "Mouth",
                  "value": "Slight Smile"
                },
                {
                  "trait_type": "Lips Color",
                  "value": "Purple"
                }
              ],
              "image": "ipfs://QmUkdJKCsV8ixm2eDLJGosH8Bntwwx942YXxfuF9yXPBzi",
              "name": "WoW #44"
            },
            "tokenUri": "ipfs://QmTNBQDbggLZdKF1fRgWnXsnRikd52zL5ciNu769g9JoUP/44"
          },
          "timeLastUpdated": "2023-12-13T15:51:22.091Z",
          "tokenId": "44",
          "tokenType": "ERC721",
          "tokenUri": "https://alchemy.mypinata.cloud/ipfs/QmTNBQDbggLZdKF1fRgWnXsnRikd52zL5ciNu769g9JoUP/44"
        }"#;
        let serialization_result = serde_json::from_value::<NftContent>(response_json.into());
        assert!(serialization_result.is_ok());
    }
}

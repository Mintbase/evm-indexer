use crate::routes::token::metadata::util::{http_link_ipfs, ENS_URI};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use eth::types::{NftId, ENS_ADDRESS};
use serde_json::Value;
use url::Url;

use super::MetadataFetching;

pub struct Homebrew {}

#[async_trait]
impl MetadataFetching for Homebrew {
    async fn get_nft_metadata(&self, token: NftId, uri: Option<String>) -> Result<Value> {
        let uri = match token.address {
            // If ENS --> We know the URI.
            ENS_ADDRESS => Some(format!("{ENS_URI}/{}", token.token_id)),
            _ => uri,
        };
        let mut metadata_url = match uri {
            None => {
                // TODO - use the TokenId only and attempt to read from Alchemy.
                return Err(anyhow!("Empty bytes for metadata url!"));
            }
            Some(token_uri) => Url::parse(&token_uri)?,
        };
        tracing::debug!("parsed tokenUri as {:?}", metadata_url);

        // TODO - implement token type indication.
        // if token.type_ == erc::ERCNFTType::ERC1155 {
        //     metadata_url.set_path(
        //         &metadata_url
        //             .path()
        //             .replace("%7Bid%7D", &hex::encode(token.id)),
        //     );
        // }
        if metadata_url.scheme() == "ipfs" {
            metadata_url = http_link_ipfs(metadata_url)?;
        }

        // If ERC1155 we need to do a replacement on the url.
        tracing::debug!("Reqwest content at {metadata_url}");
        let value: Value = reqwest::get(metadata_url).await?.json().await?;
        tracing::debug!("Reqwest response {:?}", value);
        Ok(value)
        // TODO serialize as NftContent.
        // serde_json::from_value::<NftContent>(value).context("serialize")
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use eth::types::{Address, U256};

    use super::*;

    #[tokio::test]
    async fn get_metadata_ipfs() {
        let fetcher = Homebrew {};
        let content_result = fetcher
            .get_nft_metadata(
                NftId {
                    address: Address::from_str("0xbc4ca0eda7647a8ab7c2061c2e118a18a936f13d")
                        .unwrap(),
                    token_id: U256::from(2),
                },
                Some("ipfs://QmeSjSinHpPnmXmspMjwiXyN6zS4E9zccariGR3jxcaWtq/2".into()),
            )
            .await;
        assert!(content_result.is_ok())
    }

    #[tokio::test]
    async fn ens_override() {
        let content_result = Homebrew {}
            .get_nft_metadata(
                NftId {
                    address: Address::from_str("0x57F1887A8BF19B14FC0DF6FD9B2ACC9AF147EA85")
                        .unwrap(),
                    token_id: U256::from(1),
                },
                None,
            )
            .await;
        assert!(content_result.is_ok())
    }

    #[tokio::test]
    async fn get_metadata_failure() {
        // Enjin
        // No uri because of: https://enjin.io/blog/nft-migration-to-enjin-blockchain-starts-december-8
        let token = NftId {
            address: Address::from_str("0xFAAFDC07907FF5120A76B34B731B278C38D6043C").unwrap(),
            token_id: U256::from_dec_str(
                "10855508365998405147019449313071050427871334385647330815536805870982878199808",
            )
            .unwrap(),
        };
        assert!(Homebrew {}.get_nft_metadata(token, None).await.is_err())
    }

    #[tokio::test]
    async fn get_metadata_single() {
        let token = NftId {
            address: Address::from_str("0x659A4BDAAACC62D2BD9CB18225D9C89B5B697A5A").unwrap(),
            token_id: U256::from_dec_str("1200").unwrap(),
        };
        let result = Homebrew {}
            .get_nft_metadata(
                token,
                Some("https://fateofwagdie.com/api/characters/metadata/1200".into()),
            )
            .await;
        assert!(result.is_ok());
        // println!("{:#}", result.unwrap());
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    #[ignore = "passes locally but not on github actions: https://github.com/Mintbase/evm-indexer/issues/136"]
    async fn get_metadata_bad_chars() {
        let token = NftId {
            address: Address::from_str("0xcf3a65864DFB6d4aEAaa93Dde66ad3deb227c3E3").unwrap(),
            token_id: U256::from_dec_str("2325").unwrap(),
        };
        let bad_uri = Some(
            "https://5h5jydmla4qvcjvmdgcgnnkdhy0ddrod.lambda-url.us-east-2.on.aws/?id=2325&data="
                .into(),
        );
        let result = Homebrew {}.get_nft_metadata(token, bad_uri).await;
        assert!(result.is_ok());
    }
}

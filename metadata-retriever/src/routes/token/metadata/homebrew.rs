use crate::routes::token::metadata::util::{http_link_ipfs, ENS_URI};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use eth::types::{NftId, ENS_ADDRESS};
use reqwest::Response;
use std::time::Duration;
use url::Url;

use super::{FetchedMetadata, MetadataFetching};

pub struct Homebrew {
    client: reqwest::Client,
}

impl Homebrew {
    pub fn new(timeout_seconds: u64) -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(timeout_seconds))
                .build()?,
        })
    }

    async fn make_request(&self, url: Url) -> Result<Response, reqwest::Error> {
        let response = self.client.get(url).send().await?;
        // Ensure the request was successful and map the error if not
        response.error_for_status_ref()?;
        Ok(response)
    }
}

#[async_trait]
impl MetadataFetching for Homebrew {
    async fn get_nft_metadata(&self, token: NftId, uri: Option<String>) -> Result<FetchedMetadata> {
        let uri = match token.address {
            // If ENS --> We know the URI.
            ENS_ADDRESS => Some(format!("{ENS_URI}/{}", token.token_id)),
            _ => uri,
        };
        let mut metadata_url = match uri {
            None => {
                // TODO - use the TokenId only and attempt to read from Alchemy.
                return Err(anyhow!("empty metadata url!"));
            }
            Some(token_uri) => Url::parse(&token_uri)?,
        };
        tracing::debug!("parsed tokenUri as {:?}", metadata_url);

        // TODO - implement token type indication.
        //  if token.type_ == erc::ERCNFTType::ERC1155 {
        //     metadata_url.set_path(
        //         &metadata_url
        //             .path()
        //             .replace("%7Bid%7D", &hex::encode(token.id)),
        //     );
        //  }
        if metadata_url.scheme() == "ipfs" {
            metadata_url = http_link_ipfs(metadata_url)?;
        }

        // If ERC1155 we (may) need to do a replacement on the url.
        tracing::debug!("fetching content at {metadata_url}");
        match self.make_request(metadata_url.clone()).await {
            Ok(response) => FetchedMetadata::from_response(response).await,
            Err(e) => {
                if e.is_timeout() {
                    tracing::warn!("Request timed out - suspected bad url: {}", metadata_url);
                    return Ok(FetchedMetadata::timeout());
                }
                Err(anyhow!(e.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use eth::types::{Address, U256};

    use super::*;

    fn get_fetcher() -> Homebrew {
        Homebrew::new(2).unwrap()
    }

    #[tokio::test]
    async fn get_metadata_ipfs() {
        let content_result = get_fetcher()
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
        let content_result = get_fetcher()
            .get_nft_metadata(
                NftId::from_str("0x57F1887A8BF19B14FC0DF6FD9B2ACC9AF147EA85/29779156741555693913844064206490564547787150513426002335927947804027615620901").unwrap(),
                None,
            )
            .await;
        assert!(content_result.is_ok())
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn get_metadata_failure() {
        let fetcher = get_fetcher();
        // Enjin
        // No uri because of: https://enjin.io/blog/nft-migration-to-enjin-blockchain-starts-december-8
        let token = NftId::from_str("0xFAAFDC07907FF5120A76B34B731B278C38D6043C/10855508365998405147019449313071050427871334385647330815536805870982878199808").unwrap();
        assert_eq!(
            fetcher
                .get_nft_metadata(token, None)
                .await
                .unwrap_err()
                .to_string(),
            "empty metadata url!"
        );
    }
    #[tokio::test]
    async fn get_metadata_timeout() {
        // Known Timeout URL.
        let token = NftId::from_str("0x42C24AF9C858C6AC5D65F8F0575B9655DD53C8AE/322").unwrap();
        let result = get_fetcher().get_nft_metadata(token, Some("https://ipfs.virtualhosting.hk/srclub.io/ipfs/QmYjTyaCRzKiTNXHVL93sUS3R5tuDDQofQ7pj8hL6CTR3J/322.json".into())).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn get_metadata_single() {
        let token = NftId {
            address: Address::from_str("0x659A4BDAAACC62D2BD9CB18225D9C89B5B697A5A").unwrap(),
            token_id: U256::from_dec_str("1200").unwrap(),
        };
        let result = Homebrew::new(2)
            .unwrap()
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
        let token = NftId::from_str("0xcf3a65864DFB6d4aEAaa93Dde66ad3deb227c3E3/2325").unwrap();
        let bad_uri = Some(
            "https://5h5jydmla4qvcjvmdgcgnnkdhy0ddrod.lambda-url.us-east-2.on.aws/?id=2325&data="
                .into(),
        );
        let result = get_fetcher().get_nft_metadata(token, bad_uri).await;
        assert!(result.is_ok());
    }
}

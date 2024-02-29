use crate::routes::token::metadata::{data_url::UriType, util::ENS_URI};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use eth::types::{NftId, ENS_ADDRESS};
use reqwest::Response;
use std::{str::FromStr, time::Duration};
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
        tracing::debug!("reqwest external content at {url}");
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
        let uri_type = match uri {
            None => {
                // TODO - use the TokenId only and attempt to read from Alchemy.
                return Err(anyhow!("Empty bytes for metadata url!"));
            }
            Some(token_uri) => UriType::from_str(&token_uri)?,
        };
        tracing::debug!("parsed tokenUri as {:?}", uri_type);
        return match uri_type {
            UriType::Url(metadata_url) => {
                tracing::debug!("Url Type");
                // If ERC1155 we (may) need to do a replacement on the url.
                match self.make_request(metadata_url.clone()).await {
                    Ok(response) => FetchedMetadata::from_response(response).await,
                    Err(e) => {
                        if e.is_timeout() {
                            tracing::warn!(
                                "Request timed out - suspected bad url: {}",
                                metadata_url
                            );
                            return Ok(FetchedMetadata::timeout());
                        }
                        Err(anyhow!(e.to_string()))
                    }
                }
            }
            UriType::Ipfs(path) => {
                tracing::debug!("reqwest IPFS at CID {path:?}");
                match self.make_request(Url::from(path)).await {
                    Ok(response) => FetchedMetadata::from_response(response).await,
                    Err(e) => {
                        if e.is_timeout() {
                            return Ok(FetchedMetadata::timeout());
                        }
                        Err(anyhow!(e.to_string()))
                    }
                }
            }
            UriType::Data(content) => {
                tracing::debug!("Data Type");
                Ok(FetchedMetadata::from_str(&content)?)
            }
            UriType::Unknown(mystery) => Err(anyhow!(mystery)),
        };
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use eth::types::{Address, U256};

    use super::*;

    fn get_fetcher() -> Homebrew {
        Homebrew::new(3).unwrap()
    }

    #[tokio::test]
    async fn ens_override() {
        let token_id =
            "31913142322058250240866303485500832898255309823098443696464130050119537886147";
        let content_result = get_fetcher()
            .get_nft_metadata(
                NftId::from_str(&format!("{ENS_ADDRESS}/{token_id}")).unwrap(),
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
        assert!(get_fetcher().get_nft_metadata(token, None).await.is_err())
    }

    #[tokio::test]
    async fn get_metadata_single() {
        let token = NftId {
            address: Address::from_str("0x659A4BDAAACC62D2BD9CB18225D9C89B5B697A5A").unwrap(),
            token_id: U256::from_dec_str("1200").unwrap(),
        };
        let result = get_fetcher()
            .get_nft_metadata(
                token,
                Some("https://fateofwagdie.com/api/characters/metadata/1200".into()),
            )
            .await;
        assert!(result.is_ok());
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

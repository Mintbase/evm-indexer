use crate::routes::token::metadata::{data_url::UriType, util::ENS_URI};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use eth::types::{NftId, ENS_ADDRESS};
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

    async fn url_request(&self, url: Url) -> Result<FetchedMetadata> {
        tracing::debug!("reqwest external content at {url}");
        let result = self.client.get(url).send().await;
        match result {
            Ok(response) => {
                if let Err(status_error) = response.error_for_status_ref() {
                    let error_code = status_error.status().expect("is error");
                    return Ok(FetchedMetadata::error(&error_code.to_string()));
                }
                FetchedMetadata::from_response(response).await
            }
            Err(err) => {
                let err_string = err.to_string();
                if err_string.contains("error trying to connect: ") {
                    // Known errors can be recorded and handled properly.
                    let message = err_string
                        .split("error trying to connect: ")
                        .last()
                        .expect("message after");
                    return Ok(FetchedMetadata::error(message));
                }
                Err(anyhow!(err_string))
            }
        }
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
                tracing::debug!("Url Type for {token}");
                // If ERC1155 we (may) need to do a replacement on the url.
                self.url_request(metadata_url).await
            }
            UriType::Ipfs(path) => {
                tracing::debug!("IPFS Type for {token}");
                self.url_request(Url::from(path)).await
            }
            UriType::Data(content) => {
                tracing::debug!("Data Type for {token}");
                Ok(FetchedMetadata::from_str(&content)?)
            }
            UriType::Unknown(mystery) => {
                tracing::debug!("Unknown Type for {token}");
                Err(anyhow!(mystery))
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use futures::future::join_all;
    use std::str::FromStr;

    use eth::types::{Address, U256};

    use super::*;

    fn get_fetcher() -> Homebrew {
        Homebrew::new(5).unwrap()
    }

    async fn retrieve_and_resolve_request_errors(
        client: &Homebrew,
        urls: Vec<Url>,
    ) -> Vec<FetchedMetadata> {
        let futures = urls.into_iter().map(|u| client.url_request(u));
        join_all(futures)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect()
    }

    async fn get_results_for_urls(client: &Homebrew, url_strings: &[&str]) -> Vec<FetchedMetadata> {
        let urls: Vec<_> = url_strings.iter().map(|x| Url::parse(x).unwrap()).collect();
        retrieve_and_resolve_request_errors(client, urls.clone()).await
    }

    async fn all_same_results(
        client: &Homebrew,
        url_strings: &[&str],
        expected: FetchedMetadata,
    ) -> bool {
        let results = get_results_for_urls(client, url_strings).await;
        for (url, result) in url_strings.iter().zip(results) {
            if result != expected {
                println!(
                    "Unexpected Result for {} \n  expected: {}\n       got: {}",
                    url, expected.raw, result.raw
                );
                return false;
            }
        }
        true
    }

    #[tokio::test]
    async fn url_request_certificate_errors() {
        let client = get_fetcher();
        // Untrusted Certificate
        let urls = [
            // This one inconsistently returns untrusted and closed via error
            // "https://api.derp.life/token/0",
            // got: certificate has expired
            // "https://assets.knoids.com/knoids/312",
            // unable to get local issuer certificate.
            "https://evaverse.com/api/turtle.php?id=4171",
            "https://mint.joinalienverse.io/api/metadata/918",
            "https://metadata.hungrywolves.com/api/hungry-wolves/2531",
            "https://metadata.exclusible.com/christofle/332",
            "https://royalsociety.io:1335/rsopchips/opensea/2314",
            "https://metadata.monsterapeclub.com/6680",
            "https://mint-penguinkart.com/29",
        ];
        let results = get_results_for_urls(&client, &urls).await;
        // There are inconsistencies with this error locally and on github actions.
        // This test ensures
        // 1. certificate is contained in the message
        assert!(results.iter().all(|x| x.raw.contains("certificate")));
        // Can't get this consistent.
        // // 2. All the messages are the same.
        // let error_set = results.iter().map(|x| &x.raw).collect::<HashSet<_>>();
        // println!("Certificate Error Set: {:?}", error_set);
        // assert_eq!(error_set.len(), 1);
    }
    #[tokio::test]
    async fn url_request_error_trying_to_connect() {
        let client = get_fetcher();

        // DNS Error
        // There are several variants of DNS error:
        // Examples:
        //  - failed to lookup address information: nodename nor servname provided, or not known
        //  - failed to lookup address information: Name or service not known
        let results = get_results_for_urls(
            &client,
            &[
                "https://imgcdn.dragon-town.wtf/json/1402.json",
                "https://misfits.lastknown.com/metadata/834.json",
            ],
        )
        .await;
        assert!(results.iter().all(|x| x.raw.contains("dns error:")));

        // // Protocol Error
        // let urls = ["https://metroverse.com/blocks/66"];
        // assert!(
        //     all_same_results(
        //         &client,
        //         &urls,
        //         FetchedMetadata::error("bad protocol version")
        //     )
        //     .await
        // );

        // TCP Error
        let results = get_results_for_urls(
            &client,
            &[
                "https://mint.feev.mc/api/ipfs/metadata/platinum/125",
                "https://kaijukongzdatabase.com/metadata/1404",
                "https://xoxonft.io/meta/101/1",
            ],
        )
        .await;
        // os error 61 -- macOS
        // os error 111 -- Linux
        assert!(results.iter().all(|x| x
            .raw
            .contains("tcp connect error: Connection refused (os error")));

        // Connection reset by peer
        let results = get_results_for_urls(
            &client,
            &["https://niftyfootball.cards/api/network/1/token/1610"],
        )
        .await;
        assert!(results
            .iter()
            .all(|x| x.raw.contains("Connection reset by peer")));

        // Unexpected EOF
        assert!(
            all_same_results(
                &client,
                &["https://api.raid.party/metadata/fighter/16148",],
                FetchedMetadata::error("unexpected EOF")
            )
            .await
        );

        // Internal Error
        let results = get_results_for_urls(
            &client,
            &[
                "https://api.pupping.io/pupping/meta/50",
                "https://metadata.hexinft.io/api/token/hexi/1357",
            ],
        )
        .await;
        assert!(results.iter().all(|x| x.raw.contains("internal error")));
    }

    #[tokio::test]
    async fn url_request_400_status_errors() {
        let client = get_fetcher();

        // 400 Bad Request
        let urls = ["https://www.metadoge.art/api/2D/metadata/4435"];
        assert!(all_same_results(&client, &urls, FetchedMetadata::error("400 Bad Request")).await);

        // 402 Payment Required
        assert!(
            all_same_results(
                &client,
                &[
                    "https://assets.nlbnft.com/api/metadata/82.json",
                    "https://partypenguins.club/api/penguin/208",
                    "https://ikani.ai/metadata/845",
                ],
                FetchedMetadata::error("402 Payment Required")
            )
            .await
        );

        // 403 Forbidden
        assert!(
            all_same_results(
                &client,
                &[
                    "https://api.lasercat.co/metadata/221",
                    "https://bmcdata.s3.us-west-1.amazonaws.com/UltraMetadata/1324",
                    "https://dreamerapi.bitlectrolabs.com/dreamers/metadata/17",
                ],
                FetchedMetadata::error("403 Forbidden")
            )
            .await
        );

        // 404 Not Found
        assert!(
            all_same_results(
                &client,
                &[
                    "https://airxnft.herokuapp.com/api/token/866",
                    "https://ipfs.io/ipfs/QmbDf9xpQwm6cN1pY1dsh6eKeq8HBnDzKD8Ym6XhFGiptv/0",
                    "https://skreamaz.herokuapp.com/api/2259",
                ],
                FetchedMetadata::error("404 Not Found")
            )
            .await
        );

        // 410 Gone
        assert!(
            all_same_results(
                &client,
                &[
                    "https://ipfs.io/ipfs/QmSkzoReMj5ggmU69RaFZ6XHqPor1ZTtmTRVfZoYF9rfET/621",
                    "http://api.ramen.katanansamurai.art/Metadata/1495",
                    "https://ipfs.io/ipfs/QmeN7ZdrTGpbGoo8URqzvyiDtcgJxwoxULbQowaTGhTeZc/6712.json",
                ],
                FetchedMetadata::error("410 Gone")
            )
            .await
        );
    }

    #[tokio::test]
    async fn url_request_500_status_errors() {
        let client = get_fetcher();

        // 500 Internal Server Error
        assert!(
            all_same_results(
                &client,
                &[
                    "https://metadata.theavenue.market/v1/token/mrbean/769",
                    "https://us-central1-hangry-tools.cloudfunctions.net/editionMetadata?edition=4128",
                    "https://billionaires.io/api/billionaires/2298",
                    "https://ipfs.io/ipns/749.dogsunchainednft.com",
                    "https://beepos.fun/api/beepos/5671",
                    "https://us-central1-polymorphmetadata.cloudfunctions.net/images-function-v2?id=2050",
                    "https://lilmonkies.com/api/monkies/3956",
                ],
                FetchedMetadata::error("500 Internal Server Error")
            )
            .await
        );

        // 502 Bad Gateway
        assert!(all_same_results(&client, &[
            "https://meta.showme.fan/nft/meta/1/showme/2772",
            "https://app.ai42.art/api/loop/4743",
            "https://api.supducks.com/megatoads/metadata/174",
            "https://api.nonfungiblecdn.com/zenape/metadata/4413",
            "https://api3.cargo.build/batches/metadata/0xe573b99ffd4df2a82ea0986870c33af4cb8a5589/30",
            "https://photo-nft.rexit.info/json-file/332",
        ], FetchedMetadata::error("502 Bad Gateway")).await);

        // 503 Service Unavailable
        assert!(
            all_same_results(
                &client,
                &[
                    "https://armory.warrioralliance.io/assets/347.json",
                    "https://minting-pipeline-10.herokuapp.com/3836",
                    "https://alphiewhales.herokuapp.com/tokens/632",
                    "https://minting-pipeline-beatport.herokuapp.com/2480",
                    "https://pss-silverback-go-nft-api.herokuapp.com/token/518",
                    "https://armory.warrioralliance.io/assets/Season%201/336.json",
                    "https://lit-island-00614.herokuapp.com/api/v1/uuvx/1157",
                    "https://fast-food-fren.herokuapp.com/spookyfrens/1010",
                    "https://lit-island-00614.herokuapp.com/api/v1/uuv2/chromosomes/1268",
                ],
                FetchedMetadata::error("503 Service Unavailable")
            )
            .await
        );
    }

    #[tokio::test]
    async fn url_request_unknown_errors() {
        let client = get_fetcher();
        // Unknown
        assert!(
            all_same_results(
                &client,
                &[
                    "https://undead-town.xyz/api/metadata/698",
                    "https://metadata.pieceofshit.wtf/json/0.json",
                    "https://api.traitsniper.com/api/metadata/583.json",
                    "https://metadata.nftown.com/shares/385",
                ],
                FetchedMetadata::error("530 <unknown status code>")
            )
            .await
        );
        assert!(
            all_same_results(
                &client,
                &[
                    "https://pandaparadise.xyz/api/token/2238.json",
                    "https://api.clayfriends.io/friend/121",
                ],
                FetchedMetadata::error("521 <unknown status code>")
            )
            .await
        );

        assert!(
            all_same_results(
                &client,
                &["https://api.mintverse.world/word/metadata/2215",],
                FetchedMetadata::error("526 <unknown status code>")
            )
            .await
        );
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

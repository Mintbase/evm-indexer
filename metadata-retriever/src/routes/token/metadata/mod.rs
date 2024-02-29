use anyhow::Result;
use async_trait;
use data_store::models::NftMetadata;
use eth::types::NftId;
use reqwest::Response;
use serde_json::Value;

pub(crate) mod data_url;
pub mod homebrew;
mod ipfs;
mod util;

#[async_trait::async_trait]
pub trait MetadataFetching: Send + Sync {
    async fn get_nft_metadata(&self, token: NftId, uri: Option<String>) -> Result<FetchedMetadata>;
}

#[derive(Debug, PartialEq)]
pub struct FetchedMetadata {
    raw: String,
    json: Option<Value>,
}

impl From<FetchedMetadata> for NftMetadata {
    fn from(val: FetchedMetadata) -> Self {
        NftMetadata::new(&val.raw, val.json)
    }
}

impl FetchedMetadata {
    pub async fn from_response(response: Response) -> Result<Self> {
        let body = response.text().await?;
        let json = match serde_json::from_str::<Value>(&body) {
            Ok(json) => Some(json),
            Err(err) => {
                // Log the error issue
                tracing::warn!("Invalid JSON content: {} - using None", err);
                None
            }
        };
        Ok(Self { raw: body, json })
    }

    pub fn error(text: &str) -> Self {
        Self {
            raw: text.into(),
            json: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::routes::token::metadata::{homebrew::Homebrew, MetadataFetching};
    use csv::ReaderBuilder;
    use eth::{
        rpc::{ethers::Client, EthNodeReading},
        types::{Address, NftId, U256},
    };
    use flate2::read::GzDecoder;
    use rand::{seq::SliceRandom, thread_rng};
    use std::{fs::File, io::BufReader, path::Path, str::FromStr};
    use tracing_test::traced_test;

    #[derive(serde::Deserialize, Clone)]
    struct InputCSV {
        contract_address: Address,
        token_id: U256,
        token_uri: String,
    }

    fn shuffle_and_take<T>(vec: &mut Vec<T>, n: usize) -> Vec<T>
    where
        T: Clone,
    {
        let mut rng = thread_rng();
        // Shuffle the vector in-place
        vec.shuffle(&mut rng);
        // Take the first n elements and collect them into a new vector
        vec.iter().take(n).cloned().collect()
    }

    fn load_test_data(file_name: &str, num_rows: Option<usize>) -> Vec<InputCSV> {
        // Test data was generated with the following SQL from the store DB:
        // ```sql
        // WITH RankedNFTs AS (
        //     SELECT contract_address,
        //     token_id,
        //     token_uri,
        //     ROW_NUMBER() OVER (PARTITION BY contract_address ORDER BY token_id ASC) AS rk
        //     FROM nfts
        //     WHERE token_uri IS NOT NULL
        // )
        // SELECT contract_address,
        // token_id,
        // token_uri
        // FROM RankedNFTs
        // WHERE rk = 1;
        // ```
        let path = Path::new("test_data").join(file_name);
        let file = File::open(&path).unwrap_or_else(|_| panic!("couldn't open file at {:?}", path));
        let buf_reader = BufReader::new(file);
        let gz_decoder = GzDecoder::new(buf_reader);

        // Create a CSV reader from the decompressed data
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(gz_decoder);

        // Create a vector to hold the records
        let mut records: Vec<InputCSV> = Vec::new();

        // Iterate over each record and deserialize it into a Person struct
        for result in rdr.deserialize() {
            let rec: InputCSV = result.expect("could not deserialize result");
            // Add the Person to the vector
            records.push(rec);
        }
        if let Some(length) = num_rows {
            // Stop once we've populated enough rows.
            if length < records.len() {
                return shuffle_and_take(&mut records, length);
            }
        }
        records
    }

    #[tokio::test]
    #[traced_test]
    async fn unprocessable_entity() {
        let token = NftId::from_str("0xC36442B4A4522E871399CD717ABDD847AB11FE88/257999").unwrap();
        let eth_rpc = Client::new("https://rpc.ankr.com/eth").unwrap();
        let uri = eth_rpc
            .get_uris(&[token])
            .await
            .get(&token)
            .unwrap()
            .clone();

        let fetcher = Homebrew::new(2).unwrap();
        let result = fetcher.get_nft_metadata(token, uri).await;
        println!("{result:?}");
    }
    async fn run_large_file_test(file: &str, sim_size: Option<usize>) {
        let data = load_test_data(file, sim_size);
        let total_rows = data.len();
        let mut err_count = 0;
        let fetcher = Homebrew::new(1).unwrap();
        for (index, entry) in data.into_iter().enumerate() {
            let result = fetcher
                .get_nft_metadata(
                    NftId {
                        address: entry.contract_address,
                        token_id: entry.token_id,
                    },
                    Some(entry.token_uri.clone()),
                )
                .await;
            if let Err(err) = result {
                err_count += 1;
                println!("Error at row {index} with {} {err:?}", entry.token_uri);
            }
        }
        println!(
            "Processed {} rows with {}% success rate",
            total_rows,
            (total_rows - err_count) as f32 / total_rows as f32
        );
    }

    #[tokio::test]
    #[ignore = "very large test many external requests"]
    async fn real_url_simulation() {
        run_large_file_test("store_public_nfts.csv.gz", Some(1000)).await;
    }

    #[tokio::test]
    #[ignore = "very large test many external requests"]
    async fn data_url_simulation() {
        run_large_file_test("store_public_data_uri.csv.gz", Some(100)).await;
    }
}

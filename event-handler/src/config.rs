use serde::Deserialize;
use std::{fs, path::PathBuf};

/// Where chain data should be retrieved from
#[derive(Debug, Deserialize, PartialEq)]
pub enum ChainDataSource {
    Database,
    Node,
}

#[derive(Deserialize)]
pub struct HandlerConfig {
    /// Source of chain data (blocks & transactions)
    pub chain_data_source: ChainDataSource,
    /// BlockRange width for run-loop processing.
    pub page_size: i64,
    /// True when this service should be responsible for fetching missing node data.
    pub fetch_node_data: bool,
}

impl HandlerConfig {
    pub fn from_path(path: &PathBuf) -> Self {
        let data = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("I/O error while reading {path:?}: {e:?}"));
        toml::de::from_str(&data).unwrap_or_else(|err| {
            if std::env::var("TOML_TRACE_ERROR").is_ok_and(|v| v == "1") {
                panic!("failed to parse TOML config at {path:?} with: {err:#?}")
            } else {
                panic!(
                    "failed to parse TOML config. Set TOML_TRACE_ERROR=1 to print \
                 parsing error but this may leak secrets."
                )
            }
        })
    }
}

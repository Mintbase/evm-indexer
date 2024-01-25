use crate::config::ChainDataSource;

use url::Url;

#[derive(Debug, clap::Parser)]
pub struct Args {
    /// Source database connection string.
    #[clap(long, env)]
    pub source_url: Url,

    /// Store database connection string.
    #[clap(long, env)]
    pub store_url: Url,

    /// DB schema (should be the same for both source and store)
    #[clap(long, env)]
    pub db_schema: String,

    /// The Ethereum RPC endpoint.
    #[clap(long, env)]
    pub node_url: Url,

    /// The log filter.
    #[clap(long, env, default_value = "debug")]
    pub log: String,

    /// Source of additional on-chain data
    #[clap(long, env, value_enum, default_value = "database")]
    pub chain_source: ChainDataSource,

    /// BlockRange width for run-loop processing.
    #[clap(long, env, default_value = "1000")]
    pub page_size: i64,

    /// TokenUri retry blocks
    #[clap(long, env, default_value = "1000")]
    pub uri_retry_blocks: i64,

    /// Wait time between buffered requests in milliseconds (Eth Rpc)
    #[clap(long, env, default_value = "20")]
    pub node_batch_delay: u64,

    /// Include to skip additional on-chain data fetching
    #[clap(long, env)]
    pub skip_node_fetching: bool,
}

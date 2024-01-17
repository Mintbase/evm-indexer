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

    /// Include to skip additional on-chain data fetching
    #[clap(long, env)]
    pub skip_node_fetching: bool,
}

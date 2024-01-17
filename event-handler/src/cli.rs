use crate::config::ChainDataSource;

use url::Url;

#[derive(Debug, clap::Parser)]
pub struct Args {
    /// Source database connection string
    #[clap(long, env)]
    pub source_url: Url,

    /// Store database connection string
    #[clap(long, env)]
    pub store_url: Url,

    /// The node RPC API endpoint.
    #[clap(long, env)]
    pub node_url: Url,

    /// The log filter.
    #[clap(long, env, default_value = "debug")]
    pub log: String,

    /// Source of additional on-chain data
    #[clap(long, env, value_enum)]
    pub chain_source: ChainDataSource,

    /// BlockRange width for run-loop processing.
    #[clap(long, env, default_value = "1000")]
    pub page_size: i64,

    /// Source of additional on-chain data
    #[clap(long, env, default_value = "true")]
    pub fetch_node_data: bool,
}

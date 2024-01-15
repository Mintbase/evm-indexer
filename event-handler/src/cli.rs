use {reqwest::Url, std::path::PathBuf};

#[derive(Debug, clap::Parser)]
pub struct Args {
    /// Source database connection string
    #[clap(long, env)]
    pub source_url: String,

    /// Store database connection string
    #[clap(long, env)]
    pub store_url: String,

    /// The node RPC API endpoint.
    #[clap(long, env)]
    pub node_url: Url,

    /// The log filter.
    #[clap(long, env, default_value = "debug")]
    pub log: String,

    /// Path to the handler configuration file. This file should be in TOML
    /// format. For an example see
    /// ./event-handler/example.toml.
    #[clap(long, env)]
    pub config: PathBuf,
}

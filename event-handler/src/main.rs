extern crate event_handler;

use anyhow::Result;
use clap::Parser;
use event_handler::{cli::Args, config::HandlerConfig, processor::EventProcessor};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse_from(std::env::args());

    // Log configuration
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(args.log)
        .with_ansi(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let mut handler = EventProcessor::new(
        args.source_url.as_str(),
        args.store_url.as_str(),
        args.node_url.as_str(),
        HandlerConfig {
            chain_data_source: args.chain_source,
            page_size: args.page_size,
            fetch_node_data: !args.skip_node_fetching,
            db_schema: args.db_schema,
            uri_retry_blocks: args.uri_retry_blocks,
            batch_delay: args.node_batch_delay,
            token_avoid_list: args.token_avoid_list.into_iter().collect(),
        },
    )
    .expect("error constructing EventProcessor");
    let start_from = handler.store.get_processed_block() + 1;
    tracing::info!("beginning event processor from {start_from}");
    handler.run(start_from).await
}

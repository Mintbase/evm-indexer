extern crate event_handler;

use anyhow::Result;
use clap::Parser;
use event_handler::{cli::Args, config::HandlerConfig, processor::EventProcessor};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse_from(std::env::args());
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(args.log)
        .with_ansi(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    let config = HandlerConfig::from_path(&args.config);

    let mut handler = EventProcessor::new(
        &args.source_url,
        &args.store_url,
        args.node_url.as_str(),
        config,
    )
    .expect("error constructing EventProcessor");
    let start_from = handler.store.get_processed_block() + 1;
    tracing::info!("beginning event processor from {start_from}");
    handler.run(start_from).await
}

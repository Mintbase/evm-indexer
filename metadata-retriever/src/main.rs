mod app;
mod config;
mod routes;

use actix_web::{
    web::{self, Data},
    App, HttpServer, Responder,
};
use app::AppData;
use config::Config;
use eth::types::{Address, Message, NftId};
use google_cloud_pubsub::client::ClientConfig;
use std::env;

use crate::routes::RequestHandler;

fn partition_requests(payload: Vec<Message>) -> (Vec<Address>, Vec<(NftId, Option<String>)>) {
    let mut contracts = Vec::new();
    let mut tokens = Vec::new();
    for item in payload {
        match item {
            Message::Contract { address } => contracts.push(address),
            Message::Token {
                address,
                token_id,
                token_uri,
            } => tokens.push((NftId { address, token_id }, token_uri)),
        }
    }
    (contracts, tokens)
}

async fn pubsub_callback(data: web::Bytes, state: Data<AppData>) -> impl Responder {
    if let Ok(batch) = serde_json::from_slice::<Vec<Message>>(&data) {
        let (contracts, tokens) = partition_requests(batch);
        tracing::info!(
            "received {} contract and {} token messages for",
            contracts.len(),
            tokens.len()
        );
        state.process_request(&contracts).await;
        state.process_request(&tokens).await;
    } else {
        // Attempt to parse single entry
        if let Ok(message) = serde_json::from_slice::<Message>(&data) {
            let result = match message {
                Message::Contract { address } => state.process_request(&[address]).await,
                Message::Token {
                    address,
                    token_id,
                    token_uri,
                } => {
                    state
                        .process_request(&[(NftId { address, token_id }, token_uri)])
                        .await
                }
            };
            return result;
        } else {
            tracing::warn!("Received unrecognized message format {:?}", data);
        }
    }
    // TODO - return an actual status to sender.
    "Message received and processed successfully".to_string()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("PUBSUB_EMULATOR_HOST", "localhost:8681");
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(env::var("RUST_LOGS").unwrap_or("info".to_string()))
        .with_ansi(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    let subscriber = app::build_subscription(ClientConfig::default())
        .await
        .expect("Couldn't build subscriber");
    let config = Config::from_env().expect("Config error!");
    let state = AppData::new(subscriber.clone(), config).await;
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(state.clone()))
            .service(web::resource("/pubsub_callback").route(web::post().to(pubsub_callback)))
            .wrap(tracing_actix_web::TracingLogger::default())
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}

mod app;
mod config;
mod routes;

use actix_web::{
    web::{self, Data},
    App, HttpResponse, HttpServer, Responder,
};
use app::AppData;
use config::Config;
use eth::types::{Address, Message, NftId};
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
        tracing::warn!(
            "received batch request with {} contract and {} token messages for",
            contracts.len(),
            tokens.len()
        );
        state.process_request(&contracts).await;
        state.process_request(&tokens).await;
        HttpResponse::Ok().body("Batch processed successfully")
    } else {
        let json_data = serde_json::from_slice::<serde_json::Value>(&data);
        tracing::info!("received single message with {:?} bytes", data.len());
        if let Ok(message) = serde_json::from_slice::<Message>(&data) {
            match message {
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
            }
        } else {
            tracing::warn!("Received unrecognized message format {:?}", data);
            HttpResponse::BadRequest().body(format!(
                "Received unrecognized message format {json_data:?}"
            ))
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    // Default logs set to info (use RUST_LOGS to override).
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(
            env::var("RUST_LOGS")
                .unwrap_or("info,metadata_retriever=debug,actix_web=warn".to_string()),
        )
        .with_ansi(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // let client_config = ClientConfig::default().with_auth().await.unwrap();

    // let subscription = app::build_subscription(client_config)
    //     .await
    //     .expect("Couldn't build subscriber");
    let config = Config::from_env().expect("Config error!");
    let state = AppData::new(
        // subscription.clone(),
        config,
    )
    .await;
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(state.clone()))
            .service(web::resource("/pubsub_callback").route(web::post().to(pubsub_callback)))
        // .wrap(tracing_actix_web::TracingLogger::default())
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}

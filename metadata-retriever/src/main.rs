mod app;
mod config;
mod routes;

use actix_web::{
    web::{self, Data},
    App, HttpResponse, HttpServer, Responder,
};
use app::AppData;
use config::Config;
use eth::types::{Message, NftId};
use std::env;

use crate::routes::RequestHandler;
async fn pubsub_callback(data: web::Bytes, state: Data<AppData>) -> impl Responder {
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

    let config = Config::from_env().expect("Config error!");
    let state = AppData::new(config).await;
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(state.clone()))
            // 2Mb
            .app_data(web::PayloadConfig::new(2 * 1024 * 1024))
            .service(web::resource("/pubsub_callback").route(web::post().to(pubsub_callback)))
    })
    .workers(25)
    .bind("0.0.0.0:8080")?
    .run()
    .await
}

use actix_web::{
    middleware::Logger,
    web::{self},
    App, HttpServer,
};
use api::routes::{contracts, tokens, AppState};
use data_store::models::{ContractAbi, Nft, TokenContract};
use utoipa::OpenApi;
use utoipa_swagger_ui::{SwaggerUi, Url};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    #[derive(OpenApi)]
    #[openapi(
        paths(
            tokens::tokens,
            tokens::tokens_by_owner,
            tokens::tokens_by_minter,
            contracts::contract_abi,
            contracts::contract
        ),
        components(schemas(tokens::AddressPayload, ContractAbi, Nft, TokenContract))
    )]
    struct ApiDoc;

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    dotenvy::dotenv().ok();
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(AppState::new(
                std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            )))
            // .service(web::scope("/api").service(tokens::tokens))
            .service(SwaggerUi::new("/swagger-ui/{_:.*}").urls(vec![(
                Url::new("mainnet", "/api-docs/mainnet.json"),
                ApiDoc::openapi(),
            )]))
            .service(tokens::tokens_by_owner)
            .service(tokens::tokens)
            .service(tokens::tokens_by_minter)
            .service(contracts::contract_abi)
            .service(contracts::contract)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

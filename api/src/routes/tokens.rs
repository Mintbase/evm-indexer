use crate::routes::AppState;

use actix_web::{
    get, post,
    web::{self},
    HttpResponse, Responder,
};
use data_store::store::DataStore;
use eth::types::Address;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AddressPayload {
    #[schema(example = 0, required = true)]
    id: usize,
    #[schema(
        example = "0x92BE2F02C94D214F8D38ECE700385471D9A66C0A",
        required = true
    )]
    address: String,
}

#[utoipa::path(
    request_body = AddressPayload,
    responses(
        (status = 200, description = "POST version of get tokens by owner.", body = Vec<Nft>),
        (status = 400, description = "Invalid payload"),
        (status = 500, description = "Internal server error."),
    )
)]
/// This isn't necessary, just an example use of post with payload
#[post("/tokens_by_owner")]
pub async fn tokens_by_owner(
    data: web::Data<AppState>,
    task: web::Json<AddressPayload>,
) -> impl Responder {
    // TODO use store as part of appState
    let mut store = DataStore::new(&data.db_url, &data.db_schema).expect("connect to store");
    let owner_address = match Address::from_str(&task.address) {
        Ok(address) => address,
        Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
    };
    let nfts = store.get_nfts_by_owner(owner_address);

    HttpResponse::Ok().json(nfts)
}

#[utoipa::path(
    responses(
        (status = 200, description = "Get tokens by owner.", body = Vec<Nft>),
        (status = 400, description = "Invalid payload"),
        (status = 500, description = "Internal server error."),
    )
)]
#[get("/tokens/owner/{address}")]
pub async fn tokens(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let mut store = DataStore::new(&data.db_url, &data.db_schema).expect("connect to store");
    let owner = match Address::from_str(&path.into_inner()) {
        Ok(address) => address,
        Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
    };
    HttpResponse::Ok().json(store.get_nfts_by_owner(owner))
}

#[utoipa::path(
    responses(
        (status = 200, description = "Get tokens by minter.", body = Vec<Nft>),
        (status = 400, description = "Invalid payload"),
        (status = 500, description = "Internal server error."),
    )
)]
#[get("/tokens/minter/{address}")]
pub async fn tokens_by_minter(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let mut store = DataStore::new(&data.db_url, &data.db_schema).expect("connect to store");
    let minter = match Address::from_str(&path.into_inner()) {
        Ok(address) => address,
        Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
    };
    HttpResponse::Ok().json(store.get_nfts_by_minter(minter))
}

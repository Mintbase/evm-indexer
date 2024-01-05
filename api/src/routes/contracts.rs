use crate::routes::AppState;

use actix_web::{
    get,
    web::{self},
    HttpResponse, Responder,
};
use data_store::store::DataStore;
use eth::types::Address;
use std::str::FromStr;

#[utoipa::path(
    responses(
        (status = 200, description = "Get contract abi(s) by address.", body = ContractAbi),
        (status = 400, description = "Invalid payload"),
        (status = 500, description = "Internal server error."),
    )
)]
#[get("/contract_abi/{address}")]
pub async fn contract_abi(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    // TODO use store as part of appState
    let mut store = DataStore::new(&data.db_url).expect("connect to store");
    let contract_address = match Address::from_str(&path.into_inner()) {
        Ok(address) => address,
        Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
    };
    let abi = store.get_contract_abi(contract_address);
    HttpResponse::Ok().json(abi)
}

#[utoipa::path(
    responses(
        (status = 200, description = "Get Nft project by contract address.", body = TokenContract),
        (status = 400, description = "Invalid payload"),
        (status = 500, description = "Internal server error."),
    )
)]
#[get("/contract/{address}")]
pub async fn contract(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    // TODO use store as part of appState
    let mut store = DataStore::new(&data.db_url).expect("connect to store");
    let contract_address = match Address::from_str(&path.into_inner()) {
        Ok(address) => address,
        Err(err) => return HttpResponse::BadRequest().body(err.to_string()),
    };
    let abi = store.load_contract(contract_address);
    HttpResponse::Ok().json(abi)
}

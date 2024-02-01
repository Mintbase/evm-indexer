pub mod contracts;
pub mod tokens;

pub struct AppState {
    db_url: String,
    db_schema: String,
}

impl AppState {
    pub fn new(db_url: String, db_schema: String) -> Self {
        Self { db_url, db_schema }
    }
}

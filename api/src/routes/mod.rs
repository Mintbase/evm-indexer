pub mod contracts;
pub mod tokens;

pub struct AppState {
    db_url: String,
}

impl AppState {
    pub fn new(db_url: String) -> Self {
        Self { db_url }
    }
}

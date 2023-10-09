pub mod db_reader;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_reader::diesel::BlockRange;
    use dotenv::dotenv;

    static TEST_DB_URL: &str = "postgresql://postgres:postgres@localhost:5432/arak";

    #[test]
    fn e2e_event_retrieval() {
        dotenv().ok();
        let db_url = std::env::var("DB_URL").unwrap_or(TEST_DB_URL.to_string());
        let mut pg_client =
            db_reader::diesel::EventSource::new(&db_url).expect("Failed to connect to DB");
        let block = 10_000_246;
        assert!(pg_client
            .get_events_for_block_range(BlockRange {
                start: block,
                end: block + 1,
            })
            .is_ok());
    }
}

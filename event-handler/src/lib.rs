pub mod handler;
mod models;
mod schema;
mod store;
#[cfg(test)]
mod tests {
    use crate::handler::EventHandler;

    static TEST_SOURCE_URL: &str = "postgresql://postgres:postgres@localhost:5432/postgres";
    static TEST_STORE_URL: &str = "postgresql://postgres:postgres@localhost:5432/postgres";
    #[test]
    fn event_processing() {
        let handler = EventHandler::new(TEST_SOURCE_URL, TEST_STORE_URL).unwrap();
        assert!(handler.process_events_for_block(10006884).is_ok());
    }
}

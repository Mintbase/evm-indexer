/// Where chain data should be retrieved from
pub enum ChainDataSource {
    Database,
    Node,
}

pub struct HandlerConfig {
    /// Source of chain data (blocks & transactions)
    pub chain_data_source: ChainDataSource,
    /// How wide of block ranges to process at once.
    pub page_size: i64,
    /// whether this service or another should be responsible for fetching metadata
    pub fetch_metadata: bool,
}

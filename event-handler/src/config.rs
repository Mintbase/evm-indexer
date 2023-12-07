/// Where chain data should be retrieved from
pub enum ChainDataSource {
    Database,
    Node,
}

pub struct HandlerConfig {
    /// Source of chain data (blocks & transactions)
    pub chain_data_source: ChainDataSource,
}

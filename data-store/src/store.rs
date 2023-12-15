use crate::{models::*, schema::*};

use anyhow::{Context, Result};
use bigdecimal::{BigDecimal, Zero};
use diesel::{
    pg::PgConnection,
    prelude::*,
    r2d2::{ConnectionManager, Pool, PooledConnection},
    update, RunQueryDsl,
};
use eth::types::{Address, BlockData, NftId, TxDetails};
use event_retriever::db_reader::models::EventBase;
use scheduled_thread_pool::ScheduledThreadPool;
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
pub struct DataStore {
    pool: Pool<ConnectionManager<PgConnection>>,
}

fn handle_insert_result(result: QueryResult<usize>, expected_updates: usize, context: String) {
    match result {
        Ok(value) => {
            if value != expected_updates {
                tracing::warn!(
                    "unexpected update number for {} expected {} got {}",
                    context,
                    expected_updates,
                    value
                )
            }
        }
        Err(err) => {
            panic!("unhandled query result error on {}: {:?}", context, err)
        }
    }
}

fn handle_query_result<T>(result: QueryResult<T>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => {
            panic!("unhandled query result error: {:?}", err)
        }
    }
}

impl DataStore {
    pub fn new(connection: &str) -> Result<Self> {
        let pool_size = std::env::var("STORE_POOL_SIZE")
            .unwrap_or("20".to_string())
            .parse::<u32>()
            .context("parse pool_size")?;
        let num_threads = std::env::var("STORE_NUM_THREADS")
            .unwrap_or("10".to_string())
            .parse::<usize>()
            .context("parse num_threads")?;
        Ok(Self {
            pool: Self::get_connection_pool(connection, pool_size, num_threads)?,
        })
    }

    fn get_connection(&self) -> PooledConnection<ConnectionManager<PgConnection>> {
        self.pool.get().expect("failed to get connection from pool")
    }

    pub fn insert_metadata(&mut self, token: &NftId, content: Value) {
        let record = NftMetadata::from(content);
        let uid = record.uid;

        let result = diesel::insert_into(nft_metadata::dsl::nft_metadata)
            .values(record)
            .on_conflict(nft_metadata::uid)
            .do_nothing()
            .execute(&mut self.get_connection());

        // one of the following two tables will be updated (depending on token type.)
        update(nfts::dsl::nfts)
            .set(nfts::metadata_id.eq::<Vec<u8>>(uid.into()))
            .filter(nfts::contract_address.eq(&token.db_address()))
            .filter(nfts::token_id.eq(&token.db_token_id()))
            .execute(&mut self.get_connection())
            .expect("Failed to execute nft update");
        update(erc1155s::dsl::erc1155s)
            .set(erc1155s::metadata_id.eq::<Vec<u8>>(uid.into()))
            .filter(erc1155s::contract_address.eq(&token.db_address()))
            .filter(erc1155s::token_id.eq(&token.db_token_id()))
            .execute(&mut self.get_connection())
            .expect("Failed to execute erc1155 update");
        handle_insert_result(result, 1, format!("insert_metadata: {}", token))
    }

    pub fn insert_uri(&mut self, token: &NftId, uri: Option<String>) {
        // Try Erc721:
        update(nfts::dsl::nfts)
            .set(nfts::token_uri.eq(uri.clone()))
            .filter(nfts::contract_address.eq(&token.db_address()))
            .filter(nfts::token_id.eq(&token.db_token_id()))
            .execute(&mut self.get_connection())
            .expect("Failed to execute nft update");
        // Try Erc1155
        update(erc1155s::dsl::erc1155s)
            .set(erc1155s::token_uri.eq(uri.clone()))
            .filter(erc1155s::contract_address.eq(&token.db_address()))
            .filter(erc1155s::token_id.eq(&token.db_token_id()))
            .execute(&mut self.get_connection())
            .expect("Failed to execute erc1155 update");
    }

    pub fn insert_contract_details(
        &mut self,
        address: Address,
        name: Option<String>,
        symbol: Option<String>,
    ) {
        update(token_contracts::dsl::token_contracts)
            .set((
                token_contracts::name.eq(name),
                token_contracts::symbol.eq(symbol),
            ))
            .filter(token_contracts::address.eq::<&Vec<u8>>(&address.into()))
            .execute(&mut self.get_connection())
            .expect("Failed to execute contract update");
    }

    pub fn insert_contract_abi(&mut self, abi: ContractAbi) {
        let result = diesel::insert_into(contract_abis::dsl::contract_abis)
            .values(abi.clone())
            .on_conflict(contract_abis::address)
            .do_nothing()
            .execute(&mut self.get_connection());
        handle_insert_result(result, 1, format!("insert_abi: {}", abi.address))
    }

    pub fn load_nft(&mut self, token: &NftId) -> Option<Nft> {
        let result = nfts::dsl::nfts
            .filter(nfts::contract_address.eq(&token.db_address()))
            .filter(nfts::token_id.eq(&token.db_token_id()))
            .first(&mut self.get_connection())
            .optional();
        handle_query_result(result)
    }

    pub fn load_approval(&mut self, id: &ApprovalId) -> Option<ApprovalForAll> {
        let result = approval_for_all::dsl::approval_for_all
            .filter(approval_for_all::contract_address.eq::<&Vec<u8>>(&id.contract_address.into()))
            .filter(approval_for_all::owner.eq::<&Vec<u8>>(&id.owner.into()))
            .first(&mut self.get_connection())
            .optional();
        handle_query_result(result)
    }

    pub fn load_erc1155(&mut self, token: &NftId) -> Option<Erc1155> {
        let result = erc1155s::dsl::erc1155s
            .filter(erc1155s::contract_address.eq(&token.db_address()))
            .filter(erc1155s::token_id.eq(&token.db_token_id()))
            .first(&mut self.get_connection())
            .optional();
        handle_query_result(result)
    }

    pub fn load_erc1155_owner(&mut self, token: &NftId, address: Address) -> Option<Erc1155Owner> {
        let result = erc1155_owners::dsl::erc1155_owners
            .filter(erc1155_owners::contract_address.eq(&token.db_address()))
            .filter(erc1155_owners::token_id.eq(&token.db_token_id()))
            .filter(erc1155_owners::owner.eq::<&Vec<u8>>(&address.into()))
            .first(&mut self.get_connection())
            .optional();
        handle_query_result(result)
    }

    pub fn load_contract(&mut self, address: Address) -> Option<TokenContract> {
        let result = token_contracts::dsl::token_contracts
            .filter(token_contracts::address.eq::<&Vec<u8>>(&address.into()))
            .first(&mut self.get_connection())
            .optional();
        handle_query_result(result)
    }

    pub fn get_processed_block(&mut self) -> i64 {
        // TODO - use more sophisticated reasoning here:
        //  https://github.com/Mintbase/evm-indexer/issues/102
        blocks::dsl::blocks
            .select(diesel::dsl::max(blocks::number))
            .limit(1)
            .get_result(&mut self.get_connection())
            .unwrap_or(Some(0))
            .unwrap_or(0)
    }

    pub fn get_nfts_by_owner(&mut self, owner: Address) -> Vec<Nft> {
        let result = nfts::dsl::nfts
            .filter(nfts::owner.eq::<Vec<u8>>(owner.into()))
            .load::<Nft>(&mut self.get_connection());
        handle_query_result(result)
    }

    pub fn get_nfts_by_minter(&mut self, minter: Address) -> Vec<Nft> {
        let result = nfts::dsl::nfts
            .filter(nfts::minter.eq::<Vec<u8>>(minter.into()))
            .load::<Nft>(&mut self.get_connection());
        handle_query_result(result)
    }

    pub fn get_contract_abi(&mut self, address: Address) -> Option<ContractAbi> {
        let result = contract_abis::dsl::contract_abis
            .filter(contract_abis::address.eq::<Vec<u8>>(address.into()))
            .first(&mut self.get_connection())
            .optional();
        // .load::<ContractAbi>(&mut self.get_connection());
        handle_query_result(result)
    }

    pub fn save_transactions(&mut self, txs: Vec<Transaction>) {
        // These inserts must be broken into chunks because of:
        // DatabaseError(UnableToSendCommand, "number of parameters must be between 0 and 65535\n")
        let chunk_size = 10_000;
        tracing::info!(
            "saving {} EVM transactions over {} SQL transactions",
            txs.len(),
            (txs.len() / chunk_size) + 1
        );
        for chunk in txs.chunks(chunk_size) {
            let expected_inserts = chunk.len();
            let result = diesel::insert_into(transactions::dsl::transactions)
                .values(chunk.to_vec())
                .on_conflict((transactions::block_number, transactions::index))
                .do_nothing()
                .execute(&mut self.get_connection());
            handle_insert_result(result, expected_inserts, "save_transactions".to_string())
        }
    }

    pub fn save_blocks(&mut self, blocks: Vec<BlockData>) {
        let chunk_size = 10_000;
        tracing::info!(
            "saving {} EVM blocks over {} SQL transactions",
            blocks.len(),
            (blocks.len() / chunk_size) + 1
        );
        for chunk in blocks.chunks(chunk_size) {
            let expected_inserts = chunk.len();
            let result = diesel::insert_into(blocks::dsl::blocks)
                .values(chunk.iter().map(Block::new).collect::<Vec<_>>())
                .on_conflict(blocks::number)
                .do_nothing()
                .execute(&mut self.get_connection());
            handle_insert_result(result, expected_inserts, "save_blocks".to_string())
        }
    }

    pub fn save_nft(&mut self, nft: Nft) {
        let token_id = nft.id();
        let result = diesel::insert_into(nfts::dsl::nfts)
            .values(nft.clone())
            .on_conflict((nfts::contract_address, nfts::token_id))
            .do_update()
            .set(nft)
            .execute(&mut self.get_connection());
        handle_insert_result(result, 1, format!("save_nft: {}", token_id))
    }

    pub async fn save_nfts(&mut self, nft_updates: Vec<Nft>) {
        tracing::info!("saving {} nfts", nft_updates.len());
        let mut tasks: Vec<tokio::task::JoinHandle<()>> = vec![];

        for nft in nft_updates {
            let pool = self.pool.clone();
            tasks.push(tokio::spawn(async move {
                let conn: &mut PooledConnection<ConnectionManager<PgConnection>> =
                    &mut pool.get().unwrap();
                Self::upsert_nft(conn, nft)
            }))
        }
        let errors: Vec<tokio::task::JoinError> = futures::future::join_all(tasks)
            .await
            .into_iter()
            .filter_map(|res| res.err())
            .collect();

        if !errors.is_empty() {
            tracing::error!(
                "failed to update {} nfts with errors {:?}",
                errors.len(),
                errors
            );
        }
    }

    pub async fn save_erc1155s(&mut self, updates: Vec<Erc1155>) {
        tracing::info!("saving {} erc1155s", updates.len());
        let mut tasks: Vec<tokio::task::JoinHandle<()>> = vec![];

        for token in updates {
            let pool = self.pool.clone();
            tasks.push(tokio::spawn(async move {
                let conn: &mut PooledConnection<ConnectionManager<PgConnection>> =
                    &mut pool.get().unwrap();
                Self::upsert_erc1155(conn, token)
            }))
        }
        let errors: Vec<tokio::task::JoinError> = futures::future::join_all(tasks)
            .await
            .into_iter()
            .filter_map(|res| res.err())
            .collect();

        if !errors.is_empty() {
            tracing::error!(
                "failed to update {} erc1155 with errors {:?}",
                errors.len(),
                errors
            );
        }
    }

    pub async fn save_erc1155_owners(&mut self, owner_updates: Vec<Erc1155Owner>) {
        tracing::info!("saving {} owners", owner_updates.len());
        let mut tasks: Vec<tokio::task::JoinHandle<()>> = vec![];

        for owner in owner_updates {
            let pool = self.pool.clone();
            tasks.push(tokio::spawn(async move {
                let conn: &mut PooledConnection<ConnectionManager<PgConnection>> =
                    &mut pool.get().unwrap();
                Self::upsert_erc1155_owner(conn, owner)
            }))
        }
        let errors: Vec<tokio::task::JoinError> = futures::future::join_all(tasks)
            .await
            .into_iter()
            .filter_map(|res| res.err())
            .collect();

        if !errors.is_empty() {
            tracing::error!(
                "failed to update {} erc1155 owners with errors {:?}",
                errors.len(),
                errors
            );
        }
    }

    pub async fn save_approval_for_alls(&mut self, approvals: Vec<ApprovalForAll>) {
        tracing::info!("saving {} approvals", approvals.len());
        let mut tasks: Vec<tokio::task::JoinHandle<()>> = vec![];

        for approval in approvals {
            let pool = self.pool.clone();
            tasks.push(tokio::spawn(async move {
                let conn: &mut PooledConnection<ConnectionManager<PgConnection>> =
                    &mut pool.get().unwrap();
                Self::upsert_approval_for_all(conn, approval)
            }))
        }
        let errors: Vec<tokio::task::JoinError> = futures::future::join_all(tasks)
            .await
            .into_iter()
            .filter_map(|res| res.err())
            .collect();

        if !errors.is_empty() {
            tracing::error!(
                "failed to update {} approval_for_alls with errors {:?}",
                errors.len(),
                errors
            );
        }
    }

    pub fn save_contract(&mut self, contract: TokenContract) {
        let contract_address = contract.address;
        let result = diesel::insert_into(token_contracts::dsl::token_contracts)
            .values(contract.clone())
            .on_conflict(token_contracts::address)
            .do_update()
            .set(contract)
            .execute(&mut self.get_connection());
        handle_insert_result(result, 1, format!("save_contract {:?}", contract_address))
    }

    /// This method, as opposed to its singular counter part may be used under the assumption
    /// that the contracts are not being updated during event handling.
    pub fn save_contracts(&mut self, contracts: Vec<TokenContract>) {
        let expected_inserts = contracts.len();
        tracing::info!("saving {} contracts", expected_inserts);
        let result = diesel::insert_into(token_contracts::dsl::token_contracts)
            .values(contracts)
            .on_conflict(token_contracts::address)
            .do_nothing()
            .execute(&mut self.get_connection());
        handle_insert_result(result, expected_inserts, "save_contracts".to_string())
    }

    pub fn upsert_approval_for_all(
        conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
        approval: ApprovalForAll,
    ) {
        let result = diesel::insert_into(approval_for_all::dsl::approval_for_all)
            .values(approval.clone())
            .on_conflict((approval_for_all::contract_address, approval_for_all::owner))
            .do_update()
            .set(approval.clone())
            .execute(conn);
        handle_insert_result(result, 1, format!("set_approval_for_all {:?}", approval))
    }

    pub fn load_or_initialize_nft(
        &mut self,
        base: &EventBase,
        nft_id: &NftId,
        tx: &TxDetails,
    ) -> Nft {
        match self.load_nft(nft_id) {
            Some(nft) => nft,
            None => {
                tracing::debug!("new nft {:?}", nft_id);
                Nft::new(base, nft_id, tx)
            }
        }
    }

    pub fn load_or_initialize_erc1155(
        &mut self,
        base: &EventBase,
        nft_id: &NftId,
        tx: &TxDetails,
    ) -> Erc1155 {
        match self.load_erc1155(nft_id) {
            Some(nft) => nft,
            None => {
                tracing::debug!("new erc1155 {:?}", nft_id);
                Erc1155::new(base, nft_id, tx)
            }
        }
    }

    pub fn load_or_initialize_erc1155_owner(
        &mut self,
        base: &EventBase,
        nft_id: &NftId,
        address: Address,
    ) -> Erc1155Owner {
        match self.load_erc1155_owner(nft_id, address) {
            Some(nft) => nft,
            None => Erc1155Owner {
                contract_address: base.contract_address,
                token_id: nft_id.token_id.into(),
                owner: address,
                balance: BigDecimal::zero(),
            },
        }
    }

    pub fn load_or_initialize_approval(&mut self, approval_id: &ApprovalId) -> ApprovalForAll {
        match self.load_approval(approval_id) {
            Some(approval) => approval,
            None => ApprovalForAll {
                contract_address: approval_id.contract_address,
                owner: approval_id.owner,
                operator: Address::zero(),
                approved: false,
                last_update_block: 0,
                last_update_log_index: 0,
            },
        }
    }

    fn upsert_nft(conn: &mut PooledConnection<ConnectionManager<PgConnection>>, nft: Nft) {
        let token_id = nft.id();
        let result = diesel::insert_into(nfts::dsl::nfts)
            .values(nft.clone())
            .on_conflict((nfts::contract_address, nfts::token_id))
            .do_update()
            .set(nft)
            .execute(conn);
        handle_insert_result(result, 1, format!("save_nft: {}", token_id))
    }

    fn upsert_erc1155(conn: &mut PooledConnection<ConnectionManager<PgConnection>>, nft: Erc1155) {
        let token_id = nft.id();
        let result = diesel::insert_into(erc1155s::dsl::erc1155s)
            .values(nft.clone())
            .on_conflict((erc1155s::contract_address, erc1155s::token_id))
            .do_update()
            .set(nft)
            .execute(conn);
        handle_insert_result(result, 1, format!("save_erc1155: {}", token_id))
    }

    fn upsert_erc1155_owner(
        conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
        owner: Erc1155Owner,
    ) {
        let owner_address = owner.owner;
        let result = diesel::insert_into(erc1155_owners::dsl::erc1155_owners)
            .values(owner.clone())
            .on_conflict((
                erc1155_owners::contract_address,
                erc1155_owners::token_id,
                erc1155_owners::owner,
            ))
            .do_update()
            .set(owner)
            .execute(conn);
        handle_insert_result(result, 1, format!("save_erc1155_owner {:?}", owner_address))
    }

    fn get_connection_pool(
        db_url: &str,
        pool_size: u32,
        num_threads: usize,
    ) -> Result<Pool<ConnectionManager<PgConnection>>> {
        let manager = ConnectionManager::<PgConnection>::new(db_url);
        Pool::builder()
            .max_size(pool_size) // Should be a configurable env var
            .test_on_check_out(true)
            .thread_pool(Arc::new(ScheduledThreadPool::new(num_threads)))
            .build(manager)
            .context("build connection pool")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::contract_abis;
    use diesel::{QueryDsl, RunQueryDsl};
    use eth::types::{Address, Bytes32, TxDetails, U256};
    use event_retriever::db_reader::models::EventBase;

    static TEST_STORE_URL: &str = "postgresql://postgres:postgres@localhost:5432/store";

    fn get_new_store() -> DataStore {
        let mut store = DataStore::new(TEST_STORE_URL).unwrap();
        store.clear_tables();
        store
    }

    impl DataStore {
        pub fn clear_tables(&mut self) {
            // Delete before contracts because of foreign key constraint!
            // TODO - add other foreign key (erc721/nfts).
            diesel::delete(erc1155_owners::dsl::erc1155_owners)
                .execute(&mut self.get_connection())
                .unwrap();
            diesel::delete(erc1155s::dsl::erc1155s)
                .execute(&mut self.get_connection())
                .unwrap();
            diesel::delete(nfts::dsl::nfts)
                .execute(&mut self.get_connection())
                .unwrap();
            diesel::delete(approval_for_all::dsl::approval_for_all)
                .execute(&mut self.get_connection())
                .unwrap();
            diesel::delete(contract_abis::dsl::contract_abis)
                .execute(&mut self.get_connection())
                .unwrap();
            diesel::delete(token_contracts::dsl::token_contracts)
                .execute(&mut self.get_connection())
                .unwrap();
            diesel::delete(transactions::dsl::transactions)
                .execute(&mut self.get_connection())
                .unwrap();
            diesel::delete(blocks::dsl::blocks)
                .execute(&mut self.get_connection())
                .unwrap();
            diesel::delete(nft_metadata::dsl::nft_metadata)
                .execute(&mut self.get_connection())
                .unwrap();
        }
    }

    fn test_event_base() -> EventBase {
        EventBase {
            block_number: 1,
            log_index: 2,
            transaction_index: 3,
            contract_address: Address::from(1),
        }
    }

    #[test]
    fn save_transactions() {
        let mut store = get_new_store();
        let details = TxDetails {
            hash: Bytes32::from(1),
            from: Address::from(1),
            to: Some(Address::from(2)),
        };
        // First call should not panic or log
        store.save_transactions(vec![
            Transaction::new(1, 2, details),
            Transaction::new(3, 4, details),
        ]);

        assert_eq!(
            Ok(2),
            transactions::dsl::transactions
                .count()
                .get_result(&mut store.pool.get().unwrap())
        );

        // This call will do nothing.
        store.save_transactions(vec![
            // Notice same (block, index) = (1, 2) as above.
            Transaction::new(1, 2, details),
        ]);
        assert_eq!(
            Ok(2),
            transactions::dsl::transactions
                .count()
                .get_result(&mut store.pool.get().unwrap())
        );
    }

    #[test]
    fn save_blocks() {
        let mut store = get_new_store();
        let blocks = vec![
            BlockData {
                number: 1,
                ..Default::default()
            },
            BlockData {
                number: 2,
                ..Default::default()
            },
            BlockData {
                number: 3,
                ..Default::default()
            },
            BlockData {
                number: 3,
                ..Default::default()
            },
        ];
        store.save_blocks(blocks);
        assert_eq!(
            Ok(3),
            blocks::dsl::blocks
                .count()
                .get_result(&mut store.pool.get().unwrap())
        );
    }

    #[tokio::test]
    async fn save_and_load_nft() {
        let mut store = get_new_store();
        let base = test_event_base();
        let token = NftId {
            address: base.contract_address,
            token_id: U256::from(123),
        };
        let tx = TxDetails {
            hash: Bytes32::from(1),
            from: Address::from(1),
            to: Some(Address::from(2)),
        };
        let nft = Nft::new(&base, &token, &tx);
        store.save_nfts(vec![nft.clone()]).await;
        assert_eq!(store.load_nft(&token).unwrap(), nft);
    }

    #[test]
    fn load_or_initialize_nft() {
        let mut store = get_new_store();
        let base = test_event_base();
        let token = NftId {
            address: base.contract_address,
            token_id: U256::from(123),
        };
        let tx = TxDetails {
            hash: Bytes32::from(1),
            from: Address::from(1),
            to: Some(Address::from(2)),
        };
        assert_eq!(
            store.load_or_initialize_nft(&base, &token, &tx),
            Nft::new(&base, &token, &tx)
        );
    }

    #[test]
    fn save_and_load_contract() {
        let mut store = get_new_store();
        let base = test_event_base();
        let contract = TokenContract::from_event_base(&base);
        assert!(store.load_contract(base.contract_address).is_none());
        store.save_contract(contract);
        assert!(store.load_contract(base.contract_address).is_some());
    }

    fn setup_store_with_nft() -> (DataStore, NftId, NftId) {
        let mut store = get_new_store();
        let mut base = test_event_base();
        let erc721_id = NftId {
            address: base.contract_address,
            token_id: U256::from(123),
        };
        let erc1155_id = NftId {
            address: base.contract_address,
            token_id: U256::from(456789),
        };
        let tx = TxDetails {
            hash: Bytes32::from(1),
            from: Address::from(1),
            to: Some(Address::from(2)),
        };
        // We have to add the contract because of the Foreign Key constraint Erc1155 >- Contracts.
        store.save_contract(TokenContract::from_event_base(&base));

        let nft = Nft::new(&base, &erc721_id, &tx);
        base.block_number += 1;
        let erc1155 = Erc1155::new(&base, &erc1155_id, &tx);
        store.save_nft(nft.clone());
        DataStore::upsert_erc1155(&mut store.get_connection(), erc1155.clone());
        (store, erc721_id, erc1155_id)
    }

    #[test]
    fn insert_metadata() {
        // Setup:
        let (mut store, token_id, _) = setup_store_with_nft();

        // Token has no metadata yet.
        assert!(store.load_nft(&token_id).unwrap().metadata_id.is_none());
        // content
        let content = serde_json::json!("My JSON document!");
        store.insert_metadata(&token_id, content.clone());

        let token = store.load_nft(&token_id).unwrap();
        assert!(token.metadata_id.is_some());

        let result = nft_metadata::table
            .filter(nft_metadata::uid.eq::<Vec<u8>>(token.metadata_id.unwrap()))
            .load::<NftMetadata>(&mut store.get_connection())
            .unwrap();
        assert_eq!(result, [NftMetadata::from(content)]);
    }

    #[test]
    fn insert_token_uri() {
        // Setup:
        let (mut store, erc721_id, erc1155_id) = setup_store_with_nft();

        // Token has no uri yet.
        assert!(store.load_nft(&erc721_id).unwrap().token_uri.is_none());
        let uri = Some("string".to_string());
        store.insert_uri(&erc721_id, uri.clone());
        assert_eq!(store.load_nft(&erc721_id).unwrap().token_uri, uri);
        assert!(store.load_erc1155(&erc1155_id).unwrap().token_uri.is_none());
        store.insert_uri(&erc1155_id, uri.clone());
        assert_eq!(store.load_erc1155(&erc1155_id).unwrap().token_uri, uri);
    }

    #[test]
    fn add_contract_details() {
        // Setup -- adds the contract.
        let (mut store, token_id, _) = setup_store_with_nft();
        let contract = store.load_contract(token_id.address).unwrap();
        assert!(contract.name.is_none());
        assert!(contract.symbol.is_none());

        let name = Some("Name".to_string());
        let symbol = Some("Symbol".to_string());
        store.insert_contract_details(token_id.address, name.clone(), symbol.clone());

        let contract = store.load_contract(token_id.address).unwrap();
        assert_eq!(contract.name, name);
        assert_eq!(contract.symbol, symbol);
    }

    #[test]
    fn add_contract_abi() {
        let mut store = get_new_store();
        let address = Address::zero();

        let before = contract_abis::table
            .filter(contract_abis::address.eq::<Vec<u8>>(address.into()))
            .load::<ContractAbi>(&mut store.get_connection())
            .unwrap();
        assert!(before.is_empty());

        let contract_abi = ContractAbi {
            address,
            abi: Some(serde_json::json!("Ultimate ABI")),
        };
        store.insert_contract_abi(contract_abi.clone());

        let after = contract_abis::table
            .filter(contract_abis::address.eq::<Vec<u8>>(address.into()))
            .load::<ContractAbi>(&mut store.get_connection())
            .unwrap();
        assert_eq!(after, [contract_abi]);
    }
}

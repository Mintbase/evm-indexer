use crate::db_reader::{
    models::{
        db::{
            DbApprovalForAll, DbErc1155TransferBatch, DbErc1155TransferSingle, DbErc1155Uri,
            DbErc721Approval, DbErc721Transfer, EvmEventTable,
        },
        merge_sorted_iters, EventMeta, NftEvent,
    },
    schema::{
        self, approval_for_all::dsl::approval_for_all,
        erc1155_transfer_single::dsl::erc1155_transfer_single, erc1155_uri::dsl::erc1155_uri,
        erc721_approval::dsl::erc721_approval, erc721_transfer::dsl::erc721_transfer,
    },
};
use anyhow::{Context, Result};
use diesel::{pg::PgConnection, prelude::*, sql_query, sql_types::BigInt, Connection, RunQueryDsl};

#[derive(Clone, Copy, Debug)]
pub struct BlockRange {
    pub start: i64,
    pub end: i64,
}

pub struct EventSource {
    client: PgConnection,
}

impl EventSource {
    pub fn new(connection: &str) -> Result<Self> {
        Ok(Self {
            client: Self::establish_connection(connection)?,
        })
    }

    fn establish_connection(db_url: &str) -> Result<PgConnection> {
        PgConnection::establish(db_url).context("Error connecting to Diesel Client")
    }

    pub fn get_events_for_block(&mut self, block: i64) -> Result<Vec<NftEvent>> {
        self.get_events_for_block_range(BlockRange {
            start: block,
            end: block + 1,
        })
    }

    pub fn get_events_for_block_range(&mut self, range: BlockRange) -> Result<Vec<NftEvent>> {
        let events = vec![
            Box::new(self.get_approvals_for_all_for_block_range(range)?)
                as Box<dyn Iterator<Item = NftEvent>>,
            Box::new(self.get_erc1155_transfers_batch_for_block_range(range)?),
            Box::new(self.get_erc1155_transfers_single_for_block_range(range)?),
            Box::new(self.get_erc1155_uri_for_block_range(range)?),
            Box::new(self.get_erc721_approvals_for_block_range(range)?),
            Box::new(self.get_erc721_transfers_for_block_range(range)?),
        ];
        Ok(merge_sorted_iters::<NftEvent>(events))
    }

    pub fn get_approvals_for_all_for_block_range(
        &mut self,
        range: BlockRange,
    ) -> Result<impl Iterator<Item = NftEvent>> {
        let events: Vec<DbApprovalForAll> = approval_for_all
            .filter(schema::approval_for_all::dsl::block_number.ge(&range.start))
            .filter(schema::approval_for_all::dsl::block_number.lt(&range.end))
            .load(&mut self.client)?;
        Ok(events.into_iter().map(|t| NftEvent {
            base: t.event_base(),
            meta: EventMeta::ApprovalForAll(t.into()),
        }))
    }

    pub fn get_erc1155_transfers_batch_for_block_range(
        &mut self,
        range: BlockRange,
    ) -> Result<impl Iterator<Item = NftEvent>> {
        let records: Vec<_> = sql_query(
            "
        SELECT
            tb.block_number,
            tb.log_index,
            tb.transaction_index,
            tb.address,
            tb.operator_0 as operator,
            tb.from_1 as from,
            tb.to_2 as to,
            array_agg(tbi.ids_0 ORDER BY tbi.array_index) as ids,
            array_agg(tbv.values_0 ORDER BY tbv.array_index) as values
        FROM erc1155_transfer_batch as tb
        INNER JOIN erc1155_transfer_batch_ids_0 as tbi
            ON tb.block_number = tbi.block_number
            AND tb.log_index = tbi.log_index
            AND tb.transaction_index = tbi.transaction_index
        INNER JOIN erc1155_transfer_batch_values_1 as tbv
            ON tb.block_number = tbv.block_number
            AND tb.log_index = tbv.log_index
            AND tb.transaction_index = tbv.transaction_index
        WHERE tb.block_number >= $1
        AND tb.block_number < $2
        AND tbi.array_index = tbv.array_index
        GROUP BY tb.block_number, tb.log_index, tb.transaction_index",
        )
        .bind::<BigInt, _>(range.start)
        .bind::<BigInt, _>(range.end)
        .load::<DbErc1155TransferBatch>(&mut self.client)?;

        Ok(records.into_iter().map(|t| NftEvent {
            base: t.event_base(),
            meta: EventMeta::Erc1155TransferBatch(t.into()),
        }))
    }

    pub fn get_erc1155_transfers_single_for_block_range(
        &mut self,
        range: BlockRange,
    ) -> Result<impl Iterator<Item = NftEvent>> {
        let events: Vec<DbErc1155TransferSingle> = erc1155_transfer_single
            .filter(schema::erc1155_transfer_single::dsl::block_number.ge(&range.start))
            .filter(schema::erc1155_transfer_single::dsl::block_number.lt(&range.end))
            .load(&mut self.client)?;
        Ok(events.into_iter().map(|t| NftEvent {
            base: t.event_base(),
            meta: EventMeta::Erc1155TransferSingle(t.into()),
        }))
    }
    pub fn get_erc1155_uri_for_block_range(
        &mut self,
        range: BlockRange,
    ) -> Result<impl Iterator<Item = NftEvent>> {
        let events: Vec<DbErc1155Uri> = erc1155_uri
            .filter(schema::erc1155_uri::dsl::block_number.ge(&range.start))
            .filter(schema::erc1155_uri::dsl::block_number.lt(&range.end))
            .load(&mut self.client)?;
        Ok(events.into_iter().map(|t| NftEvent {
            base: t.event_base(),
            meta: EventMeta::Erc1155Uri(t.into()),
        }))
    }

    pub fn get_erc721_approvals_for_block_range(
        &mut self,
        range: BlockRange,
    ) -> Result<impl Iterator<Item = NftEvent>> {
        let events: Vec<DbErc721Approval> = erc721_approval
            .filter(schema::erc721_approval::dsl::block_number.ge(&range.start))
            .filter(schema::erc721_approval::dsl::block_number.lt(&range.end))
            .load(&mut self.client)?;
        Ok(events.into_iter().map(|t| NftEvent {
            base: t.event_base(),
            meta: EventMeta::Erc721Approval(t.into()),
        }))
    }
    pub fn get_erc721_transfers_for_block_range(
        &mut self,
        range: BlockRange,
    ) -> Result<impl Iterator<Item = NftEvent>> {
        let db_transfers: Vec<DbErc721Transfer> = erc721_transfer
            .filter(schema::erc721_transfer::dsl::block_number.ge(&range.start))
            .filter(schema::erc721_transfer::dsl::block_number.lt(&range.end))
            .load(&mut self.client)?;
        Ok(db_transfers.into_iter().map(|t| NftEvent {
            base: t.event_base(),
            meta: EventMeta::Erc721Transfer(t.into()),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_DB_URL: &str = "postgresql://postgres:postgres@localhost:5432/arak";

    fn single_block_range(block: i64) -> BlockRange {
        BlockRange {
            start: block,
            end: block + 1,
        }
    }

    fn test_client() -> EventSource {
        EventSource::new(TEST_DB_URL).unwrap()
    }
    #[test]
    fn approvals_for_all() {
        // select block_number, count(*) cnt
        // from approval_for_all
        // group by block_number
        // order by cnt desc
        // limit 1;
        let approvals = test_client()
            .get_approvals_for_all_for_block_range(single_block_range(15_000_297))
            .unwrap();
        assert!(!approvals.collect::<Vec<_>>().is_empty());
    }

    #[test]
    fn erc1155_transfer_single() {
        let transfer_singles = test_client()
            .get_erc1155_transfers_single_for_block_range(single_block_range(15_001_228))
            .unwrap();
        assert!(!transfer_singles.collect::<Vec<_>>().is_empty());
    }
    #[test]
    fn erc1155_uri() {
        let uris = test_client()
            .get_erc1155_uri_for_block_range(single_block_range(15_000_762))
            .unwrap();
        assert!(!uris.collect::<Vec<_>>().is_empty());
    }
    #[test]
    fn erc721_approvals() {
        let approvals = test_client()
            .get_erc721_approvals_for_block_range(single_block_range(15_001_087))
            .unwrap();
        assert!(!approvals.collect::<Vec<_>>().is_empty());
    }
    #[test]
    fn erc721_transfers() {
        let transfers = test_client()
            .get_erc721_transfers_for_block_range(single_block_range(15_000_123))
            .unwrap();
        assert!(!transfers.collect::<Vec<_>>().is_empty());
    }

    #[test]
    fn erc1155_batch_transfers() {
        // These logs are emitted over two transactions:
        // 0xb698ee1beddeb16ad1b27ed0bf1ff896654fbf7b8abcb08440976f3559820350
        // 0x5dd5fa286c0944011f13dfa982f06e20c29eef3abc26a1bde096db0faefee454
        // Check them out on https://etherscan.io

        let mut client = EventSource::new(TEST_DB_URL).unwrap();
        let batch_transfer_transactions: Vec<_> = client
            .get_erc1155_transfers_batch_for_block_range(single_block_range(15_000_741))
            .unwrap()
            .map(|event| event.base.transaction_index)
            .collect();
        assert_eq!(batch_transfer_transactions, vec![56, 104]);
    }

    fn is_sorted<T: Ord>(vec: &[T]) -> bool {
        vec.windows(2).all(|w| w[0] <= w[1])
    }
    #[test]
    fn get_events_for_block() {
        // This test uses a block 15_000_000 containing relevant event types
        let mut client = EventSource::new(TEST_DB_URL).unwrap();
        let batch_transfers: Vec<_> = client.get_events_for_block(15_000_000).unwrap();

        assert!(batch_transfers.len() >= 20);
        assert!(is_sorted(batch_transfers.as_slice()))
    }
}

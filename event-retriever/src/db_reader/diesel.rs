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
use std::collections::btree_map::BTreeMap;

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

    pub fn get_events_for_block(&mut self, block: i64) -> Result<BTreeMap<u64, Vec<NftEvent>>> {
        let mut events = self.get_events_for_block_range(BlockRange {
            start: block,
            end: block + 1,
        })?;
        Ok(events.remove(&(block as u64)).unwrap_or(BTreeMap::new()))
    }

    pub fn get_events_for_block_range(
        &mut self,
        range: BlockRange,
    ) -> Result<BTreeMap<u64, BTreeMap<u64, Vec<NftEvent>>>> {
        let events = vec![
            Box::new(self.get_approvals_for_all_for_block_range(range)?)
                as Box<dyn Iterator<Item = NftEvent>>,
            Box::new(self.get_erc1155_transfers_batch_for_block_range(range)?),
            Box::new(self.get_erc1155_transfers_single_for_block_range(range)?),
            Box::new(self.get_erc1155_uri_for_block_range(range)?),
            Box::new(self.get_erc721_approvals_for_block_range(range)?),
            Box::new(self.get_erc721_transfers_for_block_range(range)?),
        ];
        // We probably don't need this anymore (or this can construct the map).
        let ordered_events = merge_sorted_iters::<NftEvent>(events);
        let mut result: BTreeMap<u64, BTreeMap<u64, Vec<NftEvent>>> = BTreeMap::new();
        for event in ordered_events {
            result
                .entry(event.base.block_number)
                .or_default()
                .entry(event.base.transaction_index)
                .or_default()
                .push(event)
        }
        Ok(result)
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
    use maplit::btreemap;
    use std::str::FromStr;

    use super::*;
    use crate::db_reader::models::*;
    use shared::eth::{Address, U256};

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
            .get_erc1155_uri_for_block_range(single_block_range(15_000_204))
            .unwrap()
            .collect::<Vec<_>>();
        assert_eq!(uris.len(), 1);
        assert_eq!(
            uris[0].meta,
            EventMeta::Erc1155Uri(Erc1155Uri {
                id: U256::from_dec_str(
                    "79495434600586702638590703444944964074128496799967025262870501822812670394369"
                )
                .unwrap(),
                value: "ipfs://bafkreibw5wy6wsqukosezcltq5k4necb3k32rdrlg6fltaw7m5q7daqhyq"
                    .to_string()
            })
        );
    }
    #[test]
    fn erc721_approvals() {
        let approvals = test_client()
            .get_erc721_approvals_for_block_range(single_block_range(15_000_976))
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(
            approvals,
            vec![
                NftEvent {
                    base: EventBase {
                        block_number: 15000976,
                        log_index: 126,
                        transaction_index: 33,
                        contract_address: Address::from_str(
                            "0x8ff1523091c9517bc328223d50b52ef450200339"
                        )
                        .unwrap()
                    },
                    meta: EventMeta::Erc721Approval(Erc721Approval {
                        owner: Address::from_str("0xbe4f28db3e39fbcf420b8f9fc5cf4d244c85a09e")
                            .unwrap(),
                        approved: Address::zero(),
                        id: U256::from(2993)
                    })
                },
                NftEvent {
                    base: EventBase {
                        block_number: 15000976,
                        log_index: 167,
                        transaction_index: 43,
                        contract_address: Address::from_str(
                            "0x5e9dc633830af18aa43ddb7b042646aadedcce81"
                        )
                        .unwrap()
                    },
                    meta: EventMeta::Erc721Approval(Erc721Approval {
                        owner: Address::from_str("0xd577002b765e048fda0b64fad500c9b2cb6fa2e4")
                            .unwrap(),
                        approved: Address::zero(),
                        id: U256::from(436)
                    })
                }
            ]
        );
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
        let batch_transfers: Vec<_> = client
            .get_erc1155_transfers_batch_for_block_range(single_block_range(15_000_741))
            .unwrap()
            .map(|event| event.meta)
            .collect();
        assert_eq!(
            batch_transfers,
            vec![
                EventMeta::Erc1155TransferBatch(Erc1155TransferBatch {
                    operator: Address::from_str("0x381e840f4ebe33d0153e9a312105554594a98c42")
                        .unwrap(),
                    from: Address::from_str("0x381e840f4ebe33d0153e9a312105554594a98c42").unwrap(),
                    to: Address::from_str("0x3bc53864b408e7bca94505c63116e9b73407f3e1").unwrap(),
                    ids: vec![
                        U256::from_dec_str("426033523385014956256145008504573800742912").unwrap()
                    ],
                    values: vec![U256::from(1)]
                }),
                EventMeta::Erc1155TransferBatch(Erc1155TransferBatch {
                    operator: Address::from_str("0xb1eaa7260ab9e0b413d40d700ebee7bd5e671803")
                        .unwrap(),
                    from: Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
                    to: Address::from_str("0xb1eaa7260ab9e0b413d40d700ebee7bd5e671803").unwrap(),
                    ids: vec![U256::from(0), U256::from(4), U256::from(9)],
                    values: vec![U256::from(1), U256::from(1), U256::from(1)]
                })
            ]
        );
    }

    // fn is_sorted<T: Ord>(vec: &[T]) -> bool {
    //     vec.windows(2).all(|w| w[0] <= w[1])
    // }
    #[test]
    fn get_events_for_block() {
        // This test uses a block 15_001_141 containing more than 1 relevant event type
        // This test also demonstrates correctness of Diesel EVM Types.
        let mut client = EventSource::new(TEST_DB_URL).unwrap();
        let batch_transfers: BTreeMap<_, Vec<_>> = client.get_events_for_block(15_001_141).unwrap();
        assert_eq!(
            batch_transfers,
            btreemap! {0 => vec![NftEvent {
                base: EventBase {
                    block_number: 15001141,
                    log_index: 0,
                    transaction_index: 0,
                    contract_address: Address::from_str(
                        "0xba100000625a3754423978a60c9317c58a424e3d"
                    )
                    .unwrap()
                },
                meta: EventMeta::Erc721Transfer(Erc721Transfer {
                    from: Address::from_str("0x527f31b668aa54e1be2a5a5b511442ec24ae5540")
                        .unwrap(),
                    to: Address::from_str("0x0450cd91ef89740410685f5e618eb4570fcce009")
                        .unwrap(),
                    token_id: U256::from(0)
                })
            }], 2 => vec![NftEvent {
                base: EventBase {
                    block_number: 15001141,
                    log_index: 1,
                    transaction_index: 2,
                    contract_address: Address::from_str(
                        "0x004cf82a346a71245193075a9b91f4329180766d"
                    )
                    .unwrap()
                },
                meta: EventMeta::ApprovalForAll(ApprovalForAll {
                    owner: Address::from_str("0x86002b029cbaa1768f16b05ba8fa68bba72a82c3")
                        .unwrap(),
                    operator: Address::from_str("0x1e0049783f008a0085193e00003d00cd54003c71")
                        .unwrap(),
                    approved: true
                })
            }], 38 => vec![NftEvent {
                base: EventBase {
                    block_number: 15001141,
                    log_index: 2,
                    transaction_index: 38,
                    contract_address: Address::from_str(
                        "0xdac17f958d2ee523a2206206994597c13d831ec7"
                    )
                    .unwrap()
                },
                meta: EventMeta::Erc721Transfer(Erc721Transfer {
                    from: Address::from_str("0xb5d85cbf7cb3ee0d56b3bb207d5fc4b82f43f511")
                        .unwrap(),
                    to: Address::from_str("0x43dcc215a0d449675ec582802d229d2df1129978")
                        .unwrap(),
                    token_id: U256::from(0)
                })
            }] }
        );
    }
}

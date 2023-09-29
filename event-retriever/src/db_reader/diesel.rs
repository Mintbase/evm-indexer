use crate::db_reader::{
    models::{
        db::{DbErc1155TransferBatch, DbErc721Transfer},
        Erc1155TransferBatch, Erc721Transfer,
    },
    schema::erc721_transfer::dsl::{block_number, erc721_transfer},
};
use anyhow::{Context, Result};
use diesel::{pg::PgConnection, prelude::*, sql_query, sql_types::BigInt, Connection, RunQueryDsl};

pub struct DieselClient {
    client: PgConnection,
}

impl DieselClient {
    pub fn new(connection: &str) -> Result<Self> {
        Ok(Self {
            client: DieselClient::establish_connection(connection)?,
        })
    }

    fn establish_connection(db_url: &str) -> Result<PgConnection> {
        PgConnection::establish(db_url).context("Error connecting to Diesel Client")
    }

    pub fn get_erc1155_transfers_batch_for_block(
        &mut self,
        block: &i64,
    ) -> Result<impl Iterator<Item = Erc1155TransferBatch>> {
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
        WHERE tb.block_number = $1
        AND tbi.array_index = tbv.array_index
        GROUP BY tb.block_number, tb.log_index, tb.transaction_index",
        )
        .bind::<BigInt, _>(block)
        .load::<DbErc1155TransferBatch>(&mut self.client)?;

        Ok(records.into_iter().map(|t| t.into()))
    }
    pub fn get_erc721_transfers_for_block(
        &mut self,
        block: i64,
    ) -> Result<impl Iterator<Item = Erc721Transfer>> {
        let db_transfers: Vec<DbErc721Transfer> = erc721_transfer
            .filter(block_number.eq(&block))
            .load(&mut self.client)?;
        Ok(db_transfers.into_iter().map(|t| t.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_reader::models::EventBase;
    use ethers::types::{Address, U256};
    use std::str::FromStr;

    static TEST_DB_URL: &str = "postgresql://postgres:postgres@localhost:5432/postgres";
    #[test]
    fn test_example() {
        let mut client = DieselClient::new(TEST_DB_URL).unwrap();
        let transfers = client.get_erc721_transfers_for_block(1001165).unwrap();
        assert!(!transfers.collect::<Vec<_>>().is_empty());
    }

    #[test]
    fn test_join() {
        let mut client = DieselClient::new(TEST_DB_URL).unwrap();
        let batch_transfers: Vec<_> = client
            .get_erc1155_transfers_batch_for_block(&10000246)
            .unwrap()
            .collect();

        assert_eq!(
            batch_transfers,
            vec![Erc1155TransferBatch {
                base: EventBase {
                    block_number: 10000246,
                    log_index: 101,
                    transaction_index: 88,
                    contract_address: Address::from_str(
                        "0xfaafdc07907ff5120a76b34b731b278c38d6043c"
                    )
                    .unwrap()
                },
                operator: Address::from_str("0x913c7fa57e6690f96b4aeb65553f0ed3664caf8b").unwrap(),
                from: Address::from_str("0x913c7fa57e6690f96b4aeb65553f0ed3664caf8b").unwrap(),
                to: Address::from_str("0x0544fbed9b72aa036517b21d1db50201a17d09ce").unwrap(),
                ids: [
                    "50885195465617476136641626189999439165077792154310195491295815731572381843464",
                    "50885195465617476142918727925386119928913581577517861907398171176036416356391",
                    "50885195465617476149195829660772800692749371000725528323500526620500450869273",
                    "50885195465617476124087422719226077637406213307894862659091104842644312817683",
                    "50885195465617476130364524454612758401242002731102529075193460287108347330610",
                ]
                .map(|t| U256::from_dec_str(t).unwrap())
                .to_vec(),
                values: [1; 5].map(U256::from).to_vec()
            }]
        )
    }
}

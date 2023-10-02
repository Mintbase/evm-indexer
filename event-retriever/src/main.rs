use crate::db_reader::diesel::DieselClient;
use dotenv::dotenv;
pub mod db_reader;

fn main() {
    dotenv().ok();
    let db_url = std::env::var("DB_URL").expect("Missing env var DB_URL");
    let mut pg_client = DieselClient::new(&db_url).expect("Failed to connect to DB");
    let block = 10_000_246;
    let transfers: Vec<_> = pg_client.get_events_for_block(block).unwrap();
    println!("Retrieved {:?} transfers at block {block}", transfers.len());
    for t in transfers {
        println!("{:?}", t);
    }
}

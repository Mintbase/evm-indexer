# Build your own pre-populated test database

## Locally from Arak Project (recommended)

1. Clone our arak project:

   ```sh
   git clone https://github.com/Mintbase/arak
   cd arak
   ```

2. Start an empty database:

   ```sh
   docker-compose up -d
   ```

3. Configure your toml to match
   our [deployment configuration](https://github.com/Mintbase/arak/blob/ec2e49fe92532d7b979591de326bb2bab422f4c9/kubernetes/config/configmap.yaml#L7)

   Note that you may want to change the start blocks.

   ```sh
   touch arak.toml # Copy your config here
   ```

4. Run arak for a few minutes to populate the DB

   ```sh
   RUST_LOGS=info,arak=debug cargo run
   ```

5. Make a backup of the DB and zip it:

   ```sh
   pg_dump --host localhost --port 5432 --user postgres --dbname arak | gzip > sample_events.sql.gz
   ```

   You will want to now shut down this database `docker-compose down` (since we will run another one later)

6. Copy this backup into the `test_db` directory here.

7. Run Migrations from this project

   ```sh
   docker-compose up -d
   ```

8. Try it out!
   ```sh
   cargo test
   ```

## Without Cloning Arak

1. Run Postgres Image

   ```sh
   docker run -p 5432:5432 -e POSTGRES_USER=postgres -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=arak -d postgres
   ```

2. Configure your Event toml (to match our deployment config -- see step 3 above)

   Make sure also to include the database and ethrpc in the config:

   ```yaml
   ethrpc = "https://rpc.ankr.com/eth"
   
   [ database.postgres ]
   connection = "postgresql://postgres:postgres@localhost:5432/postgres"
   
   
   [ indexer ]
   page-size = 25
   poll-interval = 1
   ```

   You can also pass env vars:

   ```
   NODE_URL=https://rpc.ankr.com/eth
   DB_STRING=postgresql://arak:123@localhost:5432/arak
   ```

3. Run Arak image for a while:

   ```
   docker run --network host --add-host=localhost:host-gateway -v ${PWD}/arak.toml:/opt/config.toml -e ARAKCONFIG=/opt/config.toml ghcr.io/mintbase/arak:main
   ```

   Note: If you chose the env var route you will also want to append `-e NODE_URL=...` and `-e DB_STRING=...`

4. Follow Steps 5 - 8 from above.

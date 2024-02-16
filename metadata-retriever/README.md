

1. Run Pub/Sub Emulator Service                                                                       

```sh
docker-compose up -d
```

You will need a environment file with the following values set:
```text
STORE_URL=postgresql://postgres:postgres@localhost:5432/store
DB_SCHEMA=public

ETHERSCAN_KEY=
```

2. Run the Service

    Make sure to have the following env vars set:
    ```sh
    PUBSUB_EMULATOR_HOST=localhost:8681
    TOPIC_NAME=test-topic
    SUBSCRIPTION_NAME=test-subscription
    ETHERSCAN_KEY=$ETHERSCAN_KEY
    ```
        
    ```sh
    cargo run --bin metadata-retriever
    ```

You can POST JSON documents of type [Message](../eth/src/types/message.rs) to this service as follows:

```sh
 # Contract Message
 curl -X POST \
  http://localhost:8080/pubsub_callback \
  -H 'Content-Type: application/json' \
  -d '{"contract":{"address":"0x510887C470EE8EEBEBFF0104B54D24AEF8C45368"}}'
 # Token Request
 curl -X POST \
  http://localhost:8080/pubsub_callback \
  -H 'Content-Type: application/json' \
  -d '{"token":{"address":"0x510887C470EE8EEBEBFF0104B54D24AEF8C45368","token_id":"9013","token_uri":null}}'
```

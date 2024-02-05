

1. Run Pubsub Emulator Service

```sh
docker-compose up -d
ETHERSCAN_KEY=$ETHERSCAN_KEY cargo run
```

### Batch Request

```sh
curl -X POST \
  http://localhost:8080/pubsub_callback \
  -H 'Content-Type: application/json' \
  -d '[{"contract":{"address":"0x0000000000000000000000000000000000000001"}},{"token":{"address":"0x0000000000000000000000000000000000000002","token_id":"12345678999999"}}]'
```

### Contract Request
```sh
 curl -X POST \
  http://localhost:8080/pubsub_callback \
  -H 'Content-Type: application/json' \
  -d '{"contract":{"address":"0x510887C470EE8EEBEBFF0104B54D24AEF8C45368"}}'
```


### Token Request

```sh
 curl -X POST \
  http://localhost:8080/pubsub_callback \
  -H 'Content-Type: application/json' \
  -d '{"token":{"address":"0x510887C470EE8EEBEBFF0104B54D24AEF8C45368","token_id":"9013"}}'
```

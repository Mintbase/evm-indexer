# Installation & Local Development

## Running Tests

Many of our tests depend on a sample database which can be run (in the background) with

```sh
docker-compose up -d
cargo test -- --test-threads 1 
```

## Build Project Image

```shell
docker build -f docker/Dockerfile.binary -t indexer .
```

### Run Event Handler

Copy the example environment file and fill out your desired configuration:

```shell
cp ./event-handler/.env.example ./event-handler/.env
```

```shell
docker run --rm --env-file ./event-handler/.env indexer event-handler
```

Note: If you are running against a local (docker) instance of postgres you will need to include
`--network host --add-host=localhost:host-gateway`
 
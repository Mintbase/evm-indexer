# Installation & Local Development


## Running Tests

Many of our tests depend on a sample database which can be run (in the background) with 

```sh
docker-compose up -d
cargo test -- --test-threads 1 
```
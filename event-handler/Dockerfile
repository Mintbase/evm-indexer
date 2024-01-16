FROM rust:latest as builder

WORKDIR /usr/src/handler

COPY ./src ./src
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release

FROM debian:buster-slim
COPY --from=builder /usr/src/handler/target/release/handler_binary /usr/local/bin/handler_binary

CMD ["handler_binary"]

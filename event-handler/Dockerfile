FROM rust:bookworm as builder

WORKDIR /src/

# Copy & Build
COPY . .
RUN cargo build --release

# Extract Binary
FROM docker.io/debian:bookworm-slim

RUN apt-get update && apt-get install -y tini openssl libpq-dev
COPY --from=builder /src/target/release/event-handler /usr/local/bin/event-handler

CMD echo "Specify binary..."
ENTRYPOINT ["/usr/bin/tini", "--"]

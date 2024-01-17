FROM rust:1.75.0 as builder

WORKDIR /src/

RUN apt-get update && apt-get install -y git libssl-dev

# Copy & Build
COPY . .
RUN CARGO_PROFILE_RELEASE_DEBUG=1 cargo build --release

# Extract Binary
FROM docker.io/debian:bullseye-slim

# Handle signal handlers properly
RUN apt-get update && apt-get install -y ca-certificates tini && apt-get clean
COPY --from=builder /src/target/release/event-handler /usr/local/bin/event-handler

CMD echo "Specify binary..."
ENTRYPOINT ["/usr/bin/tini", "--"]

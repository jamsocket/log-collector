FROM rust:1.77-slim-buster as builder

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
RUN mkdir src
RUN echo "fn main() {}" > src/main.rs

# Build dependencies
RUN cargo build --release

COPY src src
# We need to touch src/main.rs to force cargo to build it.
RUN touch src/main.rs

# Build application
RUN cargo build --release

FROM debian:buster-slim

COPY --from=builder /build/target/release/jamsocket-log-collector /usr/local/bin/jamsocket-log-collector

CMD ["jamsocket-log-collector"]

FROM docker.io/library/rust:1.90-alpine3.22 AS builder

RUN apk add --no-cache musl-dev
WORKDIR /root/server_rs

COPY Cargo.toml Cargo.lock .

COPY src/  src/

RUN cargo build --profile release

FROM docker.io/library/rockylinux:8

RUN useradd -m runner
USER runner

COPY --from=builder /root/server_rs/target/release/server_rs /home/runner/server_rs

entrypoint ["/home/runner/server_rs"]
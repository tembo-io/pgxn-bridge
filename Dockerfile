FROM rust:1.76-alpine as builder

RUN apk update && apk add --no-cache musl-dev openssl-dev gcc git openssl-libs-static

ENV RUSTFLAGS="-C target-feature=+crt-static"
ENV PKG_CONFIG_ALLOW_CROSS=1
ENV OPENSSL_STATIC=true

WORKDIR /usr/src
RUN git clone https://github.com/tembo-io/pgxn-bridge.git
WORKDIR /usr/src/pgxn-bridge

RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine

COPY --from=builder /usr/src/pgxn-bridge/target/x86_64-unknown-linux-musl/release/pgxn-bridge .

# Env variables required by pgxn-bridge
ENV GH_PAT=
ENV GH_USERNAME=
ENV GH_EMAIL=
ENV GH_AUTHOR=

ENTRYPOINT ["./pgxn-bridge"]
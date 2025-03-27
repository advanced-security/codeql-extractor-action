FROM docker.io/library/rust:1.85-alpine as builder

ENV TARGET=x86_64-unknown-linux-musl

WORKDIR /app

COPY . .

# Install dependencies
RUN apk update && \
    apk add --no-cache pkgconf alpine-sdk openssl-dev perl musl-dev && \
    rustup target add ${TARGET} && \
    cargo build --release --target ${TARGET} && \
    mv target/${TARGET}/release/codeql-extractor-action target/

FROM docker.io/library/alpine:3.21
WORKDIR /app

RUN apk update && \
    apk add --no-cache github-cli && \
    rm -rf /var/cache/apk/*

COPY --from=builder /app/target/codeql-extractor-action /usr/local/bin/codeql-extractor-action

ENTRYPOINT ["action"]

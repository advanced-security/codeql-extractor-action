FROM docker.io/library/rust:1.85-slim as builder

ENV TARGET=x86_64-unknown-linux-gnu

WORKDIR /app

COPY . .

# Install dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends pkg-config build-essential libssl-dev && \
    cargo build --release && \
    mv target/release/codeql-extractor-action target/

FROM docker.io/library/debian:12-slim
WORKDIR /app

COPY --from=builder /app/target/codeql-extractor-action /usr/local/bin/codeql-extractor-action

# Install gh CLI
RUN apt-get update && \
    apt-get install -y --no-install-recommends curl git ca-certificates && \
    curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg && \
    chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg && \
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | tee /etc/apt/sources.list.d/github-cli.list > /dev/null && \
    apt-get update && \
    apt-get install -y --no-install-recommends gh && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

ENTRYPOINT [ "codeql-extractor-action" ]

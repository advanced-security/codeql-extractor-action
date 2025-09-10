FROM docker.io/library/rust:1.89-slim AS builder

ENV TARGET=x86_64-unknown-linux-gnu

WORKDIR /app

COPY . .

# Install dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends pkg-config build-essential libssl-dev && \
    cargo build --release --target $TARGET && \
    mv target/${TARGET}/release/codeql-extractor-action target/

# We have to use Debian testing as the stable version has an old
# version of `glibc` that doesn't work with new-ist versions of CodeQL.
FROM docker.io/library/debian:testing-slim
WORKDIR /app

COPY --from=builder /app/target/codeql-extractor-action /usr/local/bin/codeql-extractor-action

# Install GitHub CLI
RUN apt-get update && \
    apt-get install -y curl git ca-certificates && \
    curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg && \
    chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg && \
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | tee /etc/apt/sources.list.d/github-cli.list > /dev/null && \
    apt-get update && \
    apt-get install -y --no-install-recommends gh && \
    apt-get remove -y curl && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Install the CodeQL extension for GitHub CLI
RUN gh extensions install github/gh-codeql && \
    gh codeql install-stub

ENTRYPOINT [ "codeql-extractor-action" ]

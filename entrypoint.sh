#!/bin/sh
set -e

export CODEQL_PATH="${CODEQL_PATH:-/opt/codeql}"
export CODEQL_CLI_VERSION="${CODEQL_CLI_VERSION:-latest}"

# Check for codeql
if which codeql >/dev/null; then
    export CODEQL_BINARY="codeql"
else
    mkdir -p "$CODEQL_PATH"
    echo "[+] Downloading CodeQL CLI..."
    if [ "$CODEQL_CLI_VERSION" = "latest" ]; then
        CODEQL_CLI_VERSION=$(gh release list --repo github/codeql-cli-binaries)
    fi
    echo "[+] CodeQL CLI version: $CODEQL_CLI_VERSION"

    cd "$CODEQL_PATH"
    gh release download "v${CODEQL_CLI_VERSION}" \
        --repo https://github.com/github/codeql-cli-binaries \
        --pattern codeql-linux64.zip \
        --clobber \
        --output "$CODEQL_PATH/codeql-linux64.zip"

    unzip -q "$CODEQL_PATH/codeql-linux64.zip" -d "$CODEQL_PATH"

    export CODEQL_BINARY="$CODEQL_PATH/codeql"
    echo "Completed downloading CodeQL CLI."
fi

codeql-extractor-action "$@"


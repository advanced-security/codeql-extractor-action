FROM ghcr.io/advanced-security/codeql-extractor-action:v0.1.0

ARG INPUT_TOKEN

RUN export GH_TOKEN=$INPUT_TOKEN && \
    gh extensions install github/gh-codeql

ENTRYPOINT [ "codeql-extractor-action" ]

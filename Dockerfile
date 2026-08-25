# syntax=docker/dockerfile:1.7

ARG RUST_BUILDER_IMAGE="rust:1.85-bookworm"
ARG CLI_RUNTIME_IMAGE="debian:bookworm-slim"

FROM ${RUST_BUILDER_IMAGE} AS builder

WORKDIR /source

COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY .config/rules/portability.toml .config/rules/portability.toml
COPY .config/rules/repository-intelligence.v1.toml .config/rules/repository-intelligence.v1.toml
COPY .config/rules/javascript-architecture.v1.json .config/rules/javascript-architecture.v1.json
COPY .config/megalinter/tool-matrix.json .config/megalinter/tool-matrix.json
COPY .config/security/scanner-ownership.json .config/security/scanner-ownership.json

RUN cargo build --locked --release --package "egolint" --bin "egolint"

FROM ${CLI_RUNTIME_IMAGE} AS runtime

ARG BUILD_DATE=""
ARG VERSION="0.1.0-alpha.1"
ARG REVISION=""

LABEL org.opencontainers.image.title="egolint" \
      org.opencontainers.image.description="Portable policy-driven lint orchestrator CLI" \
      org.opencontainers.image.source="https://github.com/egohygiene/egolint" \
      org.opencontainers.image.url="https://github.com/egohygiene/egolint" \
      org.opencontainers.image.documentation="https://github.com/egohygiene/egolint/tree/main/docs" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.created="${BUILD_DATE}" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${REVISION}"

COPY --from=builder /source/target/release/egolint /usr/local/bin/egolint
COPY LICENSE NOTICE /usr/share/doc/egolint/

WORKDIR /workspace
USER 65532:65532

ENTRYPOINT ["egolint"]
CMD ["--help"]

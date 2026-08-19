# Contributing

Egolint is in active alpha development. Open an issue before a large change so
contracts and policy ownership can be agreed before implementation.

## Local checks

Rust 1.85 or newer is required. From the repository root:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo run --locked -- plan --workspace "." --profile "fast"
python3 "scripts/validate_megalinter_policy.py"
```

If Docker is available, build both definitions and smoke-test introspection:

```sh
docker build --file "Dockerfile" --tag "egolint:local" "."
docker build --file "Dockerfile.full" --tag "egolint-full:local" "."
docker run --rm "egolint:local" schema config
```

Keep orchestration typed and shell-free. New persistent settings require schema,
precedence, validation, and provenance tests. New policy behavior requires both
positive and negative fixtures. Document any license added to the full-image
boundary and update `REUSE.toml` when files have different terms.

Do not commit generated reports, credentials, local configuration, or unreviewed
release workflows. Contributions are accepted under the repository's applicable
licenses.

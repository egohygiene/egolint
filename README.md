# egolint

Portable, policy-driven lint orchestration for repositories and CI.

> [!IMPORTANT]
> Egolint is an early alpha. The CLI is source-buildable, but no Cargo package,
> GitHub release, or GHCR image is claimed to be published yet. Interfaces and
> policy defaults may change before the first stable release.

Egolint separates orchestration from the lint engine:

- The Rust CLI resolves layered configuration, produces inspectable execution
  plans, and launches a hardened Docker or Podman host process as direct argv
  without host-shell interpolation.
- `ghcr.io/egohygiene/egolint` is the planned lightweight CLI image. It is useful
  for `plan`, `schema`, and configuration inspection; it does not contain a
  container runtime or require access to a Docker socket.
- `ghcr.io/egohygiene/egolint-full` is the planned lint-engine image. It extends
  MegaLinter and embeds the fast and holistic Ego Hygiene policies. The native
  CLI launches this image and intentionally preserves MegaLinter's entrypoint.

## Current alpha surface

| Command | Purpose |
| --- | --- |
| `egolint check` | Run linters with the repository mounted read-only. |
| `egolint fix` | Explicitly permit tools to modify the repository. |
| `egolint plan` | Print the redacted container invocation without running it. |
| `egolint doctor` | Validate configuration and runtime readiness. |
| `egolint config explain` | Show effective configuration and ordered sources. |
| `egolint schema` | Emit the config, plan, or report JSON Schema. |

The `fast` profile targets short changed-file feedback. The `holistic` profile
performs the broader repository and security inspection encoded by the bundled
MegaLinter policy.

## Try it from source

Rust 1.85 or newer is required.

```sh
cargo build --locked
cargo run --locked -- plan --workspace "." --profile "fast"
cargo run --locked -- schema config
cargo run --locked -- config explain --format "json"
```

Build the two images locally:

```sh
docker build --file "Dockerfile" --tag "egolint:local" "."
docker build --file "Dockerfile.full" --tag "egolint-full:local" "."

docker run --rm "egolint:local" schema config
cargo run --locked -- check \
  --workspace "." \
  --image "egolint-full:local" \
  --profile "fast" \
  --pull-policy "never"
```

The second command runs the native CLI, which launches the local full image.
Do not mount a host Docker socket into the lightweight CLI image. An in-container
local MegaLinter adapter is a future capability, not part of this alpha.

## Configuration

Copy [`examples/egolint.toml`](examples/egolint.toml) into a repository and run:

```sh
egolint config explain
egolint plan --format "json"
```

Configuration is deterministic and explainable. Later sources override earlier
ones: compiled defaults, user config, repository config, local config, explicit
config, `EGOLINT_*` environment variables, and CLI options. User and local files
are skipped when `CI` is truthy. See [configuration](docs/configuration.md) for
the exact rules.

## Documentation

- [Architecture](docs/architecture.md)
- [Configuration](docs/configuration.md)
- [Containers and image boundaries](docs/containers.md)
- [Security model](docs/security.md)
- [Release design](docs/releasing.md)
- [Migration from Empathy](docs/migration-from-empathy.md)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [Changelog](CHANGELOG.md)

## License boundary

Egolint's Rust code and first-party policy are licensed under the [MIT
License](LICENSE). The full image extends and contains MegaLinter, which is
licensed under AGPL-3.0-only, and also contains third-party linters under their
own licenses. Building or distributing the full image does not relicense those
components as MIT. See [NOTICE](NOTICE) and [containers](docs/containers.md).

<p>
  <img
    src="assets/identity/repository-presentation/assets/banner-dark-1600.svg"
    alt="EgoLint repository banner with source lines passing through an inspection lens and check"
    width="100%"
  />
</p>

# EgoLint

Portable, policy-driven lint orchestration for repositories and CI.

<p>
  <a href="evidence/repository-presentation.json">
    <img
      src="assets/identity/repository-presentation/assets/hygienic-partial.svg"
      alt="Hygienic repository presentation profile: partial; open the linked evidence"
    />
  </a>
</p>

> [!IMPORTANT] Egolint is an early alpha. The CLI is source-buildable, but no Cargo package, GitHub
> release, or GHCR image is claimed to be published yet. Interfaces and policy defaults may change
> before the first stable release.

Egolint separates orchestration from the lint engine:

- The Rust CLI resolves layered configuration, produces inspectable execution plans, and launches a
  hardened Docker or Podman host process as direct argv without host-shell interpolation.
- `ghcr.io/egohygiene/egolint` is the planned lightweight CLI image. It is useful for `plan`,
  `schema`, and configuration inspection; it does not contain a container runtime or require access
  to a Docker socket.
- `ghcr.io/egohygiene/egolint-full` is the planned lint-engine image. It extends MegaLinter and
  embeds the fast, holistic, security, and dependency-debt Ego Hygiene policies. The native CLI
  launches this image and intentionally preserves MegaLinter's entrypoint.

## Current alpha surface

| Command             | Purpose                                                                                                  |
| ------------------- | -------------------------------------------------------------------------------------------------------- |
| `egolint lint`      | Run linters with the repository mounted read-only.                                                       |
| `egolint fix`       | Generate a bounded patch in an isolated copy; never edit the source worktree.                            |
| `egolint apply-fix` | Apply and stage one reviewed patch only when its SHA-256, base commit, and expected post-tree all match. |
| `egolint validate`  | Run native portability and repository-contract checks without a container.                               |
| `egolint plan`      | Print the redacted container invocation without running it.                                              |
| `egolint doctor`    | Validate configuration and runtime readiness.                                                            |
| `egolint explain`   | Show effective configuration and ordered sources.                                                        |
| `egolint schema`    | Emit a checked-in machine contract as JSON Schema.                                                       |

`check` remains an alias for `lint`, and `config explain` remains available for compatibility with
the first alpha surface.

The `fast` profile gives short changed-file MegaLinter feedback while native portability and
repository-policy checks still inspect the complete repository. `holistic` performs the broader
scheduled, manual, and trusted-branch inspection. `security` focuses secret, static-analysis, and
infrastructure checks. `dependency-debt` focuses vulnerability and inventory evidence. All four
selections are versioned in the policy catalog.

Successful lint runs write normalized `.reports/egolint/run.json` and
`.reports/egolint/egolint.sarif`. Repository Intelligence validation also writes
`.reports/egolint/repository-intelligence.json`; repository-presentation validation writes the
privacy-safe `.reports/egolint/repository-presentation.json`. Both include represented source, rule
IDs, locations, and remediation. Dependency-debt runs write compact JSON and Markdown debt
summaries; raw MegaLinter reports remain private adapter artifacts.

## Try it from source

Rust 1.85 or newer is required.

```sh
cargo build --locked
cargo run --locked -- plan --workspace "." --profile "fast"
cargo run --locked -- schema config
cargo run --locked -- schema repository-contract
cargo run --locked -- schema repository-intelligence
cargo run --locked -- schema repository-intelligence-report
cargo run --locked -- schema repository-presentation
cargo run --locked -- schema repository-presentation-report
cargo run --locked -- explain --format "json"
cargo run --locked -- validate --repository-contract \
  "tests/fixtures/contracts/empathy-universal-v1.toml"
```

Build the two images locally:

```sh
docker build --file "Dockerfile" --tag "egolint:local" "."
docker build --file "Dockerfile.full" --tag "egolint-full:local" "."

docker run --rm "egolint:local" schema config
cargo run --locked -- lint \
  --workspace "." \
  --image "egolint-full:local" \
  --profile "fast" \
  --pull-policy "never"
```

The second command runs the native CLI, which launches the local full image. Do not mount a host
Docker socket into the lightweight CLI image. An in-container local MegaLinter adapter is a future
capability, not part of this alpha.

## Dogfood the complete architecture

Egolint is also its own reference consumer. With Docker, Node.js, pnpm, Rust, and Task available,
run:

```sh
task dogfood
```

That single developer entrypoint exercises the public `egolint validate` CLI, Egolint's JavaScript
dependency-architecture and package-quality adapters, then the public `egolint lint` CLI using a
full policy image built from the current checkout. The lint image uses `pull-policy = "never"` and
`network = "none"`, so the proof cannot silently fall back to a previously published image or
networked lint execution.

The same task runs in the read-only `Dogfood` GitHub Actions workflow and emits normalized evidence
under `.reports/egolint`. See [dogfooding](docs/dogfooding.md) for the exact proof boundary and
exception policy.

## Configuration

Copy [`examples/egolint.toml`](examples/egolint.toml) into a repository and run:

```sh
egolint explain
egolint plan --format "json"
```

Configuration is deterministic and explainable. Later sources override earlier ones: compiled
defaults, user config, repository config, local config, explicit config, `EGOLINT_*` environment
variables, and CLI options. User and local files are skipped when `CI` is truthy. See
[configuration](docs/configuration.md) for the exact rules.

## Documentation

- [Architecture](docs/architecture.md)
- [Identity Brand Kit](docs/identity.md)
- [Configuration](docs/configuration.md)
- [Dogfooding and self-consumer proof](docs/dogfooding.md)
- [Versioned consumer integrations](integrations/README.md)
- [Machine-readable contracts](docs/contracts.md)
- [Repository Intelligence validation](docs/repository-intelligence.md)
- [Repository presentation validation](docs/repository-presentation.md)
- [Containers and image boundaries](docs/containers.md)
- [Security model](docs/security.md)
- [Release design](docs/releasing.md)
- [Migration from Empathy](docs/migration-from-empathy.md)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [Changelog](CHANGELOG.md)

## License boundary

Egolint's Rust code and first-party policy are licensed under the [MIT License](LICENSE). The full
image extends and contains MegaLinter, which is licensed under AGPL-3.0-only, and also contains
third-party linters under their own licenses. Building or distributing the full image does not
relicense those components as MIT. See [NOTICE](NOTICE) and [containers](docs/containers.md).

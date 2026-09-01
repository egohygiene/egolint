# Configuration

Every TOML file must declare `config-version = 1`; unknown keys are rejected.

## Precedence

Sources apply in this order, with later scalar values replacing earlier ones:

1. Compiled defaults.
2. `$XDG_CONFIG_HOME/egolint/config.toml`, or `$HOME/.config/egolint/config.toml` when
   `XDG_CONFIG_HOME` is unset.
3. `egolint.toml` in the workspace.
4. `.egolint.toml` in the workspace.
5. `.egolint.local.toml` in the workspace.
6. The file supplied with `--config`.
7. Supported `EGOLINT_*` environment variables.
8. CLI options.

When `CI` is nonempty and is neither `0` nor `false`, the user and local files at steps 2 and 5 are
skipped. Environment maps merge by key; later values win. Run `egolint explain` to see the effective
values and applied sources. `egolint config explain` remains a compatibility form. Run
`egolint plan` to emit a redacted execution plan. Run `egolint validate` to evaluate native
portability rules and any repeatable `--repository-contract` inputs without requiring Docker or
Podman.

## Fields

| Key                 | Values                                            | Default                                |
| ------------------- | ------------------------------------------------- | -------------------------------------- |
| `profile`           | `fast`, `holistic`, `security`, `dependency-debt` | `fast`                                 |
| `runtime`           | `auto`, `docker`, `podman`                        | `auto`                                 |
| `image`             | Nonempty OCI reference                            | `ghcr.io/egohygiene/egolint-full:edge` |
| `pull-policy`       | `missing`, `always`, `never`                      | `missing`                              |
| `network`           | `none`, `bridge`                                  | `none`                                 |
| `megalinter-config` | Existing workspace-relative YAML path             | Unset; use image profile               |
| `environment`       | Map of valid environment names to strings         | Empty                                  |

Supported environment overrides are `EGOLINT_IMAGE`, `EGOLINT_PROFILE`, `EGOLINT_RUNTIME`,
`EGOLINT_PULL_POLICY`, `EGOLINT_NETWORK`, and `EGOLINT_MEGALINTER_CONFIG`.

Reports always use `.reports/egolint`. This is intentionally not configurable: `check` overlays that
one dedicated path read-write beneath an otherwise read-only workspace. Egolint rejects symlinks or
canonical aliases in the fixed report path so it cannot become a write capability for source or Git
metadata.

`environment` values are forwarded to the lint container and redacted from the public plan. Egolint
rejects values that try to replace adapter-owned controls or inject MegaLinter command/plugin
settings. The map is not a secret store: values may still be observable through the runtime or a
linter. Use the CI platform's secret mechanism and grant only the minimum required values.

## Repository policy replacement and extension

`megalinter-config` and the CLI override `--megalinter-config` must point to a file inside the
workspace. Supplying one replaces the selected embedded profile for that invocation. For small
changes to an embedded profile, prefer the repeatable `--enable-linter` and `--disable-linter`
options.

Repository configurations can inherit other repository files. MegaLinter v10 resolves every local
`EXTENDS` entry from the workspace root, regardless of the including file's directory. For example,
`.egolint/megalinter.yml` can contain:

```yaml
---
EXTENDS:
  - egolint-base.yml

DISABLE_LINTERS:
  - COPYPASTE_JSCPD
```

Here `egolint-base.yml` must be at the repository root, not beside the including file. Egolint
recursively validates the same root-relative files MegaLinter will load. Remote URLs, absolute
paths, workspace escapes, cycles, YAML merge keys, plugins, and pre/post-command keys are rejected.

MegaLinter v10 cannot use `EXTENDS` to load `/opt/egolint/profiles/*.yml`: it prefixes local entries
with the workspace path. The image therefore flattens the fast profile at build time, and Egolint
rejects that misleading absolute form in consumer policy. A future native policy-overlay contract
can add persistent inheritance from embedded profiles without relying on this upstream path
behavior.

`--repository-intelligence` selects one versioned semantic policy and requires
`--represented-commit` as a full Git SHA, `unknown`, or `not-applicable`. The policy declares its
own incremental source coverage and enabled-rule profile; see
[Repository Intelligence validation](repository-intelligence.md).

`--changed-only`, repeatable `--enable-linter`, and repeatable `--disable-linter` apply to one
invocation and are not persistent TOML fields. Repeatable `--suppression` inputs each name one
versioned suppression JSON document. Suppression evaluation deliberately requires
`--evaluation-date "YYYY-MM-DD"`, making expiry decisions reproducible rather than dependent on an
implicit workstation clock.

Prefer a digest-qualified `image` in protected CI. Tags are convenient for local alpha development
but are mutable.

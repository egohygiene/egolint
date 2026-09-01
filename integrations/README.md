# Consumer integrations

Every integration is check-only by default. It does not forward GitHub tokens, publish reports,
mount the container-engine socket, or invoke `egolint fix`.

## GitHub Action

Pin both trust boundaries: use this action at a reviewed full commit SHA and set `image` to an
immutable `ghcr.io/egohygiene/egolint-full@sha256:...` reference. The action compiles the CLI from
that checkout, runs Docker with network access disabled by default, and exposes only local artifact
paths as outputs. It emits the canonical `run-report` and `sarif-report` paths for every completed
run, plus `debt-json` and `debt-markdown` when the `dependency-debt` profile is selected. The raw
MegaLinter JSON and SARIF paths are separate, explicitly private outputs. The action never uploads
or publishes any of them. One repository contract and one reviewed suppression can be supplied
directly through `repository-contract` and `suppression`; a suppression requires the explicit
`evaluation-date` input so expiry is reproducible in CI. The composite action currently targets
Linux runners with Bash, GNU `realpath`, Rustup/Cargo, and Docker; native CLI rule packs remain the
portable path for macOS and Windows workstations.
[`github/action.example.yml`](github/action.example.yml) is a least-privilege consumer workflow with
explicit commit- and image-digest sentinels to replace. Changed-only checks require enough Git
history to compute the comparison; the example therefore uses a full checkout instead of silently
trusting a shallow clone.

Raw MegaLinter JSON, raw MegaLinter SARIF, and linter logs can contain repository paths, source
excerpts, package coordinates, and secret material. Keep them private unless an approved sanitizer
has produced a publication-safe derivative. Canonical Egolint SARIF contains normalized findings
rather than raw adapter payloads, while dependency-debt output is a bounded counts-only derivative.
Organization publication policy still applies; a canonical local artifact is not automatically
approved for public upload.

## pre-commit

Copy [`pre-commit.example.yaml`](pre-commit.example.yaml), replace the sentinel revision with a
reviewed full commit SHA, and configure a digest-pinned image in `egolint.toml`. The hook receives
no filenames because Egolint and MegaLinter own changed-file discovery. Run
`pre-commit run egolint --hook-stage manual` for an explicit check.

## Editors

Copy [`vscode/tasks.json`](vscode/tasks.json) to `.vscode/tasks.json`. The task uses the image
configured in `egolint.toml`; pin that image by digest before relying on the task as a policy gate.
The task contract recognizes normalized diagnostics in this form:

```text
egolint:path/to/file.rs:12:4: error: explanation [TOOL/RULE]
```

The CLI emits this bounded normalized form for findings with a relative source location. Both the
composite action and the VS Code task register the matching problem matcher, so those findings
become annotations/problems without parsing raw scanner logs. Findings without a safe relative
location remain available in `run.json` and canonical SARIF.

Egolint does not claim an LSP. The task is a deterministic batch check; `egolint: inspect plan` is
the safe way to inspect runtime, mounts, image, and policy before execution. Apply fixes only from
an intentional terminal run after reviewing the plan and worktree.

# Configuration examples

Copy `egolint.toml` to a repository root, then inspect the result before running any linter:

```sh
egolint config explain
egolint plan --format "json"
```

`egolint.local.toml` demonstrates a developer-only override. Local configuration is skipped in CI
and should normally be ignored by version control.

The example image uses the alpha `edge` name because that is the compiled default. No public image
is asserted to exist yet. Once releases begin, pin a reviewed `egolint-full` digest instead of
relying on a moving tag.

## Release source examples

The [actual EgoLint declaration](../.egohygiene/release.json) is a complete incubating Cargo-backed
self-consumer. It preserves a truthful advisory manual-workflow failure because the existing publisher
is tag-driven; copying it does not establish publication readiness. The
[authored release corpus](../tests/fixtures/repository-release/README.md) supplies distinct Rust,
Python, npm, container, site, publication, workspace, contract and archived examples with explicit
source authorities and evidence states. These are source fixtures, not publisher templates.

Use [release validation guidance](../docs/repository-release-validation.md) and the exact selected
Aether/Hygiene contracts before adapting a declaration. The local source planner emits a human review
handoff without choosing a version, rewriting the changelog or creating a release tag.

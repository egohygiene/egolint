# Security model

Egolint runs third-party analyzers against attacker-influenced repository files.
Treat the engine image, its plugins, project configuration, and every enabled
linter as code in the security boundary.

## Alpha safeguards

- The host container-runtime process is constructed as direct argv without
  host-shell interpolation.
- `lint` grants the repository a read-only mount. Only the report path is
  writable. That path is fixed at `.reports/egolint`; symlink and canonical
  aliases are rejected.
- Every run removes its fixed raw and canonical contract artifacts before the
  adapter starts, preventing a failed invocation from reusing stale evidence.
  Raw adapter stdout/stderr is not forwarded to workflow command channels;
  normalized findings are rendered by the parent CLI.
- `fix` materializes both isolated trees from the exact committed Git objects,
  never from ignored or live worktree files. It gives write access only to one
  candidate, discards all container-mutated Git metadata and report data, and
  generates a bounded patch through a second trusted comparison repository.
  The original worktree and its Git metadata are never mounted.
- `apply-fix` is a separate review step requiring the preview's exact SHA-256
  digest, base commit, and expected post-tree. It rechecks a clean worktree,
  uses index-aware `git apply --check`, applies and stages the patch, and
  verifies the resulting tree. The retained patch can be reversed with
  `git apply --reverse --index`.
- The default network is `none`.
- All capabilities are dropped, `no-new-privileges` is enabled, and PIDs are
  limited.
- Explicit MegaLinter configuration and every local `EXTENDS` entry must resolve
  inside the workspace using MegaLinter v10's workspace-root semantics. Remote
  or absolute inheritance, cycles, merge keys, plugins, and pre/post-command
  hooks are rejected.
- Linter IDs accept only ASCII letters, digits, and underscores.
- Adapter-owned environment controls cannot be overridden from repository TOML.
- Environment values are redacted in printed plans and configuration output.
- `run.json` is written through an atomic same-directory replacement after
  every parent and the final target are revalidated; an engine-created target
  symlink fails closed and is never followed.

These controls reduce risk; they do not make untrusted linters harmless. A
read-only mount still exposes repository contents to the container, and enabling
network access can expose those contents externally.

## Operator guidance

1. Pin `egolint-full` and its MegaLinter base by digest in protected CI.
2. Review image provenance, SBOM, and vulnerability results before promotion.
3. Use ephemeral runners for untrusted pull requests.
4. Keep network disabled unless a reviewed linter requires it.
5. Never put secrets in repository TOML or mount a host Docker socket.
6. Review `plan --format "json"`, run `validate`, and inspect `explain` when
   changing policy.
7. Review `fixes.patch`, record its printed digest, base, and post-tree, and
   only then use `apply-fix`. Never collapse preview and approval into one
   unattended step.

## Publishable evidence contracts

The checked-in JSON Schemas constrain structural shape, required fields, enums,
and exact contract versions. They intentionally do not encode every path, date,
text-size, or cross-field invariant. Configuration resolution and Rust semantic
validation used by the CLI additionally reject unsafe relative-path forms,
malformed digests and Gregorian dates, unsupported control characters, and
oversized text in fields with declared budgets. Reports receive those semantic
and cross-field checks before atomic replacement, and their completeness field
distinguishes adapter-exit-only evidence from partial or fully normalized data.

The executable Empathy compatibility fixture is deliberately sanitized. It
records the pinned upstream commit and representative normalized contracts,
but does not copy raw MegaLinter output because that output can contain cached
environment variables and workstation-specific absolute paths.

The holistic image policy contains reviewed, path-only pre-command hooks for
cache and report directory setup. These hooks are part of the image build, not
the repository-controlled configuration surface, and execute through the
inherited MegaLinter Bash entrypoint.
The full image preinstalls its locked Node policy dependencies with lifecycle
scripts disabled so CSpell dictionaries and the ESLint JSON plugin do not
depend on files in the consumer repository.

Report vulnerabilities through the private process in the repository's
[`SECURITY.md`](../SECURITY.md), not a public issue.

# Security model

Egolint runs third-party analyzers against attacker-influenced repository files.
Treat the engine image, its plugins, project configuration, and every enabled
linter as code in the security boundary.

## Alpha safeguards

- The host container-runtime process is constructed as direct argv without
  host-shell interpolation.
- `check` grants the repository a read-only mount. Only the report path is
  writable. That path is fixed at `.reports/egolint`; symlink and canonical
  aliases are rejected.
- `fix` is a separate, explicit operation with a read-write repository mount.
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
- `run.json` is written through an atomic same-directory replacement, so an
  engine-created target symlink is replaced rather than followed.

These controls reduce risk; they do not make untrusted linters harmless. A
read-only mount still exposes repository contents to the container, and enabling
network access can expose those contents externally.

## Operator guidance

1. Pin `egolint-full` and its MegaLinter base by digest in protected CI.
2. Review image provenance, SBOM, and vulnerability results before promotion.
3. Use ephemeral runners for untrusted pull requests.
4. Keep network disabled unless a reviewed linter requires it.
5. Never put secrets in repository TOML or mount a host Docker socket.
6. Review `plan --format "json"` and `config explain` when changing policy.
7. Separate automatic checks from human-approved fixes.

The holistic image policy contains reviewed, path-only pre-command hooks for
cache and report directory setup. These hooks are part of the image build, not
the repository-controlled configuration surface, and execute through the
inherited MegaLinter Bash entrypoint.
The full image preinstalls its locked Node policy dependencies with lifecycle
scripts disabled so CSpell dictionaries and the ESLint JSON plugin do not
depend on files in the consumer repository.

Report vulnerabilities through the private process in the repository's
[`SECURITY.md`](../SECURITY.md), not a public issue.

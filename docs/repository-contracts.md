# Repository contracts

The repository-contract evaluator verifies a local, pinned projection without
fetching another repository during linting. Empathy and Hygiene own the meaning
of their profiles and ecosystem context. Egolint owns only the validation
envelope and normalized findings.

A contract declares:

- `schema-version`, stable ID, version, and profile name;
- whether the projection is provisional;
- source repository, exact 40-character Git commit or blob ID, source path, and
  governing decision;
- exact-case file or directory requirements;
- whether each artifact is generated, merely required, or repository-owned;
- optional Git executable mode and deterministic content markers.

Generated content requires at least one marker. Repository-owned files are
checked for presence and kind without freezing their contents. Symbolic links do
not satisfy file requirements. Every check is offline and deterministic.

The fixtures under `tests/fixtures/contracts/` pin the observed Empathy and
Hygiene sources from August 19, 2026. They are explicitly provisional
compatibility evidence:

- Empathy issue `EMP-02` still owns the universal-file/profile contract.
- Hygiene issue `HYG-04` still owns the generated local ecosystem-context
  contract.

These fixtures must not be published as the canonical upstream contracts. Once
those issues release schemas, replace the provisional projections with pinned
release artifacts and retain the old fixtures only for compatibility tests.

Stable evaluator rules are:

- `EGO-CONTRACT-SOURCE-001` — visible warning for provisional sources;
- `EGO-CONTRACT-FILE-001` — missing, mis-cased, wrong-kind, or wrong-mode file;
- `EGO-CONTRACT-CONTEXT-001` — generated/context marker drift.

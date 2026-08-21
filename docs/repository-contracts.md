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

The fixtures under `tests/fixtures/contracts/` are reviewed, byte-pinned copies
of the canonical Empathy universal-foundation and Hygiene repository-context
artifacts. Both set `provisional = false`, identify their immutable semantic
source commit and path, and validate through the same decoder used by the CLI.
`install-manifest.json` records the local SHA-256 digest and upstream provenance
for drift detection. The old provisional compatibility projections were removed
after EMP-02 and HYG-04 published their version-1 contracts.

## Canonical upstream artifact protocol

Egolint publishes `schemas/repository-contract.schema.json` and the equivalent
`egolint schema repository-contract` CLI surface so upstream owners can author
contracts without depending on Egolint implementation source. A canonical
Empathy or Hygiene artifact must:

- decode against the published version-1 schema and pass Egolint's semantic
  validation;
- set `provisional = false` and use a source-owned stable ID, version, and
  profile or context name;
- identify the upstream repository, an exact 40-character Git commit, the
  repository-relative path of the canonical contract artifact, and its
  governing issue or accepted decision;
- distinguish generated, required, and repository-owned requirements without
  freezing consumer-owned contents;
- give every generated requirement deterministic content markers; and
- be copied or downloaded only during a reviewed installation/update step,
  then consumed locally and offline by Egolint.

The source commit and path are provenance, not a runtime network dependency.
Production lint runs must receive the reviewed local artifact through
`--repository-contract`; they never fetch Empathy, Hygiene, or another sibling
repository while evaluating a workspace. Full semantic validation remains the
trust boundary because JSON Schema cannot enforce normalized paths, exact Git
identifier meaning, cross-field uniqueness, or generated-marker policy.

Stable evaluator rules are:

- `EGO-CONTRACT-SOURCE-001` — visible warning for provisional sources;
- `EGO-CONTRACT-FILE-001` — missing, mis-cased, wrong-kind, or wrong-mode file;
- `EGO-CONTRACT-CONTEXT-001` — generated/context marker drift.

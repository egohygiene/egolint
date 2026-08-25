# Machine-readable contracts

These JSON Schemas are generated from the same Rust types used by the Egolint
CLI. They are checked in so repository templates, Relay workflows, editors, and
Observatory consumers can validate contract structure and exact versions
without compiling Egolint. Schema validation alone is not the CLI trust
boundary: path normalization, real calendar dates, text budgets, digests, and
cross-field consistency are semantic checks performed by the Rust validators.
Consumers requiring those guarantees must invoke Egolint or implement
equivalent semantic validation.

- `config.schema.json` describes `config-version = 1` TOML after TOML-to-data
  decoding.
- `profile.schema.json` describes a selected profile and its policy provenance.
- `finding.schema.json` describes a normalized tool-owned diagnostic.
- `suppression.schema.json` describes a reviewable exception with owner and
  expiry.
- `evidence.schema.json` describes a sanitized workspace-relative evidence
  reference.
- `tool-result.schema.json` describes one precise tool selection or execution
  result.
- `plan.schema.json` describes the redacted execution plan.
- `report.schema.json` describes the normalized run result.
- `debt.schema.json` describes the compact, privacy-bounded dependency-debt
  projection.
- `repository-contract.schema.json` describes the offline, immutable envelope
  used by Empathy profiles, Hygiene ecosystem context, and other source-owned
  repository requirements after TOML-to-data decoding.
- `repository-intelligence.schema.json` describes the offline semantic policy,
  exact Hygiene pins, enabled-rule profile, and incremental source coverage.
- `repository-intelligence-report.schema.json` describes normalized semantic
  validity, represented commit, coverage, diagnostics, and remediation.

Every contract version is constrained to exactly `1` in its generated schema.
The report declares whether it contains adapter-exit-only, partial, or complete
normalized detail; empty finding arrays are never presented as complete adapter
coverage unless `completeness` says `normalized`.

Regenerate and verify them with:

```sh
task schemas:write
task schemas:check
```

Schema changes are API changes. Update fixtures, documentation, and the
changelog in the same pull request.

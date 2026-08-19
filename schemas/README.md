# Machine-readable contracts

These JSON Schemas are generated from the same Rust types used by the Egolint
CLI. They are checked in so repository templates, Relay workflows, editors, and
Observatory consumers can validate contracts without compiling Egolint.

- `config.schema.json` describes `config-version = 1` TOML after TOML-to-data
  decoding.
- `plan.schema.json` describes the redacted execution plan.
- `report.schema.json` describes the normalized run result.

Regenerate and verify them with:

```sh
task schemas:write
task schemas:check
```

Schema changes are API changes. Update fixtures, documentation, and the
changelog in the same pull request.

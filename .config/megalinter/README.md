# MegaLinter policy contract

This directory makes the MegaLinter v10 inventory, profile selection, and
fixture evidence inspectable without starting the container.

## Sources of truth

- `policy.yml` records profile intent, disabled reasons, applicability, config
  ownership, and positive/negative fixture evidence.
- `v10-catalog.json` is a compact import of the official MegaLinter `v10.0.0`
  schema, descriptors, embedded versions, and removed-linter matrix.
- `.mega-linter.yml` at the repository root is the canonical holistic profile.
- `.mega-linter.fast.yml` is the explicit 12-linter changed-file
  profile used by pull requests and tight local loops.

## Generated contracts

- `tool-matrix.json` joins upstream capabilities with repository policy for all
  124 tools in the pinned image. Every entry exposes structured owner,
  policy-source, configuration, fixture, and expected runtime-evidence fields;
  selection reasons remain separate from ownership.
- `snapshots/fast.json` records the exact fast selection.
- `snapshots/holistic.json` records selected and explicitly disabled holistic
  tools.

Do not edit generated JSON by hand. Regenerate and validate it with:

```shell
task lint:contracts
task lint:contracts:write
```

Maintainers importing a future upstream release must use a verified official
MegaLinter checkout:

```shell
python3 scripts/validate_megalinter_policy.py \
  --write \
  --import-upstream /path/to/verified/megalinter
```

The validator rejects unknown variables, deprecated or removed v10 variables,
removed linter selections, missing configs, undocumented disabled tools, stale
snapshots, and target tools without positive/negative fixtures or a named
blocker.

## State semantics

Static configuration distinguishes `enabled`, `conditional`, and `disabled`.
Runtime reporting must preserve the more precise result states recorded in
`policy.yml`, including `not_applicable`, `missing_from_image`,
`configuration_error`, findings, execution failures, and timeouts. A skipped
tool must never be reported as passed.

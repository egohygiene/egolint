# Migration from Empathy

This standalone baseline reconciles the lint subsystem embedded in
`egohygiene/empathy` at commit
`560aff8430c2f170dadae9161a4603a71c41acbf` with the earlier snapshot already in
Egolint `main` at `94f89d4cd3a0d1b63fb7f1b77ac6b0e90967f949`.

The migration is a curated source transfer, not a copy of generated state.

## Included

- MegaLinter fast and holistic policies.
- Language, repository, prose, and security rule configuration.
- The offline MegaLinter v10 catalog, policy, generated tool matrix, and
  profile snapshots.
- The complementary-tool manifest and generated matrix.
- Positive and negative fixtures and their contract tests.
- The compatibility Bash wrapper, focused support scripts, Task modules,
  hooks, and reproducible private Node/Python toolchains.

All nested `egolint/` paths and Empathy-specific exclusions were rebased to the
standalone repository. Runtime evidence and caches now live under
`.reports/egolint/**` and are ignored.

The sanitized executable fixture at
`tests/fixtures/compatibility/empathy-v1/` pins this source commit and verifies
profile resolution, selected-tool counts, and normalized contract round trips.
It preserves behavioral evidence without copying generated process
environments, absolute workstation paths, or unrelated Empathy state.

## Deliberately excluded

- `.agents/**`: generated Aether projections; Aether remains their owner.
- `.reports/**` and Empathy root reports: generated evidence, including stale
  failures whose paths belonged to unrelated Empathy subsystems.
- `.DS_Store`, Dart `.dart_tool/**`, and Raku `.precomp/**`: local/generated
  caches.
- Empathy's secret baseline: `.secrets.baseline` was regenerated from this
  repository and contains only reviewed policy-commit fingerprints.
- Empathy's `.ansible/ansible.cfg` and example inventory: relocated to
  `tests/fixtures/ansible/**`; `.ansible/playbooks/site.yml` remains as the
  repository's intentional self-dogfood project while its fixture copy proves
  the portable policy contract.
- `poetry.toml`: the standalone Python manifest uses the non-package `uv`
  toolchain contract directly.
- Consumer-specific Relay workflows and report publication: Relay and
  Observatory retain those organization-level responsibilities.

The embedded Empathy implementation should be replaced with a thin,
version-pinned Egolint consumer only after this standalone pull request and its
image contract are reviewed.

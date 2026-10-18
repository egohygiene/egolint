# Canonical ADR policy-owner fixtures

These five public authority files are copied byte-for-byte from
[Hygiene `a05b14f8b9e2772124f879f818e09a911e996ad8`](https://github.com/egohygiene/hygiene/tree/a05b14f8b9e2772124f879f818e09a911e996ad8),
policy **1.2.0**, whose specific four-digit naming amendment was approved by the
maintainer and recorded in [Hygiene PR #72](https://github.com/egohygiene/hygiene/pull/72).
They are contract fixtures, not a copied validator implementation.

The [native catalog](../../../.config/rules/repository-intelligence.v1.toml)
locks the four policy/ratification/schema SHA-256 digests. The full contract index
fixture has SHA-256 `2a8c4e114c2964acbf160ed6f0183013f175d07f6752878b9984542596b6622a`;
runtime checks allow unrelated index additions while requiring unique active
ADR authority entries.

Synthetic records use four-digit canonical IDs. Explicit compatibility tests
retain the earlier v1.1.0 ordinary-consumer pin and its three-digit snapshots.
Old canonical-owner runtimes remain reproducible through their exact earlier
EgoLint/Relay revisions; current owner selection uses the new approved authority.
No architecture approval or publication authority is supplied by these fixtures.

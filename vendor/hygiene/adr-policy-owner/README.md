# Canonical ADR policy-owner fixtures

These five public authority files are copied byte-for-byte from
[`egohygiene/hygiene` at `c589587395750cd1c79c6fa0bef010189c547249`](https://github.com/egohygiene/hygiene/tree/c589587395750cd1c79c6fa0bef010189c547249).
They preserve the accepted ADR policy, ratification evidence, both schemas and
the contract index used to test explicit canonical-owner selection in EgoLint #81.

The runtime authority lock is
[the bundled rule catalog](../../../.config/rules/repository-intelligence.v1.toml).
It records the four policy/ratification/schema SHA-256 digests. The complete
contract index fixture has SHA-256
`41e1ceb50382d0fe299ac583dabb9dbad90286966341c3b9ed4e3ddc6582577f`;
runtime validation checks its header and unique ADR authority entries, allowing
unrelated contract additions. The index's umbrella `proposed` status does not
change the two active ADR contracts' accepted authority.

The ADR records used with these files are synthetic EgoLint fixtures from
`tests/fixtures/repository-intelligence/valid`, not Hygiene's real ADR corpus.
No Hygiene implementation or live private data is bundled. The owner fixture
selects the accepted revision and omits the inheritance declaration entirely.

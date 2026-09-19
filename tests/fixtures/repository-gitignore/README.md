# Pinned gitignore compatibility fixtures

`source/` is an exact data-only export of `foundation/catalog.json` and
`foundation/ignore/{universal,rust}.gitignore` from `egohygiene/empathy` commit
`b44f798bb49259f9f48416b4ffebde1103e135c0` (MIT). Their hashes are verified against the compiled
adapter catalog. No source generator runs during native evaluation.

`filament/composition.json` and `scoped-rust/composition.json` are the accepted Holon #58 fixture
inputs from Holon commit `660b941f99618806fcadd589bcdae61c519f96e4`, under
`tests/fixtures/gitignore/`. They retain Empathy's original composition framing and
provenance. The Filament root represents pilot merge `c3eb64b8087face7504e7571dc8396c19525649f`.
The accompanying TOML requests explicitly select the same profiles, scope order, overlays, and
local text; they add native validation metadata rather than changing source policy.

`inherited-policies.json` retains exact path/content/digest evidence from the same Empathy pin:
its devcontainer, Mantle, React-template, and nested EgoLint policies. The known counterexamples
come from `tests/test_gitignore_adoption.py`. These fixtures must produce findings until their
canonical owners reconcile them; inventory review is not an exemption.

`.config/rules/repository-gitignore.v1.json` records the immutable source/hash and six upstream
`tests/test_gitignore_baseline.py` methods from which 115 literal path/expected-state vectors were
exported. The accepted Filament test verifies all vectors against real Git. At runtime, vectors
are expanded at each policy scope and expected behavior is calculated from the selected valid
composition; repository probes add scoped intent such as reachable Rust output exceptions.

Tests create disposable repositories, copy only fixture policies, materialize empty probes, and
use actual Git matching. Tracked-environment fixtures stage only synthetic payloads in disposable
repositories. No fixture requests changes to live Filament, Empathy, or inherited sibling copies.

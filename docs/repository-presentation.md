# Repository presentation validation

Egolint validates the deterministic portions of Hygiene's universal repository-presentation profile.
Hygiene owns applicability and claim semantics, Identity owns reviewed visual packages and integrity
manifests, and Egolint owns local validation and normalized diagnostics. The validator does not
decide whether a repository is good, safe, secure, legally compliant, or genuinely healthy.

The currently supported Hygiene profile is pinned to version `1.0.0-alpha.1`, commit
`cb2ed63425d29abada2d2bbb43a3b3e59d11aeb8`, and normalized SHA-256
`44e0881519350e6747723995939c79c6fb4659e38a74b2c32e409866e7a186ba`. Its authority is still
`proposed`; selecting it does not activate organization policy or manufacture a passing badge.

## Run locally or in CI

Vendor the exact Hygiene profile, commit an Identity package and manifest, then add a
repository-owned policy based on
[`examples/repository-presentation.toml`](../examples/repository-presentation.toml).

```sh
egolint validate \
  --repository-presentation ".config/egolint/repository-presentation.toml" \
  --represented-commit "$(git rev-parse --verify HEAD)"
```

The same command and inputs are used by the composite action. Both paths emit the canonical run
report and SARIF plus a dedicated artifact:

```text
.reports/egolint/repository-presentation.json
```

That report contains contract pins, repository axes, evidence state, validation status, rule IDs,
paths, expected/actual structural states, remediation, and bounded counts. It never contains README
excerpts, alt text, exception reasons, private destinations, or other repository prose. Observatory
can consume it without rereading a private README.

## Validation boundary

The rule pack resolves slot requirements from the vendored profile in the profile's declared order:
default, repository type, visibility, then lifecycle. It validates:

- required semantic markers or equivalent headings;
- one balanced generated region carrying owner and profile metadata;
- a local Identity banner with alt text and fallback metadata;
- exact badge label, state message, profile version, represented commit, and evidence destination;
- evidence/profile/repository/slot agreement, including fail-closed states;
- Identity package and manifest schemas, profile/evidence bindings, byte counts, and SHA-256 values;
  and
- repository-relative README and evidence destinations without following symlinks.

HTTPS destinations are counted but never fetched by this deterministic validator. The report records
`external_reachability` as `not_checked`, and `EGO-PRESENT-EXTERNAL-001` keeps that limitation
visible. A separately authorized network job may assess reachability without changing Egolint's
local result.

`blocking` mode preserves catalog severities. `advisory` mode caps diagnostics at warnings while the
dedicated report still records `invalid` or `incomplete`. Honest evidence states behave as follows:

| Evidence state                                                     | Validation interpretation                                                                            |
| ------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------- |
| `passing`                                                          | Valid only when all required slots carry current passing evidence and every local binding validates. |
| `failing`                                                          | Invalid; the badge remains visibly failing.                                                          |
| `unknown`, `evaluating`, `advisory`, `partial`, `stale`, `blocked` | Incomplete and never passing.                                                                        |
| `exempt`                                                           | Valid only with structurally consistent evidence and any required documented exceptions.             |
| `not_applicable`                                                   | Reported distinctly; never converted to passing.                                                     |

## README composition and autofix

Generated presentation markup must stay inside one region such as:

```markdown
<!-- repository-presentation:begin owner=egohygiene/identity profile=1.0.0-alpha.1 -->

![Project repository identity banner](assets/identity/assets/banner-light-640.svg)
[![Hygienic: evidence unknown](assets/identity/assets/hygienic-unknown.svg)](evidence/repository-presentation.json)
<!-- repository-presentation:end -->
```

Repository-authored sections can use stable markers such as
`<!-- repository-presentation:slot purpose -->` or equivalent headings. Egolint does not replace
README files or prose. The existing `fix` workflow may normalize unambiguous formatting in other
rule packs, but repository-presentation validation currently offers no automatic mutation;
generated-marker or package changes must come from a reviewed Identity/Holon/Pace projection.

## Exceptions and migration

Start migration in `advisory` mode with `unknown`, `partial`, or honest failing evidence. Move to
`blocking` only after the repository's own maintainers review its prose, destinations, axes, and
generated package.

When a required slot genuinely cannot apply, set `exceptions-path` to a versioned JSON document:

```json
{
  "schema": "egolint.repository-presentation-exceptions/v1",
  "profileVersion": "1.0.0-alpha.1",
  "representedCommit": "1111111111111111111111111111111111111111",
  "exceptions": [
    {
      "slot": "security",
      "reason": "Private reporting is unavailable for this frozen publication.",
      "evidence": "docs/support-boundary.md"
    }
  ]
}
```

Exceptions are bound to one profile and represented commit, must name a real profile slot, and
require a durable reason and HTTPS or safe local evidence. Egolint records only the slot and count
in generated reports, not the reason or destination. Exceptions do not change Hygiene's profile and
cannot turn missing or stale evidence into a passing badge.

The fixtures under `tests/fixtures/repository-presentation` cover minimal, customized, private,
archived, partial, broken, and fully conformant adoption shapes.

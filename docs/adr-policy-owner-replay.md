# Canonical ADR policy-owner replay: EgoLint #81

On 2026-10-18, the baseline and candidate native CLIs validated the same immutable Hygiene source.
The candidate removed exactly the false inheritance finding. All other normalized diagnostics were
identical, and every tracked source file retained its SHA-256. Neither run repaired or approved a
Hygiene ADR.

| Input                      | Revision or selection                                                                       |
| -------------------------- | ------------------------------------------------------------------------------------------- |
| Hygiene checkout           | `b8d2c02368e40d4f6c2017bb1a22c6ec23323ad3`                                                  |
| Baseline EgoLint           | `2d3600f14848e28099acc34ce8043699da2b9a32`                                                  |
| Candidate EgoLint          | This PR's reviewed head; catalog `0.1.0-alpha.4`                                            |
| Accepted ADR authority     | Hygiene `c589587395750cd1c79c6fa0bef010189c547249`                                          |
| Enabled rules              | All eleven rules in the fixture profile, blocking                                           |
| ADR surface                | Present; canonical directory and index                                                      |
| Other surfaces             | Roadmap and commit-history adoption explicitly not applicable to this focused source replay |
| Cross-repository allowlist | Empty in both runs; no externally verified decisions supplied                               |
| Local history              | Shallow; one commit available, so both reports retain the history-incomplete warning        |

The four pinned authority artifacts are byte-identical at the accepted authority and represented
Hygiene revisions. The broader catalog has additional contracts at the represented revision; both
ADR authority entries remain unchanged.

| Native result                       | Baseline        | Candidate owner selection |
| ----------------------------------- | --------------- | ------------------------- |
| Exit / semantic status              | `1` / `invalid` | `1` / `invalid`           |
| ADR files present / parsed records  | 13 / 12         | 13 / 12                   |
| Diagnostics / blocking diagnostics  | 17 / 16         | 16 / 15                   |
| Missing policy-reference diagnostic | 1               | 0                         |
| Other metadata diagnostics          | 2               | 2                         |
| Lineage diagnostics                 | 12              | 12                        |
| Index diagnostics                   | 1               | 1                         |
| History-incomplete warning          | 1               | 1                         |
| Canonical-owner authority findings  | Not supported   | 0                         |

The removed finding is `EGO-INTEL-ADR-METADATA-001` at `docs/decisions/policy-reference.json`:
`declared ADR policy-reference file is missing`. No self-inheritance file was created. The remaining
findings are:

- Historical ADR-0001 has no YAML front matter. Its original four-digit identity and body remain
  intact. The missing parsed record also leaves its index entry and eleven local decision references
  unresolved in the native report.
- ADR-004 lacks the required `Implementation and evidence links` section.
- ADR-013 references `egohygiene/identity#ADR-017`, which was not in the supplied external
  allowlist. This identifies undeclared evidence, not proof that the external decision is absent.
- The shallow checkout leaves history incomplete even in this focused replay.

Hygiene [#47](https://github.com/egohygiene/hygiene/issues/47) owns source migration and the
reviewed disposition/exception evidence for historical ADR-0001. Proposed decisions can remain
proposed. This replay establishes owner compatibility; the corpus remains invalid and no publication
or deployment readiness is inferred.

## Repeat the focused comparison

Use isolated, trusted checkouts at the revisions above. Build the two pinned EgoLint binaries with
`cargo build --locked --bin egolint`. Prepare both policies from
`tests/fixtures/repository-intelligence/policy-owner/policy.toml`:

1. Set `known-external-decisions = []`, commit-history `state = "not-applicable"` and
   `maximum-commits = 50000`. Keep the fixture's roadmap not applicable.
2. For the baseline only, remove the entire `[adrs.policy-owner]` table and add
   `policy-reference = "docs/decisions/policy-reference.json"` under `[adrs]`.
3. Place each selected policy at `.config/egolint/adr-owner-replay.toml` in the disposable Hygiene
   worktree, outside its tracked source. Do not add a reference file or edit any ADR.
4. Run each binary with the same arguments below. Capture the dedicated JSON report separately after
   each run; both exit 1. Compare the complete ordered diagnostic arrays after removing only the
   baseline missing-reference finding. Compare tracked-source SHA-256 digests before and after both
   runs.

```sh
egolint --workspace /review/hygiene validate \
  --repository-intelligence .config/egolint/adr-owner-replay.toml \
  --represented-commit b8d2c02368e40d4f6c2017bb1a22c6ec23323ad3
```

Report: `.reports/egolint/repository-intelligence.json`. These paths describe a disposable review
setup, not a claim about the reader's filesystem. A complete local history can remove the
shallow-history warning; record that input difference.

Relay [#144](https://github.com/egohygiene/relay/issues/144) must subsequently adopt the exact
reviewed EgoLint head in both architecture and ADR runtime profiles, verify acquisition receipts,
and repeat owner plus ordinary-consumer tests through its collector. The existing publication-denied
review interface and required-mode gates remain separate. Keep EgoLint #81 open for that downstream
acceptance.

# Security and dependency-debt evidence

Egolint ships executable MegaLinter overlays for focused security and
dependency-debt checks:

- `.mega-linter.security.yml` selects source, infrastructure, and secret checks.
- `.mega-linter.dependency-debt.yml` selects vulnerability, inventory, lockfile,
  and SBOM tools.

The overlays are copied into `egolint-full`, flattened at image-build time, and
selectable through `--profile security` and `--profile dependency-debt`. They do
not make a freshness promise: OSV-Scanner, Grype, and Trivy require explicit
network access to refresh databases, or a reviewed pre-seeded cache whose
timestamp is stored with the run evidence. A network-disabled run without that
evidence is not a current vulnerability scan.

The machine-readable ownership contract is
`.config/security/scanner-ownership.json`. It makes the authoritative scanner,
advisory overlap, organizational owner, evidence boundary, and waiver policy
explicit. OSV-Scanner owns the blocking dependency-vulnerability decision;
Grype and Trivy remain advisory cross-checks to avoid duplicate/conflicting
gates.

## Debt summary design

The alpha `DebtReport` in `src/debt.rs` derives a compact, counts-only summary
from a validated `RunReport`. It includes contract and policy versions; profile
and source-report identity; counts by canonical tool, normalized severity, and
suppression state; and an explicit `fresh`, `stale`, or `unknown` state for each
network-backed vulnerability source. Freshness remains `unknown` until an
adapter supplies a reviewed database timestamp. The type deliberately excludes
finding messages, source paths, fingerprints, package coordinates, environment
values, and raw adapter payloads, with a regression test using sensitive
sentinels.

Every completed `egolint lint --profile dependency-debt` run atomically writes
`.reports/egolint/run.json`, canonical `.reports/egolint/egolint.sarif`, and the
derived `.reports/egolint/debt.json` and `.reports/egolint/debt.md` summaries.
When every active scanner reconciles across the supported JSON and SARIF
envelopes, the summaries say `normalized`. Otherwise they say `partial`, count
only observed normalized records, and carry an explicit warning that missing
coverage may hide additional debt. Adapter-exit-only evidence cannot produce a
debt summary. A six-tool pipeline regression exercises the complete normalized
contract without publishing private scanner payloads.
Later versions may add ecosystem/owner aggregates, remediation age buckets,
expiring-suppression counts, and hashes of private evidence only after their
privacy and stability contracts are specified.

Regression tests prove the compact debt projection does not copy absolute paths,
source snippets, environment values, URLs containing credentials, unredacted
secret matches, private package-registry coordinates, or arbitrary tool
payloads. This establishes the repository-level sanitizer boundary, not
organization approval to publish. `run.json` is canonical local evidence, but
it is not automatically safe to publish. Raw artifacts are private-by-default
and short-lived.

## Suppression lifecycle

Suppressions use `schemas/suppression.schema.json` and require an owner,
justification, calendar expiry, rule selector, state, and sanitized evidence.
They may additionally narrow by workspace-relative path or exact fingerprint.
The local contract requires a calendar expiry and the rule engine treats an
expired suppression as blocking. The ownership contract delegates the declared
90-day maximum renewal window to Hygiene; organization-level enforcement of
that maximum remains a cross-repository gate.

## Verified profile integration

The profile enum, plan projection, image path mapping, v10 catalog validation,
policy declarations, deterministic selection snapshots, Rust tests, Python
contract tests, and generated schemas all treat `security` and
`dependency-debt` as first-class profiles. `Dockerfile.full` packages all four
embedded profiles. The release workflow rebuilds and compares every generated
schema, including the debt summary schema, before promotion.

## Closure gates outside this repository

Issues about production distribution or adoption remain open until all of these
are observed, not merely configured:

1. One immutable release tag completes candidate verification, artifact
   attestations, image signing, and post-push digest smoke tests.
2. Empathy consumes a pinned action commit and full-image digest.
3. At least one non-Empathy repository (Optiflow is the intended pilot) consumes
   those same immutable interfaces.
4. Observatory accepts a sanitizer-tested debt summary without receiving raw
   scanner output.

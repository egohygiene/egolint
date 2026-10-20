# EgoLint completeness audit — 2026-10-20

**Report:** `egolint-completeness-20261020T222416Z`  
**Status:** complete for the declared public repository scope, with the execution and publication
limits below.  
**Audited repository:** `egohygiene/egolint`  
**Immutable source:**
[`834a230e6c0f4ee8b080a10d6c2d469be5eb6e1b`](https://github.com/egohygiene/egolint/tree/834a230e6c0f4ee8b080a10d6c2d469be5eb6e1b)  
**Source
tree:** `a0191cf19d18575f0dcc483079f1c0d9974f1b59`  
**Evidence:** [normalized validation receipt](data/egolint-completeness-20261020T222416Z.validation.json)  
**Execution
ledger:** existing [roadmap #42](https://github.com/egohygiene/egolint/issues/42)

## Executive summary

EgoLint is a source-buildable MegaLinter quality distribution with a substantial native control
plane, checked-in contracts, consumer integrations and meaningful self-consumer validation. It
already follows the intended product boundary: use established analyzers, centralize defaults and
orchestration, and add first-party repository checks where those tools do not provide the required
semantics.

The current baseline is green. Actual Rust 1.85.1 formatting, strict Clippy, build and 195 tests
pass; Python 3.12.14 with PyYAML 6.0.3 passes 75 tests and five source/policy/distribution
validators; Node 24.19.0 and the declared pnpm 11.21.0 pass 11 tests. All 23 Rust-generated schemas
match their checked-in bytes, all 27 local schemas pass meta-validation, and Cargo package
verification passes offline. Three real local self-consumer entrypoints also exit 0. GitHub reports
all five exact-main push workflows and all 12 jobs successful, including actual image builds, native
rules on three operating systems, and the complete public dogfood task.

The negative cases show why that baseline is not enough to declare the alpha finished. Routine
supported repository shapes break fix preview, focused reports can outlive the invocation that
produced them, and several handwritten validators disagree with the contracts they claim to check.
Some adapter evidence is treated as a clean verdict without preserving a failed subprocess outcome
or unsupported envelope. Current navigation, immutable contract instructions and platform examples
also need reconciliation.

The public findings map to focused repairs rather than a replacement architecture. Preserve all 27
original work scopes, create the reviewed focused issue homes, and refresh #42 as the single
complete repository ledger. The proposed combined ledger contains **44 public work items: 24
independently startable and 20 with identified full-outcome gates**; the parent itself is excluded.
All original issues, titles, labels, comments, coordination links and acceptance remain historical
evidence. Issue creation and native attachment are live publication operations whose final state
must be verified separately from this immutable source assessment.

Start with current behavior and evidence correctness. Then finish the existing release-v1
fixture/dogfood chain, reconcile the selected policy corpus and formatter ownership, and qualify the
bounded first release. The optional capability catalog remains visible without making every
experiment a prerequisite for a usable core.

## Request, scope and constraints

The user authorized a holistic repository audit, issue-coverage reconciliation, focused gap issues
and a refresh of existing roadmap #42. This report records the observation phase. Its accompanying
authorized publication workflow may add the report/evidence, update the operational continuity
checkpoint, and maintain the issue ledger. It does not repair implementation, promote a policy,
alter accepted decisions, merge a PR, publish a release, deploy, or perform fleet adoption.

Defaults recorded for this audit:

| Request dimension | Selection                                                                                                                                                                                    |
| ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Audit name        | `egolint-completeness`                                                                                                                                                                       |
| Strategy          | Holistic: implementation, contracts, evidence, tests, packaging, CI, documentation and developer experience                                                                                  |
| Depth             | Standard repository-wide discovery with deeper source review and bounded negative probes                                                                                                     |
| Included          | Public tracked source, manifests, policy/vendor locks, fixtures, schemas, integrations, workflows, root architecture documents and public issue/PR relationships                             |
| Runtime checks    | Linux source build, local native/JavaScript/Python checks, disposable synthetic consumers and exact-head hosted run/job/step evidence                                                        |
| Outputs           | Immutable Markdown report, normalized public receipt, portable public probes, reconciled proposed issue homes and resumable execution ledger                                                 |
| Excluded          | Private incubator contents, credentials, conversation bodies, raw scanner logs/artifact payloads, private inventory, unrestricted network scanning and publication/administration operations |
| Repair policy     | No implementation repair or dependency upgrade in the audit; recommendations remain open work                                                                                                |

A separately governed private follow-up under [SECURITY.md](../SECURITY.md) remains a release review
gate. This public report contains no technical details of that follow-up and does not substitute a
public issue for the repository's private reporting process.

## Repository context and canonical ownership

The frozen tree contains **719 tracked files / 3,998,007 bytes**, 27 local schema documents, 31
production Rust source files, nine Python test modules, six workflows, 24 root Markdown files and 18
provisional root architecture documents. No tracked `audits/`, `.staging/` intake, Git tag or GitHub
release was present in the initial inventory; there were 27 open work issues plus roadmap #42 and no
open PRs.

The current product has two deliberately different image boundaries. The lightweight image contains
the native CLI and runs as UID/GID 65532; it does not contain a Docker/Podman runtime and is not a
supported container-socket wrapper. The full engine image extends MegaLinter v10.0.0, installs the
locked policy toolchain and optional pinned JSONSkooma graph, renders self-contained profiles and
preserves MegaLinter's entrypoint. The native host CLI owns safe planning, configuration, execution
and normalization. The general adaptive registry and distribution-wide adoption modes are still
tracked by #35 and #36; focused validators already implemented in source must not be mistaken for
completion of that broader program.

Keep ownership explicit:

| Owner               | Responsibility used in this assessment                                                                          |
| ------------------- | --------------------------------------------------------------------------------------------------------------- |
| Hygiene             | Organization policy, applicability, lifecycle and accepted ADR authority                                        |
| Aether              | Reusable methodology and portable source contracts                                                              |
| Empathy             | Repository requirements/compositions and reviewed source profiles                                               |
| Holon               | Deterministic materialization, scaffolds and presentation                                                       |
| EgoLint             | Native validation, analyzer adapters, capability/profile selection, normalized local evidence and fix lifecycle |
| Relay               | Reusable GitHub Actions execution and governed evidence publication                                             |
| Observatory         | Derived read models and presentation of permitted evidence                                                      |
| Pace                | Reviewed rollout, convergence and adoption coordination                                                         |
| Consumer repository | Authored content, actual source checkout, approved exceptions, private state and final actions                  |

The audit does not move these responsibilities into a new umbrella tool. Existing contracts and
first-party rules should be made consistent at their current boundaries before adding more optional
adapters.

## Governing guidance and authority

Guidance was frozen at Aether
[`dc8a16851f452763ef4f7fbe4bf057d4500b1921`](https://github.com/egohygiene/aether/tree/dc8a16851f452763ef4f7fbe4bf057d4500b1921):

| Artifact                               | Selected version / lifecycle | Use                                                                                           |
| -------------------------------------- | ---------------------------- | --------------------------------------------------------------------------------------------- |
| `auditor` specification                | 2.0.0, approved              | Scope, discovery, findings, severity, evidence, positive observations and immutable reporting |
| `audit-repository` skill               | 1.1.0, draft                 | Read-only audit procedure                                                                     |
| `maintain-repository-continuity` skill | 1.1.0, draft                 | Resume before selection; separately authorized handoff refresh after domain validation        |
| `github-issue-authoring` skill         | 1.2.0, experimental          | Reviewed issue scope, coverage, dependencies and copy-ready authoring                         |

Root AGENTS and CONTINUITY were read before selecting work. The root handoff describes a four-digit
validator candidate that has since merged; the source and public ledger were checked rather than
treating that prose as current authority. The report identifies this drift; a later candidate
continuity refresh must record its own review state without changing the historical finding.

Accepted Hygiene ADR policy 1.2.0 and four-digit canonical identifiers are already selected by the
current native catalog. Historical supported source tuples and owner fixtures retain their original
meanings and bytes. The continuity projections still identify draft/proposed, unreleased inputs;
observe-mode checks and local structural repairs do not authorize ratchet/enforce promotion.
Existing #55 owns the remaining released-input acceptance, and #60 retains its explicit scheduling
gate for ADR-history backfill.

**ADR not required for the audit publication:** the bounded change records evidence, reconciles work
tracking and refreshes an operational handoff. It does not make a new product or policy decision.

## Methodology and evidence handling

Discovery proceeded through entry documentation, architecture/system context, governance/decisions,
agent instructions and portable guidance, automation/manifests, source, tests, documentation and
prior-audit inventory. The declared 18-document graph and bounded Markdown link checks establish
structure only; they do not certify every sentence, anchor or conceptual dependency.

Independent review lanes covered core configuration/planning/fix behavior; continuity, repository
contracts, portability and ignore semantics; Intelligence/presentation/release semantics; analyzer
normalization and optional adapters; distribution/CI/packaging; and the entire open issue ledger.
Findings were compared with source lines, selected contracts, focused fixtures and observed runtime
results. Source concerns without a concrete counterexample or explicit unmet requirement remain
limitations or planning notes.

The actual source-built native binary used for the final independent root replays has SHA-256
`1c6f7b75691b2a4faa3dba15f3b02c59c80b7ddf5982e338c2be18048ad86ba2`. Cargo package verification
briefly rebuilt a package-context output in the shared build location; the source binary was
restored and its identity checked before the root replays. All five public probe families then
agreed on normalized case outcomes. One earlier Intelligence receipt differed only in the recorded
binary identity, so the final root replay is the primary result for that family. Source/package
build identity and behavior agreement are distinct evidence.

Synthetic inputs are intentionally small and bounded. A fake Docker executable or analyzer
subprocess proves orchestrator behavior at a controlled seam; it does not prove a real engine
produced that shape or qualify a container. Actual CLI, actual native rules, exact local schema
comparisons, static source inspection and hosted provider outcomes are identified separately in each
finding and the receipt. The controls that reject or accept a case are retained alongside the
negative case.

No raw scanner artifact, arbitrary process environment, unrelated provider comment or private
execution root is copied into the public evidence. Normalized outputs preserve
exit/status/coverage/count or integrity facts needed to assess a finding. A hash identifies bytes;
it does not imply public access to omitted logs, authenticated approval, semantic correctness,
release availability or adoption.

## Overall assessment and validation

| Check family                  | Observed result                                                                          | Practical limit                                                                                     |
| ----------------------------- | ---------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| Rust 1.85.1                   | Formatting, locked binary build, strict Clippy and all 195 tests pass                    | Linux source execution; dedicated hosted native-rule jobs cover additional OSes                     |
| Python 3.12.14 / PyYAML 6.0.3 | 75 tests and five policy/source/distribution validators pass                             | Baseline fixtures do not cover every malformed input demonstrated below                             |
| Node 24.19.0 / pnpm 11.21.0   | Commit-policy 4, ESLint policy 1 and package-quality 6 tests pass: 11 total              | The declared exact pnpm was used; adapter stub cases are separately qualified                       |
| Generated schemas             | All 23 generated outputs exactly match checked-in bytes                                  | Generator parity does not prove runtime schema enforcement                                          |
| Schema meta-validation        | All 27 local schema documents pass                                                       | No exhaustive instance or arbitrary remote-reference validation                                     |
| Cargo distribution            | `cargo package --locked --offline` passes                                                | Package verification is not a published crate, signed release or consumer installation              |
| Native self-consumer          | Actual public `egolint validate` entrypoint exits 0                                      | Observe/advisory, unreleased-source and unavailable-evidence states remain visible                  |
| JavaScript self-consumers     | Actual dependency-architecture and package-quality entrypoints exit 0                    | These source consumers do not establish every optional adapter or distribution channel              |
| Public negative probes        | Core, distribution, Intelligence, validators and adapters independently replayed by root | Expected reproduction confirms existing defects, not remediation                                    |
| Integration bundle            | Two fresh epoch-0 archives identical; 11 members, all 10 source hashes valid             | Assembly/integrity proof, not arbitrary extracted integration execution                             |
| Declared architecture         | 18 unique IDs, all declared edges resolve, no cycle; 83 bounded relative links resolve   | Excludes code fences, fixtures, vendored/generated Markdown, anchors, external URLs and prose truth |
| Exact-head hosted validation  | Five successful workflows / 12 successful jobs                                           | Recorded source jobs, not a local container rerun or unpublished release-promotion proof            |

The native self-consumer's zero exit is not a clean semantic certification. Seven existing roadmap
state/parser warnings remain; continuity reports retain unreleased-source/live-evidence limitations,
and release evidence remains unavailable. Those explicit outcomes should survive the roadmap cleanup
and future enforcement work.

The exact-main hosted runs are CI
[39168878713](https://github.com/egohygiene/egolint/actions/runs/39168878713), Dogfood
[39168878735](https://github.com/egohygiene/egolint/actions/runs/39168878735), JavaScript package
quality [39168878683](https://github.com/egohygiene/egolint/actions/runs/39168878683), JavaScript
architecture [39168878760](https://github.com/egohygiene/egolint/actions/runs/39168878760), and
Identity [39168878756](https://github.com/egohygiene/egolint/actions/runs/39168878756). Their
recorded steps include real CLI/full-engine image builds and smokes, profile/Node/JSONSkooma checks,
package verification, native rules on Ubuntu/macOS/Windows, and the complete public dogfood
entrypoint. Docker/Podman is unavailable locally; those results are attributed to hosted evidence.

There is no historical repository audit to compare. All findings are new to this audit ledger. “New”
does not mean that a behavior was introduced by the last commit; it means no prior immutable audit
disposition was available.

## Findings summary

**21 public findings: 15 medium and 6 low.** All are confirmed observations with high confidence
within their declared scope; 17 are confirmed defects and four are documentation gaps. The
separately governed private release-review gate is excluded from these public finding and issue
counts.

| ID        | Severity | Finding                                                                                                    | Proposed home                  |
| --------- | -------- | ---------------------------------------------------------------------------------------------------------- | ------------------------------ |
| AUDIT-001 | medium   | Isolated fix preview cannot use a supported custom MegaLinter configuration                                | `FIX_PREVIEW`                  |
| AUDIT-002 | medium   | A committed executable file prevents isolated snapshot initialization                                      | `FIX_PREVIEW`                  |
| AUDIT-003 | medium   | Focused canonical native reports can survive unrelated or failed later runs                                | `NATIVE_REPORT_LIFECYCLE`      |
| AUDIT-004 | medium   | Presentation validation can accept missing checksum coverage and a missing banner descriptor               | `PRESENTATION_INTEGRITY`       |
| AUDIT-005 | medium   | ADR and roadmap validators count fenced examples as live Markdown anatomy                                  | `MARKDOWN_SOURCE_GRAMMAR`      |
| AUDIT-006 | medium   | ADR link validation accepts empty GitHub identities and a hostless HTTP destination                        | `ADR_REFERENCE_GRAMMAR`        |
| AUDIT-007 | medium   | Release Task handoff validation treats echoed or commented commands as exact workflow dispatch             | `RELEASE_TASK_BINDING`         |
| AUDIT-008 | medium   | A contradictory not-applicable represented commit can still produce a valid Intelligence report            | `INTELLIGENCE_ADOPTION_STATUS` |
| AUDIT-009 | medium   | Make native continuity validation agree with its exact pinned Aether schema                                | `CONTINUITY_SCHEMA_PARITY`     |
| AUDIT-010 | medium   | Detect case-folded file/directory conflicts in native portability inventory                                | `PORTABILITY_PATH_PREFIXES`    |
| AUDIT-011 | low      | Recognize static multi-OS matrix.include rows in the workflow shell check                                  | `PORTABILITY_INCLUDE_MATRIX`   |
| AUDIT-012 | medium   | JavaScript package-quality normalization can turn failed or unsupported adapter evidence into a clean pass | `JS_ADAPTER_EVIDENCE`          |
| AUDIT-013 | low      | JavaScript consumer manifest validation accepts fields forbidden by its published schema                   | `JS_MANIFEST_CONTRACT`         |
| AUDIT-014 | medium   | JSONSkooma selector crashes on malformed mode types and accepts a boolean version                          | `JSON_SKOOMA_CONFIG`           |
| AUDIT-015 | medium   | JSONSkooma relative report location changes with the activation branch                                     | `JSON_SKOOMA_REPORT_ROUTING`   |
| AUDIT-016 | medium   | Release input validation accepts non-object image and product records                                      | `RELEASE_INPUT_CONTRACT`       |
| AUDIT-017 | low      | Missing requested native policy files use the internal-error exit category                                 | `CLI_INPUT_EXIT`               |
| AUDIT-018 | medium   | ADR migration documentation retains a superseded schema digest and unchanged-version instructions          | `CURRENT_DOCS`                 |
| AUDIT-019 | low      | Container guidance contradicts the reviewed full-engine platform boundary                                  | `CURRENT_DOCS`                 |
| AUDIT-020 | low      | Current source navigation and architecture inventory lag merged implementation                             | `CURRENT_DOCS`                 |
| AUDIT-021 | low      | Roadmap #42 does not represent the complete current open backlog                                           | #42 (existing)                 |

Classification is evidence-based: failed schema/semantic controls are reported as defects, current
prose/ledger contradictions as documentation gaps. Severity describes consequence; it is separate
from confidence, the rule's current enforcement severity, and process exit status.

## Findings by severity

## Medium severity

### AUDIT-001 — Isolated fix preview cannot use a supported custom MegaLinter configuration

**Classification:** confirmed defect. **Severity:** medium. **Confidence:** high. **Status:**
confirmed.

**Area:** configuration and isolated fix preview. **Effort:** small. **Impact:** medium.
**History:** new audit observation.

**Observed:** `Config::resolve` validates a `megalinter-config` and stores its normalized
workspace-relative path (`src/config.rs:459–462`). `resolved_for_isolated_copy` subsequently
requires that relative path to have the absolute original workspace as a prefix
(`src/fix.rs:267–273`). Every supported nonempty custom selection therefore fails this conversion.
Its subsequent candidate-absolute rewrite would also conflict with `ExecutionPlan`'s relative input
contract.

**Evidence (observed):**

- public core case `fix_custom_config.custom` in the receipt: In the probe, a clean repository
  containing committed `egolint.toml` and `.megalinter.yml` runs `fix` with an explicit linter
  selection. It exits 2 before any simulated runtime invocation, with the isolated-prefix
  configuration error and no preview patch. The otherwise equivalent default-configuration control
  exits 0, invokes the simulator once, and produces a patch. Both preserve the original source and
  clean index.

**Why it matters:** Repositories using the documented override/extension surface cannot use fix
previews, even though planning and normal linting accept the same configuration. This is a
functional failure, not an out-of-workspace write or exploit claim.

**Recommended:** Preserve one documented relative-path representation through resolution and
candidate planning. Resolve the selected committed config and its inheritance against the isolated
candidate without reintroducing absolute paths or consulting live ignored inputs.

**Suggested validation:**

- End-to-end default and custom config previews, with a committed root-relative inherited policy,
  prove the selected config reaches the candidate runtime, expected patch is generated, original
  content/index stay unchanged, and escaping or unavailable candidate inputs still fail closed.

**Dependencies and risks:** No identified work predecessor for a bounded repair. Retain the existing
immutable-tree, runtime, and report boundaries. No real-image integration qualification was
performed here.

**Issue home:** `FIX_PREVIEW` (reviewed proposed home; see the coverage map below).

### AUDIT-002 — A committed executable file prevents isolated snapshot initialization

**Classification:** confirmed defect. **Severity:** medium. **Confidence:** high. **Status:**
confirmed.

**Area:** Git snapshot construction for fix preview. **Effort:** small. **Impact:** medium.
**History:** new audit observation.

**Observed:** `initialize_repository` creates a new Git repository, then calls
`git update-index --chmod=+x` for executable paths before the initial `git add`
(`src/fix.rs:810–823`). Those paths do not yet exist in the index.

**Evidence (observed):**

- public core case `fix_custom_config.executable` in the receipt: A fresh committed synthetic
  consumer with one `100755` `script.sh` makes the actual `fix` command exit 3 with the
  empty-index/missing-`--add` Git error. No simulator is started and no patch is created. The
  original worktree remains clean and source unchanged. The non-executable default control succeeds.

**Why it matters:** Shell scripts, executable entrypoints, and other ordinary tracked executables
make the preview feature unusable for otherwise supported repositories. This is not limited to a
malformed or hostile tree.

**Recommended:** Populate the trusted index before restoring executable modes (or explicitly use
safe add semantics), retain the exact immutable base-tree equality check, and place a path separator
before user-controlled paths in the index command.

**Suggested validation:**

- CLI preview on a clean consumer containing ordinary and executable files should reach the bounded
  runtime, preserve exact base mode/content identities, generate the intended patch, and leave the
  original untouched. Include a nested executable and a legal leading-dash filename; retain
  unsupported symlink/gitlink rejection. Platform claims require platform-specific evidence.

**Dependencies and risks:** No identified predecessor for a bounded repair. Mode preservation is
important on platforms whose filesystem does not express Git executable bits. Linux behavior was
actually observed; no macOS/Windows result is claimed.

**Issue home:** `FIX_PREVIEW` (reviewed proposed home; see the coverage map below).

### AUDIT-003 — Focused canonical native reports can survive unrelated or failed later runs

**Classification:** confirmed defect. **Severity:** medium. **Confidence:** high. **Status:**
confirmed.

**Area:** run evidence lifecycle and provenance. **Effort:** small. **Impact:** medium. **History:**
new audit observation.

**Observed:** The fixed `RUN_OWNED_ARTIFACTS` list removes raw MegaLinter reports, aggregate
JSON/SARIF, debt, and patches, but omits `repository-continuity.json`,
`repository-intelligence.json`, `repository-presentation.json`, `repository-release.json`, and
`repository-gitignore.json` (`src/plan.rs:29–44`). `write_run_outputs` only rewrites optional
focused reports when that capability is selected (`src/main.rs:1296–1341`); the focused gitignore
command also writes a report outside that optional list.

**Evidence (observed):**

- public core case `focused_report_lifecycle` in the receipt: The actual native CLI first runs an
  advisory intelligence policy successfully (exit 0) and writes an authentic focused intelligence
  report. A second `validate` without that selection exits 0 and writes a new aggregate that does
  not reference intelligence, while the old focused report remains byte-identical. A third attempt
  with a nonexistent policy fails, removes the aggregate during preparation, and still leaves the
  old focused report byte-identical. No real downstream publication or false consumer verdict is
  inferred from this local evidence.

**Why it matters:** A fixed report directory may contain canonical files from multiple runs or
retain a previous focused verdict after a failed new invocation. Consumers or artifact uploads can
mistake file presence for evidence of the latest invocation. This contradicts the documented
stale-contract cleanup boundary even though individual files carry some source metadata.

**Recommended:** Define the complete run-owned canonical artifact inventory and clear or explicitly
invalidate each focused output at the appropriate lifecycle boundary. Alternatively use isolated run
directories with an explicit manifest of the current run. Preserve unrelated private diagnostic
retention deliberately.

**Suggested validation:**

- Generate real focused reports, then run without those capabilities and run with invalid inputs.
  Prove no stale focused artifact can represent the new run; verify current selected reports and
  aggregate references remain consistent and unrelated diagnostic files are retained according to
  policy. Cover command transitions including native-only, gitignore, and container-backed runs with
  a bounded simulator before qualifying real images.

**Dependencies and risks:** No identified predecessor for a bounded repair. Coordinate all focused
report owners and artifact consumers; avoid deleting arbitrary files under the reserved report
directory or broadening its write authority.

**Issue home:** `NATIVE_REPORT_LIFECYCLE` (reviewed proposed home; see the coverage map below).

### AUDIT-004 — Presentation validation can accept missing checksum coverage and a missing banner descriptor

**Classification:** confirmed defect. **Severity:** medium. **Confidence:** high. **Status:**
confirmed.

**Area:** Repository presentation / Identity input integrity. **Effort:** medium. **Impact:**
medium. **History:** new audit observation.

**Observed:** The manifest verifier only iterates entries that remain in files, and the banner/badge
validation block runs only when a banner descriptor is present. It accepts an empty files object or
a nonempty map that omits the referenced banner. It also accepts an Identity package with its banner
descriptor deleted.

**Evidence (observed):**

- `src/rules/repository_presentation.rs`; lines 1029–1069: Missing file map is rejected, but an
  empty map is accepted and referenced asset coverage is not required.
- `src/rules/repository_presentation.rs`; lines 1071–1105: Conditional destructuring skips all
  banner/badge checks when package.banner is missing.
- Public intelligence probe results in the receipt: presentation.control returns exit0/status
  valid/21 checked files. changed_banner_full_manifest returns exit1/status invalid with
  EGO-PRESENT-MANIFEST-001. Removing only the damaged banner's file entry yields exit0/status
  valid/20 checks; emptying the map yields exit0/status valid/0 checks. Deleting package.banner
  yields exit0/status valid/21 checks.

**Why it matters:** A publishable valid result can claim the Identity presentation is bound to
checksums while the actual README banner has no verified checksum or the required descriptor is
absent.

**Recommended:** Validate required Identity package/manifest structure and cross-field coverage
before declaring valid. Require a nonempty file map and checksum entries for every referenced
banner/badge asset and descriptor that the pinned owner contract requires. Missing required
descriptors must emit diagnostics rather than skip checks.

**Suggested validation:**

- Retain the passing complete-map control and damaged-file rejection.
- Reject empty maps, nonempty maps omitting referenced assets, absent banner descriptors, and
  missing required descriptor-to-manifest bindings.
- Keep advisory mode nonblocking while focused semantic status remains invalid; preserve offline
  behavior and authored files.

**Dependencies and risks:** No external predecessor identified for the bounded repair against
current pinned contracts. Check exact Identity 3c2fd3 owner requirements and avoid inventing a new
package contract or claiming pixel/accessibility certification.

**Issue home:** `PRESENTATION_INTEGRITY` (reviewed proposed home; see the coverage map below).

### AUDIT-005 — ADR and roadmap validators count fenced examples as live Markdown anatomy

**Classification:** confirmed defect. **Severity:** medium. **Confidence:** high. **Status:**
confirmed.

**Area:** Repository Intelligence / Markdown source grammar. **Effort:** medium. **Impact:** medium.
**History:** new audit observation.

**Observed:** ADR heading/required-section matching and roadmap comment/heading/checklist matching
operate on raw strings, including literal fenced-code examples. Moving all ADR body anatomy into a
markdown code fence, or putting the entire roadmap in a code fence, still produces valid source
validation.

**Evidence (observed):**

- `src/rules/repository_intelligence.rs`; lines 1567–1604: ADR anatomy uses starts_with and
  body.find without Markdown literal-region awareness.
- `src/rules/repository_intelligence.rs`; lines 2224–2245: Roadmap HTML comment extractor accepts
  any matching byte substring, including a literal example.
- `src/rules/repository_intelligence.rs`; lines 3632–3990: Roadmap evaluation scans these extracted
  comments and segment lines.
- Public intelligence probe results in the receipt: Control passes; removing Context returns
  invalid/1 diagnostic and empty roadmap invalid/2 diagnostics. Fencing the ADR body or entire
  roadmap returns exit0/status valid/0 diagnostics with 3 ADRs and 2 roadmap steps inspected.

**Why it matters:** A code sample can substitute for the actual required document structure or
introduce phantom graph records, so semantic evidence can disagree with the rendered canonical
document.

**Recommended:** Use a bounded Markdown-aware representation or at least a shared literal-region
scanner to exclude fenced/indented code and escaped examples from ADR anatomy, indexes, roadmap
metadata, headings, and checklist counts. Keep source positions deterministic.

**Suggested validation:**

- Prove real required headings/metadata pass while fenced, indented, escaped, and inline literal
  examples do not satisfy required structure.
- Include examples alongside real roadmap records so examples do not create duplicates or phantom
  graph edges.
- Check heading level and exact required-section identity, source line locations, state/checklist
  matching, and existing historical fixtures.

**Dependencies and risks:** No external predecessor identified for the bounded current-contract
repair. Preserve existing accepted ADR metadata/lifecycle and proposed roadmap authority; no graph
policy promotion is implied.

**Issue home:** `MARKDOWN_SOURCE_GRAMMAR` (reviewed proposed home; see the coverage map below).

### AUDIT-006 — ADR link validation accepts empty GitHub identities and a hostless HTTP destination

**Classification:** confirmed defect. **Severity:** medium. **Confidence:** high. **Status:**
confirmed.

**Area:** Repository Intelligence / evidence reference grammar. **Effort:** small. **Impact:**
medium. **History:** new audit observation.

**Observed:** The GitHub URL helper checks only that splitting owner/repo produces two components,
without checking that either is nonempty. The general HTTP helper checks a prefix and a dot anywhere
after ://, allowing a path dot to stand in for an authority.

**Evidence (observed):**

- `src/rules/repository_intelligence.rs`; lines 2467–2500: HTTP and GitHub helpers use
  substring/count heuristics rather than host and repository components.
- Public intelligence probe results in the receipt: Replacing a fixture issue URL with
  `https://github.com///issues/24` and adding external evidence URL `https:///.` still returns
  exit0/status valid/0 diagnostics under all eleven enabled blocking-profile rules.

**Why it matters:** Malformed reference strings can be recorded as structurally valid issue/evidence
linkage. Offline validation cannot prove reachability or human approval, but it should not accept an
absent authority or empty repository identity.

**Recommended:** Parse the URI authority/path and require supported HTTP host and
owner/repository/positive-number or full-commit components. Reuse strict repository and identifier
validation; retain sanitized diagnostics and explicit separation from online reachability.

**Suggested validation:**

- Reject absent hosts, empty owner/repository segments, malformed commit references,
  credentials/control/whitespace ambiguities as appropriate to the pinned contract.
- Retain valid supported HTTP(S), canonical GitHub issue/PR/commit, and historical evidence
  fixtures.
- Do not fetch the destinations or treat structural success as publication/human approval.

**Dependencies and risks:** No external predecessor identified. This is a correctness finding about
structure, not a demonstrated network exploit or authenticated-approval bypass.

**Issue home:** `ADR_REFERENCE_GRAMMAR` (reviewed proposed home; see the coverage map below).

### AUDIT-007 — Release Task handoff validation treats echoed or commented commands as exact workflow dispatch

**Classification:** confirmed defect. **Severity:** medium. **Confidence:** high. **Status:**
confirmed.

**Area:** Repository release / mechanical handoff evidence. **Effort:** medium. **Impact:** medium.
**History:** new audit observation.

**Observed:** task_dispatches_workflow accepts any Task command string containing both gh workflow
run and the declared workflow path. It cannot distinguish an echo from dispatch or bind the workflow
path as the actual gh argument.

**Evidence (observed):**

- `src/rules/repository_release/checks.rs`; lines 973–993: The boolean helper directly produces a
  passed Task handoff assertion.
- `src/rules/repository_release/checks.rs`; lines 1407–1424: The helper searches two substrings
  without command/argument binding.
- Public intelligence probe results in the receipt: Both echo gh workflow run
  .github/workflows/release.yml and gh workflow run .github/workflows/other.yml #
  .github/workflows/release.yml produce CLI0, focused state compliant, validation_complete true, 7/7
  checks passed, including the exact-dispatch assertion. The normal dispatch control also passes.

**Why it matters:** A static conformance result asserts an exact publication handoff even when the
declared command only prints text or dispatches a different workflow.

**Recommended:** Define a small supported static dispatch form or structured handoff declaration,
bind the actual command and workflow argument, and report unavailable when arbitrary shell cannot be
proved. Do not execute Task commands to validate them or build an unrestricted shell interpreter.

**Suggested validation:**

- Reject or mark unavailable echo-only, comments-only, quoted documentation, different workflow
  arguments, and ambiguous compound shell forms.
- Accept the documented direct dispatch and any explicitly supported structured equivalent; preserve
  exact source location and standard focused/run/SARIF agreement.
- Extend #68's policy-boundary matrix with these cases, then include the repaired behavior in #69's
  dogfood.

**Dependencies and risks:** Current-v1 repair can start against existing accepted pins. Coordinate
`#29/#68/#69`; do not wait for successor #76 or treat local conformance as remote publication
evidence. Probe only validates source and never executes the synthetic Task commands.

**Issue home:** `RELEASE_TASK_BINDING` (reviewed proposed home; see the coverage map below).

### AUDIT-008 — A contradictory not-applicable represented commit can still produce a valid Intelligence report

**Classification:** confirmed defect. **Severity:** medium. **Confidence:** high. **Status:**
confirmed.

**Area:** Repository Intelligence / adoption and semantic status. **Effort:** small. **Impact:**
medium. **History:** new audit observation.

**Observed:** The evaluator emits an adoption diagnostic when represented_commit is not-applicable
while a source surface is present, but status aggregation ignores every adoption diagnostic for
invalidity and only treats unknown/truncated evidence as incomplete.

**Evidence (observed):**

- `src/rules/repository_intelligence.rs`; lines 819–852: Incomplete aggregation omits the
  present/not-applicable contradiction and invalid excludes ADOPTION_RULE.
- `src/rules/repository_intelligence.rs`; lines 966–982: validate_adoption explicitly diagnoses the
  incompatible selection.
- Public intelligence probe results in the receipt: With all eleven blocking-profile rules enabled
  and ADR, roadmap and commit-history present, --represented-commit not-applicable returns
  exit0/status valid, commits_inspected0, 3 ADRs, 2 roadmap steps, and an EGO-INTEL-ADOPTION-001
  warning saying represented commit cannot be not-applicable while a source surface is present.

**Why it matters:** Machine consumers can see valid semantic status for a declared present
source/history surface without its represented commit. The report itself identifies the
contradiction but does not propagate it to the status.

**Recommended:** Keep non-adoption, missing evidence, truncation, and contradictory adoption
distinct. Make a present/not-applicable represented-source conflict invalid or explicitly incomplete
under the supported contract; advisory enforcement must not upgrade that semantic state.

**Suggested validation:**

- Cover present surfaces with exact commit, unknown commit, not-applicable commit, all surfaces
  not-applicable, and truncated history.
- Assert focused semantic status independently from process exit, rule severity and
  advisory/blocking enforcement.
- Preserve the intentional scope of selected/disabled rules rather than silently enabling unrelated
  checks.

**Dependencies and risks:** No external predecessor identified for a bounded consistency repair. The
caller still owns authentication of represented identity and source checkout; this finding does not
claim that offline validation authenticates GitHub.

**Issue home:** `INTELLIGENCE_ADOPTION_STATUS` (reviewed proposed home; see the coverage map below).

### AUDIT-009 — Make native continuity validation agree with its exact pinned Aether schema

**Classification:** confirmed defect. **Severity:** medium. **Confidence:** high. **Status:**
confirmed.

**Area:** native continuity validation. **Effort:** medium. **Impact:** medium. **History:** new
audit observation.

**Observed:** Five synthetic checkpoint variants disagree between native continuity semantic status
and Draft202012Validator using the exact bundled Aether schema with registered format checks.
Removing six required nullable keys, duplicating next.references, duplicating parallel_changes, or
using an unpadded RFC3339 clock all remain native valid despite schema errors. A schema-valid
lowercase t/z RFC3339 timestamp instead becomes native invalid. The control is accepted by both.

**Evidence (observed):**

- `src/rules/repository_continuity.rs`; lines 1110-1281: Option metadata fields decode missing keys
  as None, independently of schema required arrays.
- `src/rules/repository_continuity.rs`; lines 1892-2216: The handwritten metadata validator checks
  selected fields but omits uniqueness on next references and parallel changes.
- `src/rules/repository_continuity.rs`; lines 3363-3409: The handwritten datetime parser accepts
  variable-width clock fields and requires uppercase T/Z.
- `vendor/aether/aether.repository-continuity.v1.schema.json`; SHA-256
  `a725e004fe0db2bf96a968c6e3f300d37ecf8b00b9a0923be5c5f4c529655916`: Exact normative structure used
  for the comparison.
- Public validators probe results in the receipt; case group `/continuity/cases`: Native focused
  semantic status, process result and actual pinned schema errors for control and five variants.

**Why it matters:** A consumer can receive valid structural evidence for a checkpoint that fails the
selected portable contract, while another contract-valid checkpoint receives an invalid diagnosis.
This is independent of observe-mode exit status, which intentionally remains nonblocking.

**Recommended:** Treat the selected local schema as the authoritative structural contract, or prove
equivalent typed validation for required nullable fields, uniqueness and RFC3339 semantics. Preserve
offline source integrity and layered native freshness/privacy checks.

**Suggested validation:**

- A paired native/schema matrix covers every required nullable field independently, duplicate
  references and parallel changes, unsupported fields and formats.
- Invalid unpadded timestamps produce structural findings; schema-valid lowercase RFC3339 delimiters
  preserve contract acceptance.
- Observe changes effective severity without changing invalid semantic status; no upstream lifecycle
  promotion or source-pin relaxation is needed.

**Dependencies and risks:** No external predecessor identified for the bounded correction. Existing
issue #55 owns broader continuity capability and released-input rollout acceptance; this parity
repair must not claim ratchet/enforce promotion.

**Issue home:** `CONTINUITY_SCHEMA_PARITY` (reviewed proposed home; see the coverage map below).

### AUDIT-010 — Detect case-folded file/directory conflicts in native portability inventory

**Classification:** confirmed defect. **Severity:** medium. **Confidence:** high. **Status:**
confirmed.

**Area:** native portability and inventory topology. **Effort:** small. **Impact:** medium.
**History:** new audit observation.

**Observed:** The native case checker indexes only complete leaf paths. A synthetic checkout
containing the file Readme and directory README/page.md produces no EGO-PORT-CASE-001 finding and
exits 0. The ordinary Readme.md/README.md control correctly produces the rule and exits 1.
Differently cased ancestor directories with disjoint leaf names are also not represented by the
current check.

**Evidence (observed):**

- `src/rules/portability.rs`; lines 208-234: case_collisions only folds each complete entry path.
- `src/rules/inventory.rs`; lines 137-155: Ordinary directory entries are not kept in the leaf
  inventory.
- `.config/rules/portability.toml`; lines 5-14: The selected rule promises ASCII case-insensitive
  path collision detection.
- Public validators probe results in the receipt; case group `/portability`: Positive full-path
  collision control and missed prefix topology cases.

**Why it matters:** On a case-insensitive filesystem, the same folded node cannot be both a file and
a directory. Such a tree can pass native portability checks before a consumer encounters an
unrepresentable checkout. No Windows or macOS checkout was executed during this audit; the native
detection gap is directly observed, and the portability consequence follows from case-insensitive
path identity.

**Recommended:** Build a folded path-prefix topology from the inventory, including inferred ancestor
directories, and report conflicting entry kinds and reviewed casing collisions deterministically.
Preserve the intentional ASCII-only boundary.

**Suggested validation:**

- Keep the existing identical-leaf collision test.
- Add case-folded file/directory prefix conflicts in both lexical orders and nested positions.
- Cover disjoint leaf files beneath differently cased ancestors and ordinary case-consistent shared
  directories without false positives.

**Dependencies and risks:** No predecessor identified; pure native metadata semantics. Unicode and
locale-specific filesystem equivalence remain outside the selected ASCII rule.

**Issue home:** `PORTABILITY_PATH_PREFIXES` (reviewed proposed home; see the coverage map below).

### AUDIT-012 — JavaScript adapters can turn failed or unsupported evidence into a clean pass

**Classification:** confirmed defect. **Severity:** medium. **Confidence:** high. **Status:**
confirmed.

**Area:** JavaScript adapter evidence. **Effort:** medium. **Impact:** medium. **History:** new
audit observation.

**Observed:** The unchanged package-quality CLI returns 0 with OXLINT=passed and BIOME=passed when
the explicitly stubbed Oxlint subprocess exits 1 but emits an empty diagnostics array. It also
returns 0 and zero findings when both stubs emit the unsupported object envelope {}. runAdapter
discards allowed process statuses and the normalizers substitute missing collections with empty
arrays.

**Evidence (observed):**

- `scripts/javascript-package-quality.mjs:192-207,315-317,343-345,697-705,734`; public adapters case
  `javascript_package_quality.exit_one_without_diagnostics` in the receipt
- public adapters case `javascript_package_quality.malformed_envelopes` in the receipt

**Why it matters:** Malformed, incomplete or contradictory adapter evidence is labeled passed rather
than unavailable/invalid/partial, weakening the stable quality-gate contract.

**Recommended:** Preserve adapter status alongside payload, validate supported envelopes and
reconcile nonzero exits with normalized findings. Emit an explicit non-clean result when evidence
cannot explain a failure.

**Suggested validation:**

- Keep the clean pinned-adapter control.
- Use trusted executable stubs for exit 1 with no diagnostics, empty/wrong-type envelopes and
  contradictory counts; require non-clean structured outcomes.
- Retain real pinned Oxlint/Biome consumer fixtures to prove the supported adapter shapes.
- Validate SARIF version and kind/level consistency. Pass records with level none or absent remain
  non-blocking; pass plus error is a malformed control, not a valid-pass regression.

**Dependencies and risks:** No external predecessor identified for this repair. Stubbed subprocess
evidence proves orchestrator behavior only; it does not assert current pinned analyzers naturally
emit these malformed shapes.

**Issue home:** `JS_ADAPTER_EVIDENCE` (reviewed proposed home; see the coverage map below).

### AUDIT-014 — JSONSkooma selector crashes on malformed mode types and accepts a boolean version

**Classification:** confirmed defect. **Severity:** medium. **Confidence:** high. **Status:**
confirmed.

**Area:** Optional Ruby schema adapter configuration. **Effort:** small. **Impact:** medium.
**History:** new audit observation.

**Observed:** The actual Python launcher with mode=[] or mode={} raises an uncaught TypeError, exits
1 and emits no structured report. Its documented invalid-configuration path is exit 2 with JSON
evidence. schema_version=true is accepted as version 1 and a disabled configuration exits 0.

**Evidence (observed):**

- `scripts/json_skooma.py:79-98,223-241`; public adapters case `json_skooma.list_mode` in the
  receipt
- public adapters case `json_skooma.object_mode` in the receipt
- public adapters case `json_skooma.boolean_version` in the receipt

**Why it matters:** Configuration failure is confused with validation findings and can leave
consumers without an evidence artifact; boolean/integer coercion weakens the explicit version
contract.

**Recommended:** Validate types before set membership and version equality; route all supported
malformed-configuration cases into the structured invalid_configuration outcome without launching
Ruby.

**Suggested validation:**

- Wrong JSON types for mode/schema_version return exit 2 with structured invalid_configuration
  reports.
- Boolean versions are rejected.
- Valid disabled/auto/enabled activation and skip-before-Ruby behavior remain intact.

**Dependencies and risks:** No Ruby installation is required to reproduce the observed selector
failures. No external predecessor identified.

**Issue home:** `JSON_SKOOMA_CONFIG` (reviewed proposed home; see the coverage map below).

### AUDIT-015 — JSONSkooma relative report location changes with the activation branch

**Classification:** confirmed defect. **Severity:** medium. **Confidence:** high. **Status:**
confirmed.

**Area:** Optional Ruby schema adapter evidence routing. **Effort:** small. **Impact:** medium.
**History:** new audit observation.

**Observed:** With --workspace selecting a different directory and the default relative report path,
a skip writes the report beneath the invocation directory. The applicable branch invokes its child
with cwd=workspace and passes the same relative report path, so a trusted Ruby stub writes beneath
the selected workspace. The observed destination therefore depends on activation rather than one
report-path rule.

**Evidence (observed):**

- `scripts/json_skooma.py:187-196,240-247,250-275`; public adapters case
  `json_skooma.default_report_routing` in the receipt
- public adapters case `json_skooma.applicable_default_report_routing_stubbed_ruby` in the receipt

**Why it matters:** Callers using the explicit workspace interface cannot reliably collect the same
report location for skipped, unavailable and applicable runs.

**Recommended:** Define one relative-report anchor and resolve it once before either local writing
or child invocation, retaining stdout and explicitly supported absolute-output behavior.

**Suggested validation:**

- Invoke from a directory different from --workspace and check skip, invalid, unavailable, timed-out
  and applicable branches produce the same selected destination.
- Keep --report '-' behavior and explicit absolute destinations covered.
- Use a trusted child stub for routing tests and retain real Ruby fixtures for actual schema
  semantics.

**Dependencies and risks:** The applicable routing control uses a stub, not actual Ruby validation.
No external predecessor identified.

**Issue home:** `JSON_SKOOMA_REPORT_ROUTING` (reviewed proposed home; see the coverage map below).

### AUDIT-016 — Release input validation accepts non-object image and product records

**Classification:** confirmed defect. **Severity:** medium. **Confidence:** high. **Status:**
confirmed.

**Area:** release input contract validation. **Effort:** small. **Impact:** medium. **History:** new
audit observation.

**Observed:** The release-input validator treats a null image or product record as zero findings
rather than invalid input. A copied actual CLI accepts a null full-engine product record with the
correct environment; preserving all expected names while making every image/product record null also
accepts an environment without any reviewed image values.

**Evidence (observed):**

- `scripts/validate_release_inputs.py`; lines 67–72,100–109,153–168,174–181
- `tests/test_release_inputs.py`; lines 22–81
- Public distribution probe results in the receipt; cases
  `valid_contract_missing_environment_control`, `valid_contract_pinned_environment_control`,
  `null_product_record`, `null_all_image_and_product_records`

**Why it matters:** A named, present, but wrong-type source record can receive the same success
message as a complete reviewed base-image/product contract. This weakens contract checks and can
obscure accidental source corruption before the build.

**Recommended:** Require every expected record to be an object, reject malformed nested element
types without uncaught exceptions, and keep environment checking contingent on a structurally valid
contract.

**Suggested validation:**

- Replay positive reviewed data, every missing/non-object image and product record, wrong nested
  list/map element types, and missing/drifted environment values; assert a controlled nonzero
  diagnostic for invalid data and retain the existing valid-platform and environment controls.

**Dependencies and risks:** Independently startable from checked-in source; no release, registry,
ARM64, or sibling capability predecessor is needed. This does not establish that a release can
bypass later builds, artifact checks, protected environment, or promotion.

**Issue home:** `RELEASE_INPUT_CONTRACT` (reviewed proposed home; see the coverage map below).

### AUDIT-018 — ADR migration documentation retains a superseded schema digest and unchanged-version instructions

**Classification:** documentation gap. **Severity:** medium. **Confidence:** high. **Status:**
confirmed.

**Area:** Documentation / immutable ADR contract provenance. **Effort:** small. **Impact:** medium.
**History:** new audit observation.

**Observed:** The current catalog selects accepted ADR1.2.0 at a05b14f8 and pins the updated schema
digest725a7cf5..., while the migration guide still lists491eae26... and says schemas and contract
versions remain unchanged.

**Evidence (observed):**

- `docs/repository-intelligence.md`; lines 27–43: The guide claims byte identity and records the old
  digest while its preceding table now shows1.2.0.
- `.config/rules/repository-intelligence.v1.toml`: Catalog alpha.5 selects ADR1.2.0 at a05b14f8 with
  digest725a7cf58cd0b0454d4d481e8ef72815f29ecca80a5b44da5ce48f7ec6e6b8c4.
- `vendor/hygiene/adr-policy-owner/schemas/architecture-decision.v1.schema.json`: Actual SHA256
  independently computed as725a7cf58cd0b0454d4d481e8ef72815f29ecca80a5b44da5ce48f7ec6e6b8c4,
  matching catalog rather than guide.

**Why it matters:** A consumer following the migration text can verify the wrong schema bytes or
retain an incompatible contract version despite choosing the current source revision.

**Recommended:** Rewrite the migration guide around exact current1.2 and supported historical1.1
tuples, correct the digest and four-digit behavior, and time-qualify historical compatibility
narrative.

**Suggested validation:**

- Compare every documented supported tuple and digest with the existing immutable catalog/vendor
  fixtures.
- Validate examples against the selected native binary and preserve supported older pins and
  original fixture bytes.
- Use existing meaningful source checks rather than adding a test that merely mirrors copied prose.

**Dependencies and risks:** No new policy decision is needed; four-digit policy1.2 is already
accepted. Root is consolidating current documentation/navigation drift into one issue home, separate
from future claim-drift engine #46.

**Issue home:** `CURRENT_DOCS` (reviewed proposed home; see the coverage map below).

## Low severity

### AUDIT-011 — Recognize static multi-OS matrix.include rows in the workflow shell check

**Classification:** confirmed defect. **Severity:** low. **Confidence:** high. **Status:**
confirmed.

**Area:** native workflow portability diagnostics. **Effort:** small. **Impact:** low. **History:**
new audit observation.

**Observed:** An equivalent two-OS workflow loses EGO-PORT-WORKFLOW-001 when matrix.os is expressed
through static matrix.include rows rather than a direct array. Both fixtures select runs-on from
matrix.os and omit shell. The direct-array control emits the advisory; the include form does not.

**Evidence (observed):**

- `src/rules/portability.rs`; lines 659-690: OS discovery only inspects direct sequence-of-string
  matrix axes and matches their key against runs-on; include rows are ignored.
- `docs/portability-rules.md`; lines 14-24: The native pack advertises multi-OS workflow shell
  diagnostics.
- Public validators probe results in the receipt; case group `/portability/include_multi_os_matrix`:
  Actual native command has no workflow-shell finding; the direct-array control has one.

**Why it matters:** A common static GitHub Actions matrix representation silently bypasses the
selected advisory about differing default shell semantics. This remains a diagnostic-coverage issue,
not proof that the fixture's echo command fails on either operating system.

**Recommended:** Resolve statically declared include rows and scalar selector paths before deciding
whether multiple OS families are possible. Keep dynamic or unsupported expressions explicitly
bounded rather than guessing their values.

**Suggested validation:**

- Equivalent direct arrays and include rows produce the same advisory for an omitted shell.
- Step, job and workflow default shells prevent the advisory in each supported static
  representation.
- Single-OS, matrix.exclude refinements and unresolved dynamic selectors retain documented limits.

**Dependencies and risks:** No predecessor identified. The rule remains advisory and should not
attempt general GitHub expression execution.

**Issue home:** `PORTABILITY_INCLUDE_MATRIX` (reviewed proposed home; see the coverage map below).

### AUDIT-013 — JavaScript consumer manifest validation accepts fields forbidden by its published schema

**Classification:** confirmed defect. **Severity:** low. **Confidence:** high. **Status:**
confirmed.

**Area:** JavaScript configuration contract. **Effort:** small. **Impact:** low. **History:** new
audit observation.

**Observed:** The checked-in manifest schema has additionalProperties=false. A manifest with the
unknown key profil_typo is schema-invalid but the unchanged CLI accepts it and returns 0 under clean
stubbed adapters. validateManifest checks selected values without enforcing the closed object
grammar.

**Evidence (observed):**

- `schemas/javascript-package-quality-manifest.schema.json:5; scripts/javascript-package-quality.mjs:99-136`;
  public adapters case `javascript_package_quality.unknown_manifest_property` in the receipt

**Why it matters:** Consumer typos and stale fields silently diverge from the public machine
contract and can conceal configuration intent.

**Recommended:** Validate manifests against the closed published contract or enforce the same
grammar and unique-item constraints mechanically, preserving the one canonical schema.

**Suggested validation:**

- Unknown properties and duplicate ignore entries fail as configuration errors before adapters
  start.
- Valid base and react-library manifests remain accepted.
- Schema and runtime rejection agree for representative malformed values.

**Dependencies and risks:** No external predecessor identified. The observed unknown property is
harmless synthetic data; no security exploit is claimed.

**Issue home:** `JS_MANIFEST_CONTRACT` (reviewed proposed home; see the coverage map below).

### AUDIT-017 — Missing requested native policy files use the internal-error exit category

**Classification:** confirmed defect. **Severity:** low. **Confidence:** high. **Status:**
confirmed.

**Area:** public CLI exit semantics. **Effort:** small. **Impact:** low. **History:** new audit
observation.

**Observed:** `src/error.rs` defines missing requested paths as configuration exit 2, and
`MissingPath` maps accordingly. The common native input reader maps missing ancestor/leaf metadata
directly to `Filesystem` (`src/main.rs:1083–1087`), which maps to internal exit 4. Issue-title
validation separately handles NotFound as `MissingPath`, so public commands disagree.

**Evidence (observed):**

- public core case `focused_report_lifecycle` in the receipt: The third lifecycle probe runs
  `validate --repository-intelligence missing.toml --represented-commit unknown` in the same valid
  clean synthetic Git consumer and exits 4. The requested file does not exist; no decoder, runtime,
  or internal computation has failed.

**Why it matters:** Scripts distinguishing user input errors from internal failures get the wrong
stable exit category. This does not alter the detected error or mean that a failed run passed.

**Recommended:** Preserve NotFound as the documented missing-input/configuration category in the
shared input reader, while retaining genuine permission/I/O/internal failures as their own
categories.

**Suggested validation:**

- Public CLI missing native policy/suppression inputs return configuration exit 2; malformed input
  and genuine I/O failures retain distinct documented categories. Verify no stale current-run output
  survives these early failures (AUDIT-003).

**Dependencies and risks:** No identified predecessor. Avoid recategorizing every filesystem error
as configuration or weakening regular-file/path/budget checks.

**Issue home:** `CLI_INPUT_EXIT` (reviewed proposed home; see the coverage map below).

### AUDIT-019 — Container guidance contradicts the reviewed full-engine platform boundary

**Classification:** documentation gap. **Severity:** low. **Confidence:** high. **Status:**
confirmed.

**Area:** distribution documentation. **Effort:** small. **Impact:** low. **History:** new audit
observation.

**Observed:** The current container instructions request linux/amd64,linux/arm64 for egolint-full,
call the reviewed MegaLinter base a manifest-list, and require both-architecture full-engine smokes.
The release design also says that the full image is exercised on both architectures. The source
contract and actual release matrix instead declare the full image AMD64-only; the same release
document later states that limit.

**Evidence (observed):**

- `docs/containers.md`; lines 33–50
- `docs/releasing.md`; lines 60–65,96–103
- `.config/release/base-images.v1.json`; lines 4–12
- `.github/workflows/release.yml`; symbol `build-images.strategy.matrix.include`

**Why it matters:** A maintainer following the copyable command is directed to an unsupported
full-engine target and receives contradictory publication acceptance instructions.

**Recommended:** Align current commands and prose with the checked-in CLI dual-platform/full-engine
AMD64-only contract. Retain future ARM64 delivery and proof under #45, separate from correcting
present instructions.

**Suggested validation:**

- Review all user-facing platform and release examples against the declared product matrix; check
  the corrected example targets. Actual ARM64 full-engine build/consumer proof is required only if
  the advertised platform contract is intentionally expanded.

**Dependencies and risks:** Current documentation repair is independently startable; #45's future
ARM64 artifact delivery remains gated. Do not use this finding to claim that an ARM64 product was
built, released, or validated.

**Issue home:** `CURRENT_DOCS` (reviewed proposed home; see the coverage map below).

### AUDIT-020 — Current source navigation and architecture inventory lag merged implementation

**Classification:** documentation gap. **Severity:** low. **Confidence:** high. **Status:**
confirmed.

**Area:** current repository navigation and architecture documentation. **Effort:** small.
**Impact:** low. **History:** new audit observation.

**Observed:** The root checkpoint still tells the next worker to merge the four-digit validator
candidate even though PR #83 is merged at the audited main revision. ROADMAP.md still presents
closed issue #73 as current under-review work and reproduces seven state/parser mismatches already
noted in roadmap #42. SYSTEM.md marks the profile catalog, MegaLinter adapter, report normalizer and
CLI as Target with repeated generic responsibilities despite their implemented and tested source.
Its provisional target intent remains valid context, but it does not provide a usable current
implementation inventory.

**Evidence (observed):**

- `CONTINUITY.md`; section State snapshot; Next dependency-ready work
- `ROADMAP.md`; section Current gate; EGL-Q07; EGL-Q11
- `SYSTEM.md`; section System inventory
- `docs/architecture.md`; section Native binary and lightweight image; Full image
- [https://github.com/egohygiene/egolint/pull/83](https://github.com/egohygiene/egolint/pull/83):
  merged as 834a230e6c0f4ee8b080a10d6c2d469be5eb6e1b
- [https://github.com/egohygiene/egolint/issues/73](https://github.com/egohygiene/egolint/issues/73):
  closed when independently inspected

**Why it matters:** A new human or agent can repeat delivered work, misread the current release gate
or underestimate implemented capabilities. The declared 18-document graph is structurally sound;
that does not establish the truth or freshness of its prose.

**Recommended:** Reconcile source navigation, current system responsibilities and implementation
evidence while retaining explicit target/proposed status. Hand off roadmap #42's existing
source-cleanup note into the focused current-documentation issue. The authorized audit publication
refreshes its own operational CONTINUITY checkpoint; that bounded refresh does not complete the
wider source-doc repair or accept architecture policy.

**Suggested validation:**

- Read all changed navigation, system, contract-version and platform statements against exact source
  and verified issue/PR states.
- Use the existing native Intelligence parser to reconcile current roadmap warnings and preserve
  accepted/proposed distinctions.
- Keep the declared graph and actual relative destinations valid; no repository-root layout
  reorganization or new mirror test is required by this finding.

**Dependencies and risks:** No predecessor identified for correcting current evidence and prose.
Future documentation-location policy, historical ADR migration, accepted policy changes and complete
release acceptance remain with their existing owners.

**Issue home:** `CURRENT_DOCS` (reviewed proposed home; see the coverage map below).

### AUDIT-021 — Roadmap #42 does not represent the complete current open backlog

**Classification:** documentation gap. **Severity:** low. **Confidence:** high. **Status:**
confirmed.

**Area:** repository execution ledger and issue coverage. **Effort:** small. **Impact:** low.
**History:** new audit observation.

**Observed:** At the frozen starting observation there are 27 open local work issues plus roadmap
`#42`, and no open PR. The parent directly links 10 of those work issues and omits 17. All 28
original issues have no native parent or children; the older parent also includes closed local
foundations and external organization coordination. Those historical references are useful but are
not complete current local work coverage.

**Evidence (observed):**

- [https://github.com/egohygiene/egolint/issues/42](https://github.com/egohygiene/egolint/issues/42):
  complete original body and metadata saved before mutation
- Initial public issue-ledger observation recorded in the receipt: all 28 original bodies read;
  original metadata and body digests retained for preservation checks
- Initial public issue-ledger observation recorded in the receipt: direct current links
  5,29,35,36,37,38,39,40,41,77; all 28 original native child sets empty

**Why it matters:** Workers resuming from the existing parent can miss substantial public work and
have no native complete-backlog ledger. Closed foundations and external programs must not inflate
local outstanding counts.

**Recommended:** Refresh existing #42 rather than create another parent. Preserve its exact original
body as clearly dated historical program context, retain all original titles, labels, text, comments
and semantic Program/Parent links, and attach every original open work issue plus focused public
audit follow-ups exactly once in the agreed order. Mark full-outcome gates without treating all
preparation as blocked.

**Suggested validation:**

- Verify the complete original set and focused additions exactly once in the parent body and native
  hierarchy, with no closed/cross-repository item counted as local open work.
- Verify original body prefixes remain intact on work issues; preserve the entire original #42 text
  under its historical heading.
- Verify the agreed issue order, markers, counts, next action and native completed count against
  actual provider state after publication.

**Dependencies and risks:** No external predecessor identified for this authorized planning
workflow. The audit publication addresses ledger completeness only; no original acceptance, source
fix, release, adoption or optional research is completed merely by linkage.

**Issue home:** Existing [roadmap #42](https://github.com/egohygiene/egolint/issues/42); no new
duplicate parent.

## Positive observations

The strongest part of this repository is the explicit separation of execution, contracts and
evidence. Several concrete controls already work and should be retained while repairing the gaps:

- Configuration is typed, ordered and explainable. Unknown fields and wrong versions are rejected in
  the native configuration layer; CI excludes implicit user/local overrides while allowing explicit
  selections.
- Host process construction uses executable-plus-argv. Default container plans disable network
  access, drop capabilities, set no-new-privileges, bound PIDs and reserve a fixed writable report
  boundary around a read-only source mount. Report-parent symlink/canonical-alias checks, bounded
  adapter output and atomic report replacement are present. These are observed controls, not an
  exhaustive security certification.
- Fix preview and reviewed apply are different capabilities. The default disposable preview control
  produces the expected patch while preserving original bytes/index; apply requires the exact patch
  digest, base commit and expected post-tree, verifies a clean root and attempts recovery when final
  verification fails. The ordinary custom-config/executable failures are repairable without
  abandoning that architecture.
- Native and adapter reports retain rule IDs, source locations, source selections, remediation and
  explicit applicability/coverage states. The public debt projection is deliberately narrower than
  raw scanner payloads.
- The focused Git-ignore command uses metadata-only inventories and actual isolated Git behavior
  instead of reading every ordinary repository payload. Source integrity, content, nested policies,
  tracked files and effective behavior have distinct evidence. This review did not find a new public
  Git-ignore defect.
- Versioned source locks preserve exact upstream identity and lifecycle. Native ADR support retains
  the accepted current four-digit policy and explicit historical tuples; tests preserve owner
  fixtures instead of rewriting them into the latest policy.
- MegaLinter policy records v10.0.0 at `15e5b45552097e318c93de385779ce3b1084052c`, with a generated
  matrix of 124 tools, 11 result states, separate profile snapshots and owned disabled/advisory
  reasons. A skipped tool is not supposed to become a pass; the focused normalization repairs should
  reinforce that existing contract.
- The lightweight and full images have different, documented purposes. Full-image policy is
  flattened at build time, Node dependencies are locked with lifecycle scripts disabled, and
  JSONSkooma's six gem archives are content-addressed. Actual hosted image/adapter smokes support
  those packaging claims within their recorded scope.
- Integrations remain check-only by default and the consumer action does not upload reports or
  invoke fix. Their distribution bundle has a version-bound per-member manifest and demonstrably
  deterministic archive construction.
- NOTICE and container documentation distinguish original MIT content, MegaLinter AGPL-3.0-only,
  derived BSD prose and independently licensed analyzers. This is a useful license boundary
  inventory; complete legal/license conformance was not certified here.
- Tests and self-consumer workflows cross public CLI/Task boundaries. They provide useful baseline
  confidence even though the targeted counterexamples reveal uncovered branches.
- Existing roadmap #42 already states the correct broad product boundary. Its history, operating
  principles and future program exits can be preserved while adding complete tactical coverage.

## Suggested issue backlog and coverage reconciliation

This is an audit-local proposed issue map, not a newly ratified organization report schema. The
symbolic keys below identify reviewed homes before provider numbers are assigned. There are **17
focused new homes plus existing #42**; multiple findings share a home only when the acceptance
criteria remain explicit.

| Proposed home key                                               | Findings                        | Bounded outcome                                                                                                                           |
| --------------------------------------------------------------- | ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `NATIVE_REPORT_LIFECYCLE`                                       | AUDIT-003                       | Keep every focused canonical report tied to the current invocation, including unselected and failed subsequent runs                       |
| `JS_ADAPTER_EVIDENCE`                                           | AUDIT-012                       | Preserve actual analyzer status, supported envelopes and protocol-aware normalization without turning absent evidence into a pass         |
| `INTELLIGENCE_ADOPTION_STATUS`                                  | AUDIT-008                       | Propagate contradictory represented-source selections into the focused semantic status                                                    |
| `PRESENTATION_INTEGRITY`                                        | AUDIT-004                       | Require complete referenced asset/descriptor checksum coverage under the selected owner contract                                          |
| `CONTINUITY_SCHEMA_PARITY`                                      | AUDIT-009                       | Make native structure/format acceptance agree with the exact selected portable schema                                                     |
| `RELEASE_INPUT_CONTRACT`                                        | AUDIT-016                       | Reject malformed expected image/product records and retain reviewed environment/platform controls                                         |
| `RELEASE_TASK_BINDING`                                          | AUDIT-007                       | Prove the supported static Task handoff binds to the actual declared workflow argument                                                    |
| `MARKDOWN_SOURCE_GRAMMAR`                                       | AUDIT-005                       | Exclude literal Markdown examples from live ADR/roadmap structure and graph records                                                       |
| `ADR_REFERENCE_GRAMMAR`                                         | AUDIT-006                       | Validate HTTP authority and supported GitHub repository/identifier components offline                                                     |
| `FIX_PREVIEW`                                                   | AUDIT-001, AUDIT-002            | Support committed custom configurations and ordinary tracked executables in isolated previews, with separate acceptance for both failures |
| `PORTABILITY_PATH_PREFIXES`                                     | AUDIT-010                       | Detect folded file/directory-prefix conflicts and documented ancestor casing collisions                                                   |
| `PORTABILITY_INCLUDE_MATRIX`                                    | AUDIT-011                       | Recognize supported static matrix.include OS rows in the shell advisory                                                                   |
| `JS_MANIFEST_CONTRACT`                                          | AUDIT-013                       | Enforce the published closed package-quality manifest grammar before adapters start                                                       |
| `JSON_SKOOMA_CONFIG`                                            | AUDIT-014                       | Emit structured configuration outcomes for malformed selector types without starting Ruby                                                 |
| `JSON_SKOOMA_REPORT_ROUTING`                                    | AUDIT-015                       | Resolve relative output once so activation does not change the report destination                                                         |
| `CLI_INPUT_EXIT`                                                | AUDIT-017                       | Keep missing native policy/suppression inputs in the documented configuration-error category                                              |
| `CURRENT_DOCS`                                                  | AUDIT-018, AUDIT-019, AUDIT-020 | Reconcile current ADR source tuples, support commands, source navigation and implementation inventory                                     |
| Existing [#42](https://github.com/egohygiene/egolint/issues/42) | AUDIT-021                       | Represent the full local backlog once, retain exact prior program context and verify native coverage                                      |

The existing open scopes are preserved. New bug homes do not reopen closed foundations #24/#27/#14
or duplicate general registry #35, future claims engine #46, checkpoint-report validator #48,
release program #5, continuity program #55 or historical ADR migration #60.

All 28 original issue bodies were read in full, including parent #42; the sole comment on #5 retains
actual signed release and two-consumer adoption acceptance. All complete original label sets were
empty. At the starting observation all original native parent/child sets were empty. The old parent
directly linked 10 of the 27 current work issues and omitted 17, while retaining useful closed and
cross-repository context.

The authorized refresh should keep #42's identity, title and labels; put the dated current ledger
first, and retain its exact complete original body as clearly dated historical program context.
Preserve original work-issue text and append any necessary dated reconciliation. Body-level release
checkpoints, `Roadmap-Key` and organization `Program`/`Parent` references remain intact even when
the complete local native ledger uses #42 as the common parent.

Asterisks mean an identified predecessor, required input, lifecycle/approval or explicit scheduling
gate for the **full outcome**. They do not prohibit useful bounded preparation. The combined
proposal has 17 ready new homes and seven ready original scopes, hence **24 independently startable
/ 20 gated / 44 work items**. Native attachment, exact set/order, all original text/labels, the
current next action and provider completion counts require a final live readback; a checked Markdown
count alone is not proof.

## A finite path to v1

The source-buildable product does not need every optional tool in the backlog to become useful. A
bounded release path is:

1. Repair current report, validator, adapter, fix-preview and documentation correctness in focused
   review checkpoints. Re-run the meaningful positive and negative cases for each repair; passing
   this audit's reproduction is evidence of the original behavior, not a regression test for the
   intended fix.
2. Continue current release-v1 fixture matrix **#68**, then final dogfood **#69**, then reconcile
   parent **#29**. Its earlier source checkpoints and PRs are already delivered. Successor **#76**
   is a separate contract delta and should not restart or silently replace this chain.
3. Define the current formatter owner matrix in **#53**, and curate the reusable policy corpus under
   **#51** when its reviewed public-safe input is available. **#54** evaluates gaps/overlap after
   its stated baseline-acceptance entry condition; popularity or tool count is not an adoption
   criterion.
4. Use **#5** to qualify the chosen first release, signed immutable artifacts, supported platform
   truth and digest-pinned adoption in Empathy plus another consumer. The generic private follow-up
   under SECURITY remains a separate release review gate. Source implementation, green CI and a
   closed prerequisite issue do not substitute for publication/adoption evidence.
5. Extend adaptive activation **#35** and coherent distribution-wide manual/observe/ratchet/enforce
   behavior **#36** through bounded design and implementation checkpoints, retaining their declared
   full-outcome gates. Existing focused continuity modes do not complete the broader distribution
   capability.

Preserve optional tracks without turning them into universal blockers: Betterleaks **#52**,
Copywrite **#47**, CSpell tooling **#72**, format-owned diagrams **#74**, EPUBCheck **#77**,
ast-grep **#37**, Rust dependency/API adapters **#38/#39**, web-runtime and authorized DAST
**#40/#41**, and full-engine ARM64 **#45**. An evidence-backed defer/reject decision is a valid
evaluation outcome where the original scope permits it. A minimal/native/offline path remains
useful.

Contract expansion stays with its owners: prospective commit semantics **#49**, diagnosis records
**#50**, checkpoint evidence **#48**, general claims drift **#46**, successor release **#76**,
released continuity acceptance **#55** and explicitly scheduled ADR backfill **#60**. Bounded parser
or fixture preparation can start where appropriate, but missing accepted inputs, human dispositions
and actual release gates must remain explicit. Never infer that every organization repo must be
finished before a local current-contract repair can proceed.

## Opportunities by area

| Area                   | Useful next improvement                                                                         | Evidence/owner boundary                                                 |
| ---------------------- | ----------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| Developer experience   | Restore supported fix previews, stable exit categories and truthful current navigation          | Existing CLI/lifecycle contracts; new focused repairs                   |
| Evidence quality       | Keep reports invocation-scoped and make invalid/unknown/unavailable states survive aggregation  | Current normalized report contracts; no publication approval inferred   |
| Validator design       | Reduce divergence between selected schemas and handwritten shape/format/Markdown/URL heuristics | Reuse pinned contracts and bounded parsers; retain offline operation    |
| Analyzer adapters      | Preserve status and supported protocol shape before deriving findings                           | Existing pinned analyzers remain semantic owners                        |
| Distribution           | Qualify actual immutable release and two real consumer adoptions                                | #5; current CLI dual-platform/full-engine AMD64-only support            |
| Policy curation        | Use one formatter owner and a provenance-aware reusable corpus                                  | #53/#51; reviewed source/export and explicit consumer exceptions        |
| Optional capabilities  | Evaluate unique value, overlap and maintenance cost before activation                           | #54 and dedicated capability issues; controlled execution/authorization |
| Continuity and roadmap | Keep the compact handoff and complete issue ledger synchronized with verified source            | Operational checkpoint plus #42; history remains immutable              |

## Uncertainties, deferred work and exclusions

- This is a bounded public repository audit, not exhaustive security, performance, accessibility,
  legal/license, cross-platform runtime or adversarial concurrency certification. The private
  follow-up is governed separately by SECURITY.
- Local Docker/Podman execution was unavailable. Exact-head hosted image/dogfood success is recorded
  with provider attribution; it does not prove a new release pipeline run, all registry states,
  ARM64 full-engine support or every arbitrary consumer.
- No GitHub release or local tag was observed. Registry inventory, release variables,
  protected-environment reviewers and tag-ruleset configuration were not inspected or changed. Their
  presence and effectiveness remain release acceptance checks, not assumed facts.
- Native advisory/observe exit 0 can coexist with invalid, unavailable, unknown or incomplete
  semantic evidence. Severity, process exit, applicability, semantic status and actual external
  publication are distinct dimensions.
- The schema checks cover local schema meta-validity and generated-byte parity. They are not
  exhaustive instance validation, proof of runtime enforcement, remote reference evaluation or
  conformance to an unfinished canonical checkpoint-report schema.
- Static source grammar checks do not execute arbitrary shell, fetch URL destinations, authenticate
  represented commits, judge ADR approval substance or infer human dispositions. Consumer source
  identity and any live-state adapter remain separately trusted inputs.
- Source lock lifecycle labels are retained. Accepted ADR policy and four-digit naming do not ratify
  proposed roadmap/continuity contracts or promote observe mode. #55 and #60 preserve released-input
  and scheduling constraints.
- No private incubator commit, configuration contents, raw telemetry or unrelated account context
  was included. #51's reviewed public-safe export/source-freeze and #54's baseline-acceptance
  condition remain unresolved inputs.
- Declared graph and relative-link checks omit code examples, test fixtures, vendored/generated
  Markdown, anchors, external destinations and prose truth. They confirm bounded navigation
  structure only.
- One proposed synthetic SARIF observation was rejected during review after its input was found not
  to satisfy protocol semantics. It is not a finding. Corrected non-failure controls and the
  protocol's supported semantics remain meaningful validation for the adapter repair.
- No implementation repair, dependency upgrade, policy acceptance, issue closure, repository-root
  reorganization, historical ADR migration, release or deployment is completed by recording this
  audit or linking a work item.

## Evidence index and reproduction

The [normalized validation receipt](data/egolint-completeness-20261020T222416Z.validation.json)
records public source/guidance identities, baseline command results, schema parity/meta checks,
source inventory, bounded navigation checks, exact-head hosted run/job/step outcomes, issue-coverage
observations and final root probe results. It is audit-local evidence, not delivery of the future
organization checkpoint-report contract tracked by #48.

| Portable artifact                                                                            | What it demonstrates                                                                      | Important limit                                                                                           |
| -------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| [Core probes](data/egolint-completeness-20261020T222416Z.core-repros.py.txt)                 | Actual native fix configuration/executable controls and focused-report/exit lifecycle     | Docker executable is a bounded simulator; no real engine qualification                                    |
| [Distribution probes](data/egolint-completeness-20261020T222416Z.distribution-repros.py.txt) | Actual copied release-input CLI and deterministic integration archive assembly            | Disposable repository-authored metadata; no publication/registry execution                                |
| [Intelligence probes](data/egolint-completeness-20261020T222416Z.intelligence-repros.py.txt) | Actual presentation, Markdown, link, represented-source and static release handoff checks | Synthetic public sources; no arbitrary Task command or URL execution                                      |
| [Validator probes](data/egolint-completeness-20261020T222416Z.validators-repros.py.txt)      | Paired actual native/pinned-schema and portability controls                               | Exact local schema and selected rule scope, not complete OS checkout proof                                |
| [Adapter probes](data/egolint-completeness-20261020T222416Z.adapters-repros.py.txt)          | Package-quality and JSONSkooma selector/evidence boundaries                               | Trusted subprocess/Ruby stubs are marked; no claim current analyzers naturally emit every synthetic shape |

The portable programs are immutable plain-text evidence snapshots with a `.py.txt` suffix. Their
executed source bytes are preserved; they remain directly runnable with `python3`. Read each
program's argument help and run it against the exact frozen source with the documented local
toolchain and source-built binary. Re-run in disposable directories; preserve the repository
worktree and compare normalized case results, not temporary paths. Script and result identities in
the receipt bind the selected public probes to this assessment.

Equivalent public commands for the observed baseline checks:

```sh
cargo fmt --all --check
cargo build --locked --bins
cargo clippy --all-targets --all-features --locked -- --deny warnings
cargo test --all-targets --all-features --locked
cargo package --locked --offline

python3 scripts/validate_megalinter_policy.py --check
python3 scripts/complementary_tools.py --check
python3 scripts/validate_release_inputs.py
python3 scripts/validate_repository_release_sources.py
python3 scripts/package_integrations.py --check
python3 -m unittest discover --start-directory "tests" --pattern "test_*.py" --verbose

pnpm run commitlint:test
pnpm run eslint-config:test
pnpm run javascript-quality:test
```

The recorded Clippy invocation used Rust's `-D warnings` spelling; `--deny warnings` above is its
equivalent long form. Native self-consumer, architecture and package-quality invocations include
explicit selected policy, represented source, comparison evidence and evaluation date in the
receipt. Their per-command times and nonzero/limited evidence outcomes are retained without copying
raw logs into the report.

## Validation notes and continuation

The report model includes the required scope/defaults, discovery, prior-audit awareness,
classification/severity/confidence/status/area/effort/impact, observed evidence, impact,
recommendations, suggested validation, dependencies, positive observations, uncertainty and
candidate backlog. All finding IDs are unique and grouped by severity. Public proof cases are
independent reproductions of source behavior; they are not implementation fixes.

Before the publication candidate is handed off, verify its exact report/receipt/probe scope,
immutable artifact bytes, current continuity structure/budgets/privacy, original issue preservation
and exact native child coverage under #42. Verify any candidate CI separately against its actual
head. The frozen report continues to describe `834a230e6c0f4ee8b080a10d6c2d469be5eb6e1b`; later
ledger updates and the containing PR do not retroactively mark these findings resolved.

For implementation, resume from the live dated portion of #42 and the current operational continuity
checkpoint, recheck overlapping work, and select one bounded ready issue. Update its observed
behavior, meaningful checks, remaining limits and review state before claiming completion.

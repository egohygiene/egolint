# Progressive enforcement

Issue [#36](https://github.com/egohygiene/egolint/issues/36) separates adoption
from the actual validation result. The default is `enforce`, preserving the
existing blocking policy. Each per-tool advisory/blocking decision remains
owned by its policy.

| Mode      | Execution and process result                                                                                                                                                  |
| --------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `manual`  | Local invocation or workflow dispatch runs normal validation. Automatic events write an explicit skipped-run record without launching validators or the engine.               |
| `observe` | Runs validation and retains findings; completed findings are nonblocking. Configuration and execution failures still fail.                                                    |
| `ratchet` | A separately pinned reviewed baseline tolerates unchanged owned debt while new/worsened blocking debt fails. Invalid, missing or incompatible evidence cannot waive findings. |
| `enforce` | Existing applicable blocking findings fail; tool-level advisory findings remain visible.                                                                                      |

`--enforcement` and `EGOLINT_ENFORCEMENT` follow normal CLI/environment
precedence. Repository configuration uses `[enforcement] mode = "observe"`.
`--invocation` supplies explicit caller provenance. Otherwise GitHub event names
are mapped to typed events; missing/unknown provenance in CI is automatic,
never implicitly manual. This is invocation data, not independent authentication
or permission to publish. Relay/workflows own their authenticated event context.

The run's `status`, findings, suppressions and raw adapter exit remain intact.
`egolint_exit_code` is the effective adoption result. An `enforcement` object
records mode, invocation, skipped execution, baseline state and finding changes.
Human output and SARIF invocation properties expose the same distinction;
SARIF findings retain canonical fingerprints and adoption state. A nonblocking
observe result does not mean the underlying findings disappeared.

## Reviewable baseline workflow

1. Inspect a clean committed checkout. Run the intended scope with observe or
   enforce mode and retain its normal report. The reserved `.reports/egolint`
   output does not make the represented source dirty, even without an ignore
   rule. Other untracked or modified source still does.
2. Create a candidate from that report and exact checkout. Create the chosen
   parent directory first; the command refuses to overwrite an existing file.

   ```sh
   egolint baseline --report .reports/egolint/run.json \
     --output .egolint/baseline.v1.json --repository owner/repository \
     --evaluation-date 2026-11-09 --expires-on 2026-12-09 \
     --reviewed-by reviewer-handle --review-reference owner/repository#123
   ```

3. Review the candidate and diagnostic report. Metadata is caller-supplied;
   EgoLint does not invent or independently authenticate approval. Pin the exact
   printed SHA-256 separately in trusted, reviewed configuration.
4. Set `[enforcement]` fields `mode = "ratchet"`, `repository`, `baseline`, and
   `baseline-sha256`. Invoke validation with `--evaluation-date YYYY-MM-DD`.
   Keep the pin outside the baseline bytes. Changing both candidate and pin is
   an explicit review change, not automatic debt acceptance.

A baseline records schema version, repository, represented commit/tree, policy
identity, tool versions, exact executable identity, review reference, dates,
and grouped finding fingerprints/counts/severity. It contains no source excerpt
or raw diagnostic message. Its source must be locally available and an ancestor
with the recorded tree. Engine-backed baselines require an immutable image
digest; catalog declarations alone do not prove a running tool's identity.
Only fully normalized completed executions from clean source can establish a
baseline. The run records its producing executable SHA-256; baseline creation
requires that exact executable rather than assigning the current tool's identity
to an older report. Older reports missing effective policy or producing-executable
provenance need a fresh run.

Provider process completion alone does not establish comparable debt. A
full-engine report that retains aggregate-only diagnostics is partial and
cannot establish a reviewed baseline; use fully normalized adapter or native
evidence. This limitation stays explicit rather than allowing aggregate counts
to waive unknown new findings.

Effective policy identity includes configuration, explicit selections and native
policy inputs, including local MegaLinter extensions. Adoption mode and baseline
pin/path are excluded to avoid a self-referential checksum and permit promotion
from observe to ratchet. Tool-policy scope or executable upgrades produce an
incompatible baseline requiring review, not a repository-regression claim.

Fingerprints bind tool, rule, path and exact message; positions and severity do
not define identity. Line movement therefore stays unchanged. Severity or
duplicate-count increases are worsened debt. Renames conservatively produce
new/resolved entries; the tool does not infer that a rename waives a finding.
Suppression is independent, visible and cannot hide an unsuppressed duplicate.
States include resolved, unchanged, new, worsened, suppressed, expired and
unavailable. Expired reviews block ratchet until explicitly renewed. Every
blocking tool outcome needs comparable findings from that tool. Incompletely
normalized failing adapter output remains unavailable to ratchet even when
another tool has reviewed unchanged debt. Partial advisory detail with a clean
adapter exit does not invent an execution failure. An unattributed blocking
adapter result cannot be baselined away.

## Promotion and rollback

Promote through manual/observe, an explicit reviewed ratchet baseline, then
enforce as debt is resolved. Change the mode through the normal reviewed config
path to roll back adoption; retained findings and expired/unavailable evidence
stay visible. Fix generation/apply remains checksum-bound and independent of
adoption. No mode permits automatic in-place fixes, changes branch protections,
publishes an artifact or modifies the inspected source.

The implementation batch's [verification ledger](work-27-37.md) records actual
checks and limits. Full-image/non-Linux and hosted acceptance are separate;
this document does not claim their success.

# Release design

`.github/workflows/release.yml` is a manual release candidate pipeline. Its presence does not
prove that a release or public artifact exists: the first write-once tag, signed artifacts, OCI
digests, attestations, and post-push smoke evidence remain observed closure gates.

## Current source adoption

The repository's [.egohygiene/release.json](../.egohygiene/release.json) records one Cargo authority
under the incubating `cli-library` profile. Its local source plan leaves the version/changelog decision
pending maintainer review. `release:plan` prints that draft; `release:prepare` exclusively writes a
caller-selected Markdown review handoff; `release:verify` invokes local source validators.

The literal `release:publish` Task opens the manual workflow input handoff. `operation` defaults to
`rehearse`; `publish` requires a separately selected successful rehearsal run, exact reviewed main
revision, matching Cargo version and existing version tag. Tag pushes do not trigger publication.
Native self-dogfood passes all seven current-v1 source checks while retaining incubating/advisory
rollout. This source result leaves signed publication and consumer adoption under #5, and final
checkpoint acceptance under #69/#29, open.

The helper records relative paths and hashes, preserves authored Cargo/changelog/declaration bytes,
and refuses to overwrite an existing preparation file. Review a source handoff with:

```sh
python3 scripts/repository_release_plan.py --workspace "."
python3 scripts/repository_release_plan.py --workspace "." --output ".reports/egolint/release-review.md"
```

No release Task or workflow was executed to qualify this source checkpoint. Hosted final-head,
full-image and non-Linux qualification remain deferred.

The [acceptance ledger and consumer handoff](repository-release-acceptance.md) identify the exact
source-qualified producer, retained self-findings and remaining maintainer/hosted gates.

## Planned products

| Product                                        | Planned channel                     |
| ---------------------------------------------- | ----------------------------------- |
| `egolint` crate source and Linux x86-64 binary | GitHub Releases                     |
| crates.io and additional platform installers   | Later, separately approved channels |
| Lightweight CLI image                          | `ghcr.io/egohygiene/egolint`        |
| MegaLinter engine/policy image                 | `ghcr.io/egohygiene/egolint-full`   |

## Promotion checklist

1. Update the version and changelog from one reviewed commit.
2. Merge that commit to `main`, then run the **Release** workflow manually with `operation=rehearse`,
   the exact Cargo version and full main revision. This rehearsal must pass before a tag is created. It has
   read-only repository permission and does not sign bytes, push images, create tags, or publish a
   GitHub release.
3. Run formatting, tests, policy validation, the reusable action, and clean and findings consumer
   smoke tests against both local images.
4. Confirm crate contents with `cargo package --list` and a dry run.
5. Use a crates.io trusted publisher rather than a long-lived API token if that distribution channel
   is approved later. The current workflow does not publish to crates.io.
6. Build the CLI OCI manifest for `linux/amd64` and `linux/arm64`, and the full-engine image for its
   honestly supported `linux/amd64` platform, from the same commit.
7. Pin every container base, including MegaLinter and Node, to the exact references reviewed in
   `.config/release/base-images.v1.json`.
8. Generate CycloneDX/SPDX SBOMs and build provenance for binaries and images.
9. Sign image digests with keyless Sigstore/Cosign after provenance succeeds.
10. Create the protected write-once Git version tag at that source, then request `operation=publish`
    from `main` with the same version/revision and successful rehearsal run ID. `latest`, `edge`,
    moving action tags and crates.io publication remain separate channels.
11. Verify downloaded archives and `oci://` images against the repository's attestations before
    announcing the release.

Both operations execute the reviewed workflow from `main`. The request validator checks the supplied
full revision against the dispatch SHA, checkout and freshly fetched main, and the version against
`Cargo.toml`. Publication also checks authenticated Actions metadata for the same repository,
workflow, main revision, successful completed status and exact non-publishing rehearsal run title.
It rejects a run approving itself, another workflow/source, failed or incomplete rehearsal, and a
missing or mismatched version tag. A local caller-supplied JSON fixture is useful for tests; the
production workflow fetches metadata with its read-only Actions token.

Both operations validate the same digest-qualified base-image variables. A rehearsal artifact is
temporary candidate evidence, not a release or approved consumer pin. Publication rebuilds from
the exact source and verifies the newly produced bytes; it does not promote rehearsal artifacts.

An authorized maintainer can start the rehearsal after the release-preparation pull request merges:

```sh
release_revision="$(git rev-parse --verify HEAD)"
gh workflow run .github/workflows/release.yml --ref main \
  --field operation=rehearse --field version=0.1.0-alpha.1 \
  --field "revision=${release_revision}"
```

After a successful rehearsal, an authorized maintainer creates the protected `v0.1.0-alpha.1` tag
from that exact commit and selects the successful rehearsal's run ID for a second manual dispatch:

```sh
gh workflow run .github/workflows/release.yml --ref main \
  --field operation=publish --field version=0.1.0-alpha.1 \
  --field "revision=${release_revision:?select the reviewed main revision}" \
  --field "rehearsal_run=${release_rehearsal_run:?select the successful rehearsal run ID}"
```

If main advances, rehearse the new reviewed source and choose its version/tag deliberately; an
old successful run cannot authorize a changed commit. The default operation remains rehearsal.

The release workflow fails closed unless the Rust, Debian, Node, and MegaLinter base images are
supplied as reviewed manifest digests. Its unprivileged candidate job has no OIDC or attestation
authority. A separate job behind the protected `release` environment downloads those exact bytes,
verifies their checksums, signs the checksum manifest, associates the source SPDX SBOM with the
crate, and associates a separately generated binary/archive SBOM with the Linux tarball. A fresh job
then verifies the downloaded bytes, signature, attestations, and executable archive.

OCI builds initially publish only per-attempt `release-candidate-<run>-<attempt>` quarantine tags. A
build fails closed if its attempt tag already exists; it never trusts, resumes, or replaces a
previous quarantine reference. The build/smoke matrix has package write access but no OIDC
authority. It executes the CLI entrypoint on AMD64 and ARM64. The full engine's inherited MegaLinter
entrypoint runs against a minimal clean consumer on its supported AMD64 platform only; it does not
promise a full-engine ARM64 image. Every packaged full-image profile is parsed separately.

A separate protected, no-checkout job downloads the exact digest evidence from the current workflow
run, resolves that immutable `image@sha256` reference directly, signs and attests both products, and
only then creates the version tags. Mutable quarantine tags are not authorization evidence. A rerun
rebuilds into a fresh attempt tag; if a public version tag exists, promotion succeeds only when it
already resolves to the newly authorized digest. Any conflicting tag or indeterminate registry
response fails closed. The announcement job resolves both public tags again before publishing the
release.

Automated GHCR deletion can remove the digest referenced by a promoted tag, so the workflow
intentionally leaves attempt quarantine tags for a reviewed package-retention policy instead of
risking deletion of verified content. GHCR does not enforce immutability for the public version tag
at the registry layer; the workflow's write-once check narrows but cannot eliminate a registry-side
time-of-check/time-of-use race. Consumers must pin digests. A draft GitHub release is promoted only
after all of those checks pass.

Before any tag is pushed, configure the `release` environment with required reviewers, add a
repository tag ruleset, and set these repository variables to the exact immutable references in the
checked-in release-input contract:

| Variable                     | Image boundary                 |
| ---------------------------- | ------------------------------ |
| `EGOLINT_RUST_BUILDER_IMAGE` | Rust 1.85 builder              |
| `EGOLINT_CLI_RUNTIME_IMAGE`  | Minimal Debian CLI runtime     |
| `EGOLINT_NODE_POLICY_IMAGE`  | Node policy dependency stage   |
| `EGOLINT_MEGALINTER_IMAGE`   | MegaLinter v10 full-image base |

The input validator requires a complete closed contract before comparing environment values. Every
image record contains `reference`, `digest`, `media_type` and a nonempty unique supported `platforms`
list; `upstream_platform_evidence` is optional nonempty text. Every product contains a unique exact
`base_images` list and a nonempty unique supported `platforms` list; `limitation` is optional nonempty
text. Unsupported fields, missing/non-object records and malformed nested elements produce controlled
configuration errors. Only a structurally valid contract proceeds to base-platform intersection checks
and exact environment comparison.

The workflow rejects missing, tag-only, merely well-formed, or drifted values. It freezes only the
four references that match `.config/release/base-images.v1.json` byte-for-byte.

MegaLinter `v10.0.0` publishes its full image as a single `linux/amd64` manifest. Version 10 adds
ARM64 support for compatible custom and standalone-linter builds, but does not make the complete
124-tool image multi-architecture; upstream tracks complete-image support in
[MegaLinter #1180](https://github.com/oxsecurity/megalinter/issues/1180). The reviewed first-release boundary selects
a lightweight CLI image for AMD64 and ARM64 and `egolint-full` for AMD64 only. The product-platform matrix is derived from the intersection of its reviewed base images
and fails closed if the contract overclaims a platform. ARM64 full-engine publication remains
blocked on an upstream-compatible base or a separately reviewed reduced flavor.

GitHub Actions used by generated or handwritten workflows are pinned to reviewed immutable commit
SHAs. `.github/workflows/ci.yml` checks the Rust MSRV, package contents, generated schemas, policy
contracts, Node lockfile, commit policy, lightweight image, and a native-architecture full-image
build with embedded Python and Node policy smokes.

Do not close distribution or adoption issues from workflow code alone. Closure requires one
successful immutable release plus pinned consumption by Empathy and at least one non-Empathy pilot
repository.

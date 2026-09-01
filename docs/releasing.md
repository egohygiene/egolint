# Release design

`.github/workflows/release.yml` is a tag-driven release candidate pipeline. Its presence does not
prove that a release or public artifact exists: the first write-once tag, signed artifacts, OCI
digests, attestations, and post-push smoke evidence remain observed closure gates.

## Planned products

| Product                                        | Planned channel                     |
| ---------------------------------------------- | ----------------------------------- |
| `egolint` crate source and Linux x86-64 binary | GitHub Releases                     |
| crates.io and additional platform installers   | Later, separately approved channels |
| Lightweight CLI image                          | `ghcr.io/egohygiene/egolint`        |
| MegaLinter engine/policy image                 | `ghcr.io/egohygiene/egolint-full`   |

## Promotion checklist

1. Update the version and changelog from one reviewed commit.
2. Merge that commit to `main`, then run the **Release** workflow manually with the exact Cargo
   version. This rehearsal must pass from the current `main` commit before a tag is created. It has
   read-only repository permission and does not sign bytes, push images, create tags, or publish a
   GitHub release.
3. Run formatting, tests, policy validation, the reusable action, and clean and findings consumer
   smoke tests against both local images.
4. Confirm crate contents with `cargo package --list` and a dry run.
5. Use a crates.io trusted publisher rather than a long-lived API token if that distribution channel
   is approved later. The current workflow does not publish to crates.io.
6. Build OCI manifests for `linux/amd64` and `linux/arm64` from the same commit.
7. Pin every container base, including MegaLinter and Node, by reviewed multi-architecture manifest
   digest.
8. Generate CycloneDX/SPDX SBOMs and build provenance for binaries and images.
9. Sign image digests with keyless Sigstore/Cosign after provenance succeeds.
10. Publish only a write-once version tag. `latest`, `edge`, moving action tags, and crates.io
    publication require separate approval after adoption evidence.
11. Verify downloaded archives and `oci://` images against the repository's attestations before
    announcing the release.

The manual rehearsal is intentionally the only branch-mode execution path. It rejects any selected
ref that is not the current `main` commit, requires its version input to equal `Cargo.toml`, and
validates the same digest-qualified base-image variables used by tag promotion. A rehearsal artifact
is temporary candidate evidence, not a release and not an approved consumer pin.

An authorized maintainer can start the rehearsal after the release-preparation pull request merges:

```sh
gh workflow run "Release" --ref "main" --field "version=0.1.0-alpha.1"
```

Only after that run is green should the protected `v0.1.0-alpha.1` tag be created from the rehearsed
commit. The tag run rebuilds from source; it never promotes the rehearsal's temporary artifacts.

The release workflow fails closed unless the Rust, Debian, Node, and MegaLinter base images are
supplied as reviewed manifest digests. Its unprivileged candidate job has no OIDC or attestation
authority. A separate job behind the protected `release` environment downloads those exact bytes,
verifies their checksums, signs the checksum manifest, associates the source SPDX SBOM with the
crate, and associates a separately generated binary/archive SBOM with the Linux tarball. A fresh job
then verifies the downloaded bytes, signature, attestations, and executable archive.

OCI builds initially publish only per-attempt `release-candidate-<run>-<attempt>` quarantine tags. A
build fails closed if its attempt tag already exists; it never trusts, resumes, or replaces a
previous quarantine reference. The build/smoke matrix has package write access but no OIDC
authority. It inspects both manifest platforms, executes the CLI entrypoint on amd64 and arm64, and
executes the full image's inherited MegaLinter entrypoint against a minimal clean consumer on both
architectures. It also parses every packaged full-image profile as a separate packaging check.

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
repository tag ruleset, and set these repository variables to reviewed multi-architecture manifest
digests:

| Variable                     | Image boundary                 |
| ---------------------------- | ------------------------------ |
| `EGOLINT_RUST_BUILDER_IMAGE` | Rust 1.85 builder              |
| `EGOLINT_CLI_RUNTIME_IMAGE`  | Minimal Debian CLI runtime     |
| `EGOLINT_NODE_POLICY_IMAGE`  | Node policy dependency stage   |
| `EGOLINT_MEGALINTER_IMAGE`   | MegaLinter v10 full-image base |

The workflow rejects missing, tag-only, or syntactically invalid values and freezes the four
accepted digest references as candidate outputs for the multi-architecture publish jobs.

GitHub Actions used by generated or handwritten workflows are pinned to reviewed immutable commit
SHAs. `.github/workflows/ci.yml` checks the Rust MSRV, package contents, generated schemas, policy
contracts, Node lockfile, commit policy, lightweight image, and a native-architecture full-image
build with embedded Python and Node policy smokes.

Do not close distribution or adoption issues from workflow code alone. Closure requires one
successful immutable release plus pinned consumption by Empathy and at least one non-Empathy pilot
repository.

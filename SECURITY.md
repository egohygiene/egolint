# Security policy

Egolint is pre-release software. Security fixes currently target the `main`
branch; no version has a guaranteed support window yet.

Do not report a suspected vulnerability in a public issue. Use GitHub's private
vulnerability reporting for `egohygiene/egolint` if it is enabled. A secondary
private contact has not yet been designated; if private reporting is
unavailable, do not include vulnerability details in a public issue.

Include the affected commit or version, operating system, container runtime,
minimal reproduction, impact, and whether untrusted repository content or
network access is required. Do not include real credentials or sensitive data.

Reports involving unsafe argument construction, workspace escape, unintended
write access during `check`, plan redaction failures, image provenance, or the
full-image supply chain are especially useful. Dependency findings without a
demonstrated reachable impact may be handled through routine maintenance.

No response-time SLA is promised during the alpha. The project will acknowledge
validated reports privately and coordinate disclosure after a fix is available.

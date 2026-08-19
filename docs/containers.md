# Containers

Two OCI products are planned under GHCR. They are build definitions in this
alpha; their presence in a registry is not asserted.

## `ghcr.io/egohygiene/egolint`

`Dockerfile` builds the Rust CLI and runs it as UID/GID 65532. It is suitable for
schema and plan inspection:

```sh
docker build --file "Dockerfile" --tag "egolint:local" "."
docker run --rm --volume "$PWD:/workspace:ro" "egolint:local" \
  plan --workspace "/workspace"
```

Actual `check` and `fix` commands currently require a Docker or Podman executable
and responsive daemon on the same host as the CLI. The image intentionally does
not bundle a runtime client. Mounting `/var/run/docker.sock` would grant broad
host control and is outside the supported design.

## `ghcr.io/egohygiene/egolint-full`

`Dockerfile.full` extends `ghcr.io/oxsecurity/megalinter:v10.0.0`, embeds policy,
materializes the locked Node-based policy dependencies with lifecycle scripts
disabled, flattens the fast profile so it has no runtime `EXTENDS` dependency,
and preserves the upstream entrypoint. Build it locally with:

```sh
docker build --file "Dockerfile.full" --tag "egolint-full:local" "."
```

For releases, pass the reviewed upstream manifest-list digest:

```sh
docker buildx build \
  --file "Dockerfile.full" \
  --build-arg "MEGALINTER_IMAGE=ghcr.io/oxsecurity/megalinter@sha256:<digest>" \
  --platform "linux/amd64,linux/arm64" \
  --tag "ghcr.io/egohygiene/egolint-full:<version>" \
  "."
```

The full image extends and contains MegaLinter. MegaLinter is AGPL-3.0-only,
Egolint's original content is MIT, and bundled linters have their own licenses.
Preserve the applicable notices and publish an SBOM with every image. See
`NOTICE`.

> [!WARNING]
> These Dockerfiles have not yet been smoke-built in this workspace because no
> Docker or Podman executable is available. Publishing is blocked until clean
> builds and arbitrary-consumer-repository smoke tests pass on both amd64 and
> arm64.

## Runtime defaults

The native CLI launches Docker or Podman as direct host argv without host-shell
interpolation, drops every Linux capability, enables `no-new-privileges`, limits
PIDs to 512, and disables the network by default. `check` mounts the workspace
read-only while keeping only the report directory writable. `fix` explicitly
changes the workspace mount to read-write. Inside the container, the trusted
MegaLinter entrypoint and embedded path-only pre-commands use upstream Bash.

Some configured tools or MegaLinter pre-commands may need dependency downloads.
Such operations fail under `network = "none"`; enabling `bridge` is an explicit
trust decision. Prefer images with dependencies preinstalled for reproducible
protected CI.

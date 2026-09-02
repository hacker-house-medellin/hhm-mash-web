# Rust Lambda route split

Tracking: `DEN-1950`

This directory is an isolated Rust package. It deliberately does not become part of the
long-running web server package, so each infrequent route can be compiled, measured, and
deployed as its own AWS Lambda function.

## Entrypoints

- `hhm-occupancy-report`: MASH/Maud HTML report adapter.
- `hhm-roster-export`: bounded JSON export adapter.

Both binaries import the pinned `hhm-contracts` package from `hhm-lib-core`. Domain and
authorization behavior stays in shared crates. The Lambda adapter owns only HTTP event
validation, bounded request parameters, rendering/serialization, and AWS runtime startup.

The ORM is intentionally not initialized during process startup. A route that needs
`hhm-orm-core` must initialize its pool lazily after authentication and tenant checks, and
must reuse that pool across warm invocations. A dedicated `hhm-lambdas` repository can
replace the pinned Git dependency with workspace paths after the repository-administration
workflow is available.

## Build and package

The Dockerfile has three stages:

1. `build` compiles and strips one selected binary on the target architecture.
2. `artifact` exposes exactly one file named `bootstrap`.
3. `runtime` copies that file into the AWS `provided.al2023` base image.

Examples:

```sh
docker buildx build \
  --platform linux/arm64 \
  --target artifact \
  --build-arg LAMBDA_BIN=hhm-occupancy-report \
  --output type=local,dest=dist \
  src/lambdas

docker buildx build \
  --platform linux/amd64 \
  --build-arg LAMBDA_BIN=hhm-roster-export \
  --tag hhm-roster-export:x86_64 \
  src/lambdas
```

Publish separate architecture-specific image tags. Do not attach both architectures to one
Lambda function image manifest: each Lambda function is configured for one architecture.

The ZIP artifact contains only `bootstrap`. Runtime assets are allowed, but they should be
embedded with `include_str!` or `include_bytes!` when practical. Add external files only
through an explicit allowlist and account for their byte cost in cold-start evidence.

## Security and cold-start gates

- GET only; invalid methods fail closed.
- Request bodies are bounded at 64 KiB.
- `limit` is validated instead of silently clamped.
- Responses disable storage, MIME sniffing, referrers, and unneeded content sources.
- Request bodies, query values, credentials, and tenant data are not logged.
- Release builds use fat LTO, one codegen unit, aborting panics, symbol stripping, and
  size-oriented optimization.
- CI records executable and ZIP byte counts for `arm64` and `x86_64`.
- CI fails if the artifact directory or ZIP contains anything except `bootstrap`.

Before production routing, add authenticated integration tests against the shared query
adapter and record p50/p95 init duration, handler duration, memory, and artifact size.

# Isolated Rust Lambda entrypoints

This nested crate proves that heavy and infrequent routes from `hacker-house-medellin/hhm-mash-web` can be built and deployed independently of the main Mash web server. It does not change the existing server entrypoint, initialize Mash, open a listener, create a WebSocket, or construct the main application/database state.

## Entry points

| Binary | API Gateway route | Required body field | Intended workload |
|---|---|---|---|
| `heavy_application_score` | `POST /api/heavy/application-score` | `application_id` | Score a resource-intensive member application. |
| `heavy_roster_export` | `POST /api/heavy/roster-export` | `cohort_id` | Build a cohort roster export away from the web process. |

Each binary is one AWS Lambda deployment unit:

- `heavy_application_score`
- `heavy_roster_export`

The proof handler accepts API Gateway v1 and v2 request envelopes, requires an exact route and method, rejects base64 bodies, caps the serialized event at 1 MiB, emits structured Lambda tracing fields, and returns `202 Accepted`. Before production traffic, replace the accepted-response seam with a call into a framework-independent domain/service crate; do not link the Mash UI or server startup graph into these binaries.

## Validate

```bash
cargo +1.88.0 test --manifest-path src/lambdas/Cargo.toml --all-targets
cargo +1.88.0 build --manifest-path src/lambdas/Cargo.toml --release --bins
```

CI also type-checks every binary for `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`.

## Build one ZIP per route and architecture

Cargo Lambda emits an AWS custom-runtime executable named `bootstrap` and packages it at the ZIP root:

```bash
# x86_64 / amd64
cargo lambda build \
  --manifest-path src/lambdas/Cargo.toml \
  --release \
  --output-format zip \
  --bin heavy_application_score

# arm64
cargo lambda build \
  --manifest-path src/lambdas/Cargo.toml \
  --release \
  --arm64 \
  --output-format zip \
  --bin heavy_roster_export
```

Repeat the command with the other binary name when publishing its independent function. Configure AWS Lambda with runtime `provided.al2023`, architecture matching the artifact, and API Gateway routing matching the table above.

## Deployment contract

- Give every route its own memory, timeout, reserved concurrency, and ephemeral-storage settings.
- Keep immutable assets embedded when small; store large generated/media data in object storage.
- Keep secrets out of ZIPs and OCI layers. Inject them through the platform or decrypt SOPS ciphertext at runtime.
- Propagate the request/invocation ID into OpenTelemetry so traces correlate with the parent web request and Scintilla control plane.
- Retry only idempotent operations. Use a request/idempotency key before connecting the proof handler to writes.
- Keep the executable entrypoint direct; never route a request-derived command through a shell.

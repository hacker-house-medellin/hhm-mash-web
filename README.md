# hhm-mash-web

**Hacker House Medellín — MASH web server: Maud + Axum + SeaORM + Supabase + HTMX + WebSockets**

Operations and community software for an entrepreneur-focused coliving and coworking house in Medellín, Colombia.

This repository was bootstrapped on 2026-08-04. It is designed as an independently deployable component and as a member of the `hhm-monorepo` workspace.

## GitHub target

`hacker-house-medellin/hhm-mash-web`

## Baseline

- Rust 2024 edition for backend and native components.
- Axum HTTP/WebSocket transport.
- Maud-first HTML rendering with additive, server-only Leptos components.
- Supabase/PostgreSQL configuration through `DATABASE_URL`, `SUPABASE_URL`, and environment-only secrets.
- Structured operational events through the immutable-pinned `ores-otel` logger.
- Docker, Nix, and GitHub Actions entry points.
- Contracts live in `hhm-interfaces`; shared behavior lives in `hhm-libs`.

## Development

```bash
nix develop  # optional
just env-audit
cargo fmt --check
cargo test
```

## Status

Foundation scaffold. Domain behavior, persistence migrations, authentication policy, and production secrets must be reviewed before deployment. Process-local reservation state is not durable. Unauthenticated demo writes require both `ALLOW_UNAUTHENTICATED_DEMO_WRITES=true` and an explicit loopback bind host (`127.0.0.1`, `::1`, or `localhost`); the flag fails closed on every other host.

## Versioned HTML components

`GET /v1/components` returns the `hhm.component.v1` manifest. `GET /v1/components/reservations` returns an escaped, non-executable `text/html` fragment for browser injection or a Flutter WebView. `GET /v1/components/leptos-capabilities` demonstrates the same inert fragment contract with server-side Leptos rendering; it does not hydrate or ship executable code. Successful component and HTML error responses are private and non-cacheable, carry a sandboxed content-security policy, disable MIME sniffing and referrer propagation, and declare their component contract in `x-hhm-component-contract`.

Maud remains the primary document and reservation renderer. Leptos is an additive SSR option for bounded components that benefit from its component model; Axum continues to own routing and security policy, while HTMX and WebSockets remain the browser interaction layer.

The document CSP authorizes its constant inline WebSocket bootstrap with a fresh per-response nonce and does not allow arbitrary inline scripts. `/ws` requires exactly one syntactically valid browser `Origin` whose authority matches the request `Host`; missing, duplicate, malformed, cross-host, and non-HTTP(S) origins are rejected. Incoming WebSocket messages are capped at 16 KiB and individual frames at 8 KiB.

`GET /partials/reservations` remains as the HTMX compatibility route. Form bodies are capped at 16 KiB, fields reject unknown input, text is trimmed and bounded, process-local records are capped, and HTML is generated through Maud's escaping boundary. The external HTMX asset is version-pinned and protected by SHA-384 subresource integrity.

No WASM binary is advertised. A Rust WASM module is not a Flutter component by itself; shipping one requires a versioned ABI, generated Dart binding, content digest and signature, browser and native compatibility policy, and conformance tests. Until that contract exists, the inert HTML/WebView representation is the supported cross-surface format.

## Cross-surface delivery

User-visible, membership, application, room, booking, event, payment, message,
notification, permission, navigation, or deep-link changes in this Rust web
server must be evaluated for:

- `hacker-house-medellin/hhm-flutter` on Android, iOS, Flutter Web/mobile web,
  and Flutter desktop;
- `hacker-house-medellin/hhm-desktop-app.rs`, the planned Rust desktop companion
  with a non-Qt UI and FFI boundary; and
- `hhm-interfaces`, generated clients, membership/room/event/payment schemas,
  route types, offline fixtures, and conformance tests.

This is judgment-based coordination. Public marketing, SEO, and browser-only
administration may remain web-only. Native check-in, printing, local files,
secure storage, notifications, and offline organizer workflows may be
native-specific. Membership/account status, applications, bookings, event
state, payment status, messaging, permissions, errors, and navigation normally
require coordinated updates or an explicit no-change rationale and parity
follow-up.

Deep links are HTTPS-first:

```text
https://<verified-hacker-house-medellin-owned-host>/open/<route>?<bounded-query>
```

with `hhm://` fallback. MASH web, Flutter, and Tauri desktop must share
versioned route types and fixtures and support cold start, already-running
delivery, authentication resume, replay/expiry rejection, and browser fallback.
Payment credentials, access codes, private member data, room-entry secrets,
message contents, provider credentials, and bearer/refresh tokens are
prohibited in URLs. Applications, invitations, bookings, check-in, and payment
handoffs use bounded identifiers or short-lived, single-use, audience-bound
codes and explicit confirmation.

See [`docs/CROSS_SURFACE_DELIVERY.md`](docs/CROSS_SURFACE_DELIVERY.md) and the
[portfolio policy](https://github.com/ORESoftware/project-registry/blob/main/docs/cross-surface-delivery.md).

## Environment secrets

Secrets live in this repo **encrypted** with [sops](https://github.com/getsops/sops) + [age](https://github.com/FiloSottile/age):
`env/enc/<dev|prod>.env.enc` is committed; `just env-use <name>` decrypts it to
`env/dec/<name>.env` (gitignored, mode 0600) and symlinks `./.env` to it. The
Nix dev shell provides the tooling, `just env-audit` runs keyless in CI, and
containers decrypt at `docker run` — never at build. See [`env/README.md`](env/README.md).

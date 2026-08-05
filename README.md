# hhm-mash-web

**Hacker House Medellín — MASH web server: Maud + Axum + SeaORM + Supabase + HTMX + WebSockets**

Operations and community software for an entrepreneur-focused coliving and coworking house in Medellín, Colombia.

This repository was bootstrapped on 2026-08-04. It is designed as an independently deployable component and as a member of the `hhm-monorepo` workspace.

## GitHub target

`hacker-house-medellin/hhm-mash-web`

## Baseline

- Rust 2024 edition for backend and native components.
- Axum HTTP/WebSocket transport.
- Supabase/PostgreSQL configuration through `DATABASE_URL`, `SUPABASE_URL`, and environment-only secrets.
- OpenTelemetry-compatible tracing hooks.
- Docker, Nix, and GitHub Actions entry points.
- Contracts live in `hhm-interfaces`; shared behavior lives in `hhm-libs`.

## Development

```bash
cp .env.example .env 2>/dev/null || true
nix develop  # optional
cargo fmt --check 2>/dev/null || true
cargo test 2>/dev/null || true
```

## Status

Foundation scaffold. Domain behavior, persistence migrations, authentication policy, and production secrets must be reviewed before deployment.

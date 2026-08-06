# Cross-surface delivery

Verified **2026-08-06**.

## Surfaces

- Rust MASH web server: `hacker-house-medellin/hhm-mash-web`
- Flutter Android/iOS, Flutter Web, and Flutter desktop: `hacker-house-medellin/hhm-flutter` — planned
- Rust desktop: `hacker-house-medellin/hhm-desktop.rs` — planned Tauri 2 app
- Shared contracts: `hhm-interfaces`, generated clients, membership/application/room/event/payment schemas, routes, offline fixtures, and conformance tests

## Judgment-based propagation

Evaluate mobile, Flutter Web, Flutter desktop, Rust desktop, and shared contracts for every user-visible or contract-changing web change. Marketing, SEO, and browser-only administration may remain web-only. Native check-in, printing, local files, secure storage, notifications, and offline organizer workflows may be native-specific. Membership/account status, applications, bookings, event state, payment status, messaging, permissions, errors, and navigation normally propagate or require an explicit rationale and parity issue.

## Deep links

```text
https://<verified-hacker-house-medellin-owned-host>/open/<route>?<bounded-query>
hhm://<route>?<bounded-query>
```

The HTTPS host must be verified. All surfaces share versioned routes and fixtures and support cold start, already-running delivery, authentication resume, replay/expiry rejection, browser fallback, and explicit confirmation before applications, invitations, bookings, check-in, payment-adjacent, or destructive actions.

Never put payment credentials, access codes, private member data, room-entry secrets, message contents, provider credentials, or bearer/refresh tokens in URLs. Use bounded identifiers or short-lived, single-use, audience-bound codes and validate route version, member/application/room/event/booking IDs, action, authorization, limits, and user intent.

## Review checklist

- [ ] Flutter Android/iOS impact evaluated.
- [ ] Flutter Web/mobile-web impact evaluated.
- [ ] Flutter desktop impact evaluated.
- [ ] Tauri Rust desktop impact evaluated.
- [ ] Shared membership/client/route/fixture impact evaluated.
- [ ] Deep-link and offline compatibility tested where relevant.
- [ ] Omitted surfaces have a rationale and follow-up when needed.

## Routing

- GitHub Project: [`hacker-house-medellin-project` — Project 1](https://github.com/orgs/hacker-house-medellin/projects/1)
- Linear project: [`github.com/hacker-house-medellin`](https://linear.app/denman/project/githubcomhacker-house-medellin-d4043553c2b4)
- Central policy: [`cross-surface-delivery.md`](https://github.com/ORESoftware/project-registry/blob/main/docs/cross-surface-delivery.md)
- Desktop registry: [`desktop-applications.json`](https://github.com/ORESoftware/project-registry/blob/main/registry/desktop-applications.json)

# hhm-mash-web

Maud + Axum + SeaORM + Supabase/PostgreSQL + HTMX + WebSocket web server for Hacker House Medellín.

**Product:** Hacker House Medellín — Operations software for an entrepreneur coliving and coworking community.

Run rooms, desks, member stays, community events, access workflows, and day-to-day operations for a hacker house in Medellín, Colombia.

## Safety and production boundary

The bootstrap does not implement payments, identity verification, door-control hardware, or Colombian lodging compliance. Add those only after security and local regulatory review.

This repository is an executable bootstrap, not a production deployment. Before live
use, add authentication, tenant authorization, rate limits, durable migrations,
observability, backups, incident response, dependency review, and secret management.
## Stack

Maud renders escaped server-side HTML, Axum serves HTTP/WebSockets, HTMX handles
progressive updates, SeaORM connects to Supabase-compatible PostgreSQL, and the
browser refreshes fragments after WebSocket notifications.

`DATABASE_URL` and `SUPABASE_URL` are optional for the in-memory bootstrap. Never
expose a Supabase service-role key to browser code.

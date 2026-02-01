# Phase 1 Implementation — Pulse Analytics Core Service

## Overview

Pulse Analytics is a self-hosted, multi-tenant analytics microservice built in Rust (Axum). It provides a unified API that wraps Umami pageview data and custom event tracking, with both a drop-in script tag and a typed TypeScript SDK for client integration.

This document covers the Phase 1 implementation: the core service, database schema, ingestion pipeline, query API, admin endpoints, client SDK, and deployment configuration.

## Architecture Decisions

### Why Rust + Axum?
- Existing Rust patterns in the portfolio engine service
- Sub-millisecond response times for the ingestion endpoint (critical for analytics)
- Low memory footprint for a long-running service
- Strong type system prevents common runtime errors in data processing

### Why a Standalone Repo?
- Independent deployment lifecycle — analytics shouldn't block portfolio deploys
- Reusable across all projects (Quantum, Aura, Tapdue, etc.)
- Clean separation of concerns

### Multi-Tenant via API Keys (not separate databases)
- All projects share one PostgreSQL database with `project_id` filtering on every table
- Simpler than database-per-tenant, sufficient for single-operator self-hosted use
- API keys with SHA-256 hashing and scope-based access control (ingest/query/admin)

### Redis Buffering for Ingestion
- Events are pushed to Redis lists immediately, flushed to PostgreSQL every 5 seconds
- This absorbs traffic spikes without overloading PostgreSQL with single-row inserts
- Batch INSERT for high throughput during flush
- Redis also handles session resolution (30-min TTL hashes), real-time visitor tracking (sorted sets), rate limiting, and API key caching

### Partitioned Tables for Pageviews/Events
- Monthly partitions on `created_at` for efficient range queries and data retention
- Indexes on `(project_id, created_at)` and `(project_id, path/event_name, created_at)`
- Pre-computed daily rollup tables for fast dashboard queries

## Technical Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Web framework | Axum 0.8 | Native async, tower middleware ecosystem |
| Database | sqlx 0.8 (runtime queries) | No compile-time DB required, still type-safe |
| Redis | redis 0.27 | ConnectionManager for pooling |
| UA parsing | woothee 0.13 | Lightweight, Rust-native |
| GeoIP | maxminddb 0.24 | In-memory reader, MaxMind GeoLite2 |
| Hashing | sha2 0.10 + hex 0.4 | API key hashing |
| SDK bundler | tsup 8.x | Outputs CJS, ESM, IIFE, and .d.ts |

## Cost Analysis

- **GeoIP**: MaxMind GeoLite2 is free (requires registration, updated biweekly)
- **Hosting**: Runs on existing VPS (72.62.82.57) via Coolify — no additional cost
- **Database**: Uses existing PostgreSQL server (new `pulse_analytics` database)
- **Redis**: Uses existing Redis server (new ACL user `pulse_analytics_user`)
- **Domain**: `pulse.ayushojha.com` — DNS record on existing domain

## File Inventory

### Rust Service (`crates/pulse-server/`)

| File | Description |
|------|-------------|
| `src/main.rs` | Entry point: DB/Redis init, migration runner, route assembly, server start |
| `src/config.rs` | Environment-based configuration struct |
| `src/state.rs` | `AppState` with PgPool, Redis ConnectionManager, GeoIP reader |
| `src/error.rs` | `AppError` enum with `IntoResponse` for consistent JSON error responses |
| `src/models/project.rs` | Project, ApiKeyRow, ResolvedKey, CreateProject, CreateApiKey structs |
| `src/models/session.rs` | Session and SessionCache structs |
| `src/models/pageview.rs` | BufferedPageview for Redis/batch pipeline |
| `src/models/event.rs` | BufferedEvent for Redis/batch pipeline |
| `src/routes/ingest.rs` | `POST /api/collect` — unified ingestion endpoint |
| `src/routes/query.rs` | `GET /api/v1/stats,pages,referrers,events,devices,geo,realtime` |
| `src/routes/admin.rs` | Project CRUD, API key generation/revocation |
| `src/routes/health.rs` | Health check (DB + Redis connectivity) |
| `src/routes/script.rs` | `GET /api/script.js` — serves the tracking script |
| `src/middleware/auth.rs` | API key validation (Redis-cached) + admin Bearer token auth |
| `src/middleware/rate_limit.rs` | Redis sliding window rate limiter |
| `src/middleware/cors.rs` | Dynamic CORS based on config |
| `src/services/ingestion.rs` | Redis buffer push + background flush task |
| `src/services/session.rs` | Session resolution and counter updates |
| `src/services/ua.rs` | User-Agent parsing via woothee |
| `src/services/geo.rs` | IP → country/region/city via MaxMind |

### Shared Types (`crates/pulse-common/`)

| File | Description |
|------|-------------|
| `src/types.rs` | `CollectEnvelope`, `CollectRequest` (Pageview/Event/Identify), payload structs |

### Database Migrations (`migrations/`)

| File | Description |
|------|-------------|
| `001_initial_schema.up.sql` | projects, api_keys, sessions, pageviews (partitioned), events (partitioned) |
| `002_rollup_tables.up.sql` | daily_stats, daily_pages, daily_referrers, daily_events, daily_geo, daily_devices |

### TypeScript SDK (`sdk/`)

| File | Description |
|------|-------------|
| `src/client.ts` | `PulseClient` class with auto-tracking, event/pageview/identify, query methods |
| `src/server.ts` | `PulseServerClient` for Node.js backends (no auto-tracking) |
| `src/auto.ts` | Lightweight IIFE script for `<script>` tag integration (<4KB gzipped) |
| `src/types.ts` | TypeScript interfaces for all API contracts |
| `src/index.ts` | NPM entry point re-exports |

### Deployment

| File | Description |
|------|-------------|
| `Dockerfile` | Multi-stage Rust build (builder → slim Debian runtime) |
| `docker-compose.yml` | Local dev with PostgreSQL + Redis containers |
| `docker-compose.coolify.yml` | Production deployment with Traefik labels |

## Database Schema

### Core Tables
- **projects** — tenant registry with optional Umami linking
- **api_keys** — SHA-256 hashed keys with scopes (`ingest`, `query`, `admin`)
- **sessions** — visitor sessions with device/geo metadata, bounce tracking
- **pageviews** — partitioned monthly, append-only
- **events** — partitioned monthly, append-only, JSONB payload

### Rollup Tables (pre-computed daily aggregates)
- **daily_stats** — pageviews, visitors, sessions, bounces, duration
- **daily_pages** — per-path view counts
- **daily_referrers** — per-domain visit counts
- **daily_events** — per-event-name counts
- **daily_geo** — per-country visitor counts
- **daily_devices** — browser/OS/device breakdown

## API Documentation

### Ingestion
- `POST /api/collect` — Single endpoint. Body: `{ type: "pageview"|"event"|"identify", payload: {...}, visitor_id: "..." }`. Auth: `X-Pulse-Key` header or `?key=` query param.

### Query (requires `query` scope)
- `GET /api/v1/stats?start_at=&end_at=` — Overview with previous-period comparison
- `GET /api/v1/pages?start_at=&end_at=&limit=&offset=` — Top pages
- `GET /api/v1/referrers` — Top referrers
- `GET /api/v1/events` — Event breakdown
- `GET /api/v1/devices` — Browser/OS/device
- `GET /api/v1/geo` — Country breakdown
- `GET /api/v1/realtime` — Active visitors count (last 5 minutes)

### Admin (requires `Authorization: Bearer <PULSE_ADMIN_TOKEN>`)
- `POST /api/admin/projects` — Create project
- `GET /api/admin/projects` — List projects
- `GET /api/admin/projects/{id}` — Get project
- `POST /api/admin/projects/{id}/keys` — Generate API key (returns full key once)
- `GET /api/admin/projects/{id}/keys` — List keys
- `DELETE /api/admin/projects/{id}/keys/{key_id}` — Revoke key

### Public
- `GET /health` — Service health (DB + Redis)
- `GET /api/script.js` — Tracking script (cached 24h)

## Setup Instructions

### Local Development (Docker)
```bash
cd pulse-analytics
cp .env.example .env
docker compose up
```
This starts PostgreSQL (port 5434) and Redis (port 6381) locally. The service runs on port 8090.

### Local Development (Native)
```bash
# Start SSH tunnels to VPS
ssh -L 5433:127.0.0.1:5433 ayush@72.62.82.57 -N &
ssh -L 6380:127.0.0.1:6380 ayush@72.62.82.57 -N &

# Create database
PGPASSWORD='i87RfJUBx5HZJuykZt4v9u3zaq10wAqV' psql -h localhost -p 5433 -U admin -d postgres -c "CREATE DATABASE pulse_analytics;"

# Configure .env
cp .env.example .env
# Edit .env with VPS connection details

# Run
cargo run -p pulse-server
```

### Production Deployment (Coolify)
1. Create `pulse_analytics` database on VPS PostgreSQL
2. Add `pulse_analytics_user` Redis ACL entry
3. Point `pulse.ayushojha.com` DNS to VPS
4. Deploy via Coolify using `docker-compose.coolify.yml`

### SDK Usage
```html
<!-- Script tag (any website) -->
<script src="https://pulse.ayushojha.com/api/script.js"
        data-key="pa_live_xxx" defer></script>
```

```typescript
// NPM SDK
import { createPulse } from '@ayushojha/pulse-analytics';
const pulse = createPulse({ apiKey: 'pa_live_xxx' });
pulse.event('signup_click', { source: 'header' });
```

```typescript
// Server-side
import { PulseServerClient } from '@ayushojha/pulse-analytics/server';
const pulse = new PulseServerClient({ apiKey: 'pa_live_xxx' });
await pulse.trackEvent({ visitorId: 'user123', eventName: 'purchase', data: { amount: 99 } });
```

## What's NOT Included (Deferred)

- **Umami proxy/merge** — Endpoints exist but Umami client integration is Phase 2
- **Daily rollup background task** — Schema ready, computation logic is Phase 2
- **Data retention automation** — Partition cleanup is Phase 5
- **Built-in dashboard UI** — API-first for now, UI is Phase 5
- **Webhook/alert notifications** — Phase 5

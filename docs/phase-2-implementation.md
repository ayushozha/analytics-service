# Phase 2 Implementation — Umami Proxy, Rollups, and Unified Queries

## Overview

Phase 2 adds three capabilities to Pulse Analytics:

1. **Umami HTTP Client** — Authenticated proxy to the self-hosted Umami instance, with JWT caching
2. **Daily Rollup Background Task** — Pre-computes aggregated stats from raw tables into rollup tables daily at 00:05 UTC
3. **Hybrid Query Engine** — Query endpoints now combine rollup tables (completed days) + raw tables (today's partial data) + Umami data (when configured)

## Architecture Decisions

### Umami Proxy vs. Direct Replacement

Rather than replacing Umami, Pulse wraps it. Projects that already use Umami can set `umami_website_id` on their Pulse project and get unified data. New projects that only use Pulse get the same query API — the Umami layer is transparent and optional.

The proxy authenticates to Umami via `POST /api/auth/login` and caches the JWT in Redis for 55 minutes (Umami tokens expire after 60 minutes). This avoids re-authenticating on every request.

### Hybrid Query Strategy

Dashboard queries use a two-tier approach:

```
Completed days  → daily_stats / daily_pages / ... rollup tables (fast, pre-computed)
Today (partial) → raw pageviews / events / sessions tables (real-time, query-time aggregation)
Umami data      → merged additively when project has umami_website_id configured
```

This gives sub-100ms response times for large date ranges while keeping today's data fresh (5-second buffer delay from ingestion).

### Rollup Timing

The rollup task runs at startup for yesterday (catches missed runs) and then schedules at 00:05 UTC daily. This 5-minute offset ensures the previous day's data has fully flushed from Redis buffers. Rollups are idempotent — they use DELETE + INSERT (for tables with composite PKs that include variable fields like `path`) or ON CONFLICT DO UPDATE (for tables with fixed PKs).

## Technical Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Umami auth | JWT cached in Redis (55min TTL) | Avoids per-request auth overhead |
| HTTP client | reqwest with 10s timeout | Prevents slow Umami responses from blocking queries |
| Rollup schedule | Tokio sleep loop | No external cron dependency, runs in-process |
| Today's data | Query-time aggregation | Avoids rollup staleness, acceptable for single-day range |

## File Inventory

### New Files

| File | Description |
|------|-------------|
| `src/services/umami_client.rs` | Umami HTTP client with JWT caching, methods for stats/pages/referrers/countries/browsers/os/events/active-visitors |
| `src/services/aggregation.rs` | Daily rollup background task computing 6 rollup tables from raw data |

### Modified Files

| File | Changes |
|------|---------|
| `src/state.rs` | Added `umami: Option<UmamiClient>` to `AppState` |
| `src/services/mod.rs` | Added `pub mod aggregation; pub mod umami_client;` |
| `src/main.rs` | Initialize `UmamiClient` from env vars, start rollup task, add `/api/v1/stats/timeseries` route |
| `src/routes/query.rs` | Complete rewrite: hybrid rollup+raw queries, Umami data merging, new `get_timeseries` endpoint |

## API Changes

### New Endpoint

- `GET /api/v1/stats/timeseries?start_at=&end_at=&unit=day` — Time-bucketed pageviews/visitors/sessions data. Returns rollup data for completed days + raw data for today.

### Enhanced Endpoints (now with Umami merge)

All query endpoints now:
1. Read rollup tables for completed days (`date < today`)
2. Query raw tables for today's partial data (`created_at >= today`)
3. Merge Umami data when the project has `umami_website_id` set

The Umami data appears in responses as additional fields:
- `GET /api/v1/stats` → adds `umami` object with Umami's raw metrics
- `GET /api/v1/pages` → merges Umami page views additively (includes `umami_views` field)
- `GET /api/v1/referrers` → merges Umami referrer visits
- `GET /api/v1/geo` → merges Umami country visitors
- `GET /api/v1/realtime` → adds `umami_active_visitors` and `total_active_visitors`

## Umami Client API

The `UmamiClient` wraps the following Umami API endpoints:

| Method | Umami Endpoint | Description |
|--------|---------------|-------------|
| `get_stats()` | `GET /api/websites/{id}/stats` | Pageviews, visitors, visits, bounces, totaltime |
| `get_pageviews()` | `GET /api/websites/{id}/metrics?type=url` | Top pages |
| `get_referrers()` | `GET /api/websites/{id}/metrics?type=referrer` | Top referrers |
| `get_browsers()` | `GET /api/websites/{id}/metrics?type=browser` | Browser breakdown |
| `get_os()` | `GET /api/websites/{id}/metrics?type=os` | OS breakdown |
| `get_countries()` | `GET /api/websites/{id}/metrics?type=country` | Country breakdown |
| `get_events()` | `GET /api/websites/{id}/metrics?type=event` | Event breakdown |
| `get_active_visitors()` | `GET /api/websites/{id}/active` | Real-time active count |

## Rollup Tables Computed

| Table | Source | Aggregation |
|-------|--------|-------------|
| `daily_stats` | pageviews + sessions | SUM pageviews, COUNT DISTINCT visitors, COUNT DISTINCT sessions, bounce count, total duration |
| `daily_pages` | pageviews | Per-path: COUNT views, COUNT DISTINCT unique views, AVG duration |
| `daily_referrers` | pageviews | Per-referrer-domain: COUNT DISTINCT sessions |
| `daily_events` | events | Per-event-name: COUNT |
| `daily_geo` | sessions | Per-country: COUNT DISTINCT visitors |
| `daily_devices` | sessions | Per-browser/os/device: COUNT DISTINCT visitors |

## Configuration

Phase 2 uses these environment variables (all optional — Umami proxy is disabled if not set):

```env
UMAMI_URL=https://analytics.ayushojha.com
UMAMI_USER=admin
UMAMI_PASS=your-umami-password
```

## What's NOT Included (Deferred)

- **Umami event merging in `/api/v1/events`** — Currently only Pulse custom events. Could merge Umami events in a future iteration.
- **Partition management** — Auto-creating future monthly partitions (Phase 5)
- **Data retention cleanup** — Dropping old partitions based on `retention_days` setting (Phase 5)
- **Admin rollup trigger endpoint** — `trigger_rollup()` function exists but no route yet

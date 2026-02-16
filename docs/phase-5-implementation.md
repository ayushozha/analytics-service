# Phase 5: Polish — Implementation Documentation

## Overview

Phase 5 adds five features to make Pulse Analytics production-ready and self-sufficient:

1. **Partition Management** — Automatic creation of future monthly table partitions
2. **Data Retention** — Configurable automated cleanup of old raw data
3. **Webhook Alerts** — Traffic spike, zero traffic, and daily summary notifications
4. **Built-in Dashboard** — HTMX + Tailwind analytics dashboard at `/dashboard`
5. **Project Onboarding** — Interactive CLI script to onboard new projects

## Architecture Decisions

### Partition Management as a Background Task
PostgreSQL range-partitioned tables (`pageviews`, `events`) require partitions to exist before data can be inserted. The initial migration hardcoded Jan–Jun 2026. The partition service queries `pg_inherits` to discover existing partitions and creates any missing ones through 3 months ahead, running at startup and monthly thereafter.

### Data Retention via Partition Drops
Rather than running expensive `DELETE` queries on large partitioned tables, the retention service drops entire partitions whose date range is fully past the cutoff. This is an O(1) metadata operation in PostgreSQL. Only the boundary partition (partially overlapping the cutoff) needs row-level deletes.

### HMAC-Signed Cookies over `SignedCookieJar`
The axum-extra `SignedCookieJar` requires `Key` to be extractable from the app state via `FromRef`. Since the app uses `Extension<SharedState>` (not `State<AppState>`), integrating `SignedCookieJar` would require restructuring all existing routes. Instead, the dashboard uses a regular `CookieJar` with manual HMAC-SHA256 signing of the cookie value, achieving the same tamper-proof session cookie with minimal code changes.

### HTMX + Tailwind over React SPA
The dashboard uses server-rendered Askama templates with HTMX for interactivity. This keeps everything in Rust with no separate frontend build, produces fast server-rendered HTML, and leverages Chart.js via CDN for visualizations. HTMX handles dynamic date range updates and auto-refreshing realtime data.

### Shared Query Service Layer
Extracted all SQL query logic from `routes/query.rs` into `services/query.rs`. Both the JSON API endpoints and the dashboard HTMX partials call the same functions, eliminating code duplication and ensuring consistent data between the API and dashboard.

### Interactive Onboarding over Batch
The onboarding script creates one project at a time with interactive prompts, outputting ready-to-paste integration snippets. This lets users onboard projects incrementally without needing to configure everything at once.

## Technical Choices

### Askama Templates (v0.12)
Compile-time Jinja2-like templates. Catches template errors at compile time, zero runtime overhead, and integrates with Axum via `askama_axum`.

### Webhook Baseline Spike Detection
Traffic spikes are detected by comparing the current 10-minute pageview rate (extrapolated to hourly) against a rolling 14-day average stored in `webhook_baselines`. The baselines are segmented by hour-of-day and day-of-week to account for natural traffic patterns.

### Webhook HMAC Signatures
When a webhook has a `secret` configured, the payload is signed with HMAC-SHA256 and the signature is sent in the `X-Pulse-Signature` header. Receivers can verify authenticity by computing the same HMAC.

## Database Schema

### New Migration: `003_webhooks.up.sql`

```sql
-- Webhook subscriptions
webhooks (
    id UUID PK,
    project_id UUID FK -> projects,
    url TEXT,
    events TEXT[],           -- "traffic_spike", "zero_traffic", "daily_summary"
    secret VARCHAR(64),      -- HMAC signing secret (optional)
    is_active BOOLEAN,
    last_triggered_at TIMESTAMPTZ,
    created_at, updated_at
)

-- Baseline hourly traffic for spike detection
webhook_baselines (
    project_id UUID FK,
    hour_of_day SMALLINT,    -- 0-23
    day_of_week SMALLINT,    -- 0-6 (Mon-Sun)
    avg_pageviews DOUBLE PRECISION,
    updated_at TIMESTAMPTZ,
    PK (project_id, hour_of_day, day_of_week)
)
```

## API Additions

### Webhook Admin Endpoints (Bearer token auth)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/admin/projects/{id}/webhooks` | Create webhook |
| GET | `/api/admin/projects/{id}/webhooks` | List webhooks |
| PUT | `/api/admin/projects/{id}/webhooks/{wid}` | Update webhook |
| DELETE | `/api/admin/projects/{id}/webhooks/{wid}` | Delete webhook |
| POST | `/api/admin/projects/{id}/webhooks/{wid}/test` | Test fire webhook |

### Dashboard Routes (cookie auth)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/dashboard` | Redirect to login or overview |
| GET/POST | `/dashboard/login` | Login with API key |
| POST | `/dashboard/logout` | Clear session |
| GET | `/dashboard/overview` | Stats + timeseries chart |
| GET | `/dashboard/pages` | Top pages table |
| GET | `/dashboard/referrers` | Referrer breakdown |
| GET | `/dashboard/events` | Custom events |
| GET | `/dashboard/devices` | Browser/OS/device |
| GET | `/dashboard/geo` | Country breakdown |
| GET | `/dashboard/realtime` | Live visitor count |

HTMX partial endpoints at `/dashboard/api/*` return HTML fragments for dynamic updates.

## File Inventory

### New Files

| File | Description |
|------|-------------|
| `crates/pulse-server/src/services/partition.rs` | Partition management background task |
| `crates/pulse-server/src/services/retention.rs` | Data retention cleanup task |
| `crates/pulse-server/src/services/webhook.rs` | Webhook alert tasks (spike, zero traffic, daily summary) |
| `crates/pulse-server/src/services/query.rs` | Shared query logic for API + dashboard |
| `crates/pulse-server/src/routes/dashboard.rs` | Dashboard route handlers + HTMX partials |
| `crates/pulse-server/src/models/webhook.rs` | Webhook model structs |
| `migrations/003_webhooks.up.sql` | Webhooks + baselines tables |
| `migrations/003_webhooks.down.sql` | Drop webhooks tables |
| `crates/pulse-server/templates/base.html` | Shared HTML layout |
| `crates/pulse-server/templates/dashboard/login.html` | Login page |
| `crates/pulse-server/templates/dashboard/overview.html` | Overview with HTMX targets |
| `crates/pulse-server/templates/dashboard/pages.html` | Pages table page |
| `crates/pulse-server/templates/dashboard/referrers.html` | Referrers table page |
| `crates/pulse-server/templates/dashboard/events.html` | Events table page |
| `crates/pulse-server/templates/dashboard/devices.html` | Devices table page |
| `crates/pulse-server/templates/dashboard/geo.html` | Geography table page |
| `crates/pulse-server/templates/dashboard/realtime.html` | Realtime visitor page |
| `crates/pulse-server/templates/partials/nav.html` | Sidebar navigation |
| `crates/pulse-server/templates/partials/date_picker.html` | Date range selector + JS |
| `crates/pulse-server/templates/partials/stats_cards.html` | Metric cards partial |
| `crates/pulse-server/templates/partials/timeseries.html` | Chart.js timeseries partial |
| `scripts/onboard-project.sh` | Interactive project onboarding CLI |

### Modified Files

| File | Change |
|------|--------|
| `crates/pulse-server/Cargo.toml` | Added askama, askama_axum, axum-extra, hmac |
| `crates/pulse-server/src/main.rs` | Registered dashboard routes, started partition/retention/webhook tasks |
| `crates/pulse-server/src/config.rs` | Added `data_retention_days`, `cookie_secret` fields |
| `crates/pulse-server/src/routes/mod.rs` | Added `pub mod dashboard;` |
| `crates/pulse-server/src/routes/query.rs` | Refactored to call `services::query::*` functions |
| `crates/pulse-server/src/routes/admin.rs` | Added webhook CRUD + test endpoints |
| `crates/pulse-server/src/services/mod.rs` | Added partition, query, retention, webhook modules |
| `crates/pulse-server/src/models/mod.rs` | Added `pub mod webhook;` |

## New Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| askama | 0.12 | Compile-time HTML templates |
| askama_axum | 0.4 | Askama integration with Axum |
| axum-extra | 0.10 | Cookie jar for dashboard sessions |
| hmac | 0.12 | HMAC-SHA256 for webhook signatures + cookie signing |

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `DATA_RETENTION_DAYS` | 365 | Days to keep raw data (0 = disabled) |
| `PULSE_COOKIE_SECRET` | random | Secret for HMAC-signing dashboard session cookies |

## Background Tasks

| Task | Schedule | Description |
|------|----------|-------------|
| Partition manager | Startup + 1st of each month at 01:00 UTC | Creates partitions 3 months ahead |
| Retention cleanup | Daily at 01:00 UTC | Drops expired partitions, cleans old sessions |
| Traffic spike checker | Every 10 minutes | Compares current rate vs baseline |
| Zero traffic checker | Every hour | Fires if no pageviews for 3+ hours |
| Daily summary | Daily at 00:30 UTC | Sends yesterday's stats digest |
| Baseline updater | Daily at 00:30 UTC (after summary) | Updates 14-day rolling averages |

## Setup Instructions

### Dashboard Access
1. Create an API key with `query` scope for a project
2. Navigate to `https://pulse.ayushojha.com/dashboard`
3. Enter the query API key to log in
4. Dashboard session lasts 7 days (cookie-based)

### Onboarding a New Project
```bash
export PULSE_URL=https://pulse.ayushojha.com
export PULSE_ADMIN_TOKEN=your-admin-token
./scripts/onboard-project.sh
# Follow the prompts, save the output credentials

# Optional: also create Redis ACL user on VPS
./scripts/onboard-project.sh --with-redis-acl
```

### Webhook Setup
```bash
# Create a webhook for traffic spike alerts
curl -X POST https://pulse.ayushojha.com/api/admin/projects/{id}/webhooks \
  -H "Authorization: Bearer $PULSE_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"url":"https://hooks.slack.com/...", "events":["traffic_spike","daily_summary"], "secret":"optional-signing-secret"}'

# Test it
curl -X POST https://pulse.ayushojha.com/api/admin/projects/{id}/webhooks/{wid}/test \
  -H "Authorization: Bearer $PULSE_ADMIN_TOKEN"
```

## What's NOT Included (addressed in Phase 6 or deferred)

- **User-level visitor tracking and drill-down**: Implemented in Phase 6
- **Pricing page analytics**: Implemented in Phase 6
- **Rich visualizations (heatmaps, funnels, doughnut charts)**: Implemented in Phase 6
- **Webhook retry/dead-letter queue**: Failed webhooks are logged but not retried
- **Dashboard dark mode**: Uses light theme only
- **Custom date range input**: Dashboard supports preset ranges (Today, 7d, 30d, 90d) but not custom date pickers
- **Dashboard multi-project switching**: Each login is scoped to one project; switch by logging out and back in with a different key
- **Automated partition cleanup for rollup tables**: Rollup tables are small and kept indefinitely

# Phase 6: User Intelligence & Pricing Analytics — Implementation Documentation

## Overview

Phase 6 extends the Pulse dashboard with three new pages focused on individual user behavior and pricing page performance:

1. **Visitors Page** — Live visitor count, real-time activity feed, searchable visitor table with drill-down
2. **Visitor Detail Page** — Per-visitor summary cards, daily activity bar chart, event breakdown doughnut chart, expandable session timeline with inline page/event details
3. **Pricing Analytics Page** — Pricing-specific stats, gradient area timeseries, visit frequency distribution, referrer table, hour-by-day heatmap, and conversion funnel

All features are built on existing data — no schema migrations were needed. The `sessions`, `pageviews`, `events`, and `daily_pages` tables already contain visitor IDs, page paths, and timestamps. Existing indexes (`idx_sessions_visitor`, `idx_pageviews_path`) support the new query patterns.

## Architecture Decisions

### No New Migrations

Every query operates on existing tables. Visitor-level queries use `sessions` (grouped by `visitor_id`) and raw `pageviews`/`events`. Pricing queries filter `daily_pages` and `pageviews` by `path LIKE '%/pricing%' OR path LIKE '%/plans%'`. The existing `idx_sessions_visitor(project_id, visitor_id)` index serves per-visitor lookups efficiently.

### Server-Rendered Heatmap and Funnel

Chart.js does not have a native heatmap type. Rather than adding a third-party Chart.js plugin (which would require a CDN dependency), the heatmap is rendered as a server-side CSS grid with inline `rgba()` opacity values scaled against the maximum. The conversion funnel is also server-rendered as gradient progress bars. This keeps the zero-build-tool approach consistent with the rest of the dashboard.

### Hybrid Rollup+Raw Pattern for Pricing Queries

`fetch_pricing_stats` and `fetch_pricing_timeseries` follow the same two-tier pattern established in Phase 2 — reading from `daily_pages` for completed days and querying raw `pageviews` for today. This gives fast historical queries while keeping today's data fresh.

### Visitor Queries Hit Raw Tables Only

Unlike the aggregate dashboard pages, visitor-level queries do not have pre-computed rollup tables. They query `sessions`, `pageviews`, and `events` directly, scoped by `visitor_id` and date range. This is acceptable because:
- Queries are always scoped to a single visitor (narrow result set)
- The `idx_sessions_visitor` index makes lookups fast
- Partition pruning on `created_at` limits the scan to relevant months

If visitor list queries become slow on very large datasets, a `daily_visitors` rollup table could be added in a future migration.

### Funnel: Simplified Path Matching

The funnel counts distinct visitors who visited pages matching each step pattern (`path LIKE '/pricing%'`), executed as separate queries per step. This is a "who visited each page" funnel, not a strict sequential funnel (which would require ordered window functions). The simplified approach is sufficient for most conversion analysis and avoids complex query construction.

## Technical Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Visitor activity chart | Chart.js bar chart with gradient fill | Matches existing Chart.js usage, gradient adds visual polish |
| Event breakdown | Chart.js doughnut chart (65% cutout) | Clear proportional view with legend, no extra library |
| Pricing timeseries | Chart.js line chart with 3-stop gradient fill | Smooth area chart effect, dashed line for unique visitors |
| Visit frequency | Chart.js horizontal bar with intensity-scaled opacity | Shows distribution clearly, opacity indicates relative count |
| Heatmap | Server-rendered CSS grid with inline rgba | No extra JS library needed, works with HTMX partial swap |
| Funnel | Server-rendered gradient progress bars | Visual drop-off indicators, no chart library needed |
| Session detail | Nested HTMX (hx-get on click, once) | Lazy loads only when expanded, reduces initial payload |

## Database Queries Added

### Visitor Queries (in `services/query.rs`)

| Function | Query Pattern | Key Tables/Indexes |
|----------|--------------|-------------------|
| `fetch_visitors_list` | GROUP BY visitor_id on sessions, paginated, optional search | sessions + idx_sessions_visitor |
| `fetch_recent_activity` | UNION ALL pageviews + events from last hour, ORDER BY created_at | pageviews, events (partition-pruned) |
| `fetch_visitor_summary` | Aggregate sessions + count pricing pageviews for one visitor | sessions + pageviews |
| `fetch_visitor_sessions` | List sessions for one visitor, ordered by first_at | sessions + idx_sessions_visitor |
| `fetch_session_detail` | Pageviews + events for one session_id, ordered by created_at | pageviews, events |
| `fetch_visitor_daily_activity` | GROUP BY date on pageviews for one visitor | pageviews |
| `fetch_visitor_event_breakdown` | GROUP BY event_name on events for one visitor | events |

### Pricing Queries (in `services/query.rs`)

| Function | Query Pattern | Key Tables/Indexes |
|----------|--------------|-------------------|
| `fetch_pricing_stats` | Hybrid rollup+raw, filtered by path LIKE pricing/plans | daily_pages + pageviews + sessions |
| `fetch_pricing_timeseries` | Hybrid rollup+raw, GROUP BY date, filtered by path | daily_pages + pageviews |
| `fetch_pricing_frequency` | Subquery: per-visitor visit count, then bucket into 1x–5x+ | pageviews |
| `fetch_pricing_referrers` | GROUP BY referrer_domain, filtered by pricing paths | pageviews |
| `fetch_pricing_heatmap` | EXTRACT(DOW/HOUR), GROUP BY day+hour, filtered by pricing paths | pageviews |
| `fetch_funnel` | Per-step COUNT(DISTINCT visitor_id) WHERE path LIKE step% | pageviews |

## API Additions

### Dashboard Routes (cookie auth)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/dashboard/visitors` | Visitors list page |
| GET | `/dashboard/visitors/{visitor_id}` | Visitor detail page |
| GET | `/dashboard/pricing` | Pricing analytics page |

### HTMX Partial Endpoints (cookie auth, return HTML fragments)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/dashboard/api/visitors/live-count` | Live visitor count badge (polls every 5s) |
| GET | `/dashboard/api/visitors/activity-feed` | Recent activity feed (polls every 10s) |
| GET | `/dashboard/api/visitors/table` | Searchable visitor table (date range + search param) |
| GET | `/dashboard/api/visitor/{id}/summary` | 5 stat cards (sessions, pageviews, events, duration, pricing views) |
| GET | `/dashboard/api/visitor/{id}/sessions` | Expandable session timeline |
| GET | `/dashboard/api/visitor/{id}/session/{sid}/detail` | Inline session page/event list |
| GET | `/dashboard/api/visitor/{id}/activity-chart` | Chart.js gradient bar chart |
| GET | `/dashboard/api/visitor/{id}/events-breakdown` | Chart.js doughnut chart |
| GET | `/dashboard/api/pricing/stats` | 4 stat cards (views, unique visitors, avg time, bounce rate) |
| GET | `/dashboard/api/pricing/timeseries` | Chart.js gradient area chart |
| GET | `/dashboard/api/pricing/frequency` | Chart.js horizontal bar chart (1x–5x+ buckets) |
| GET | `/dashboard/api/pricing/referrers` | Referrer table for pricing pages |
| GET | `/dashboard/api/pricing/heatmap` | 7x24 hour/day CSS grid heatmap |
| GET | `/dashboard/api/pricing/funnel` | Conversion funnel with drop-off indicators |

## File Inventory

### New Files

| File | Description |
|------|-------------|
| `crates/pulse-server/templates/dashboard/visitors.html` | Visitors list page — live badge, activity feed, search input, visitor table |
| `crates/pulse-server/templates/dashboard/visitor_detail.html` | Visitor detail — summary cards, charts, session timeline |
| `crates/pulse-server/templates/dashboard/pricing.html` | Pricing analytics — stats, timeseries, frequency, referrers, heatmap, funnel |

### Modified Files

| File | Changes |
|------|---------|
| `crates/pulse-server/src/services/query.rs` | Added 13 query functions (7 visitor, 6 pricing) |
| `crates/pulse-server/src/routes/dashboard.rs` | Added 3 template structs, 2 param structs, 3 page handlers, 14 HTMX partial handlers (~1000 lines) |
| `crates/pulse-server/src/main.rs` | Registered 17 new routes in `dashboard_routes` |
| `crates/pulse-server/templates/partials/nav.html` | Added "User Intelligence" section with Visitors and Pricing nav links |

## Visualization Specifications

### Chart.js Visualizations (inline scripts in HTMX partials)

| Chart | Type | Key Config |
|-------|------|-----------|
| Visitor daily activity | `bar` | Gradient fill (indigo→violet), borderRadius: 6, easeOutQuart animation |
| Visitor event breakdown | `doughnut` | 65% cutout, 10-shade indigo/violet palette, right legend with point styles |
| Pricing timeseries | `line` | 3-stop gradient fill, 0.4 tension, hidden points, dashed unique visitors line |
| Pricing visit frequency | `bar` (horizontal) | Intensity-scaled rgba opacity per bar, axis titles |

### Server-Rendered Visualizations (HTML in Rust handlers)

| Visualization | Technique |
|---------------|-----------|
| Activity heatmap | CSS grid (40px label col + 24×24px cells), inline `rgba(99,102,241, opacity)`, legend strip |
| Conversion funnel | Gradient progress bars (`linear-gradient(90deg, #6366f1, #8b5cf6)`), width by %, red drop-off indicators |
| Live count badge | Inline flex with animated green dot + count |
| Activity feed | Scrollable divided list with type-colored icons |

## Navigation Structure

The sidebar now has two sections:

**Main Navigation** (existing):
- Overview, Pages, Referrers, Events, Devices, Geography, Realtime

**User Intelligence** (new, separated by a border divider):
- Visitors (users group icon)
- Pricing (dollar circle icon)

The visitor detail page (`/dashboard/visitors/{id}`) reuses `active_page: "visitors"` so the nav highlight stays on "Visitors" during drill-down.

## What's NOT Included

- **Strict sequential funnel**: Current funnel counts visitors per page independently; a sequential funnel (A→B→C in order) would require window functions
- **Configurable pricing paths**: Currently hardcoded to `%/pricing%` and `%/plans%`; could be made configurable via project settings JSON
- **Daily visitors rollup table**: Visitor list queries hit raw sessions table; a rollup could improve performance at scale
- **Export/CSV download**: Dashboard is view-only; data export would require new API endpoints
- **Visitor identification/aliasing**: Visitors are identified by fingerprint hash; no mechanism to alias multiple visitor IDs to a known user
- **Custom funnel step configuration via UI**: Funnel steps are passed as query parameters; a UI picker would improve usability

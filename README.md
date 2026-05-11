# Pulse Analytics

Pulse Analytics is a self-hosted, cloud-native analytics service for teams that
want the breadth of a modern product analytics stack without locking all product
data into one vendor. It combines web analytics, product analytics, event
collection, session intelligence, feature workflows, privacy controls, BI, and
data routing behind one Rust service and one lightweight SDK.

The product bet is simple:

- Install in minutes.
- Track without slowing down the website.
- Keep data portable.
- Let teams bring their own database or warehouse.
- Make integration language-neutral: every app can call plain HTTP.

## What You Get

Pulse includes the core pieces usually split across several products:

- Web analytics: pageviews, referrers, devices, geo, realtime visitors, UTM and
  campaign reporting.
- Product analytics: custom events, funnels, goals, retention, cohorts, paths,
  activation reports, lifecycle reports, product impact analysis, dashboards,
  saved reports, and query explorer history.
- Identity and account analytics: user profiles, aliases, identity graph,
  account profiles, account members, SCIM users, and SCIM groups.
- Session intelligence: visitor timelines, session detail, session replay,
  scroll depth, outlinks, search queries, click heatmaps, visual event labels,
  and friction signals.
- Experimentation and activation: feature flags, remote config, A/B tests,
  surveys, NPS, sentiment, and in-app guides.
- Reliability analytics: JavaScript errors, releases, source maps, logs,
  alerts, and webhook notifications.
- AI and LLM analytics: natural-language analytics query helper, insights,
  LLM traces, generations, evaluations, token usage, latency, and cost stats.
- CDP-style data movement: event sources, source ingestions, destinations,
  retryable delivery outbox, signed webhooks, and destination health.
- BI layer: semantic metrics, row policies, SQL editor, visual queries,
  drill-through, CSV uploads, database connections, saved SQL, embeds, and
  query run history.
- Privacy and governance: data retention, IP anonymization, DNT/GPC handling,
  consent modes, bot filtering, DSAR export/delete, audit logs, tracking plans,
  event schemas, data dictionary, and instrumentation health.

## How Pulse Is Different

Most analytics tools optimize for one of these modes:

- Product analytics, like funnels and retention.
- Website analytics, like pageviews and attribution.
- Session replay and heatmaps.
- Feature flags and experiments.
- CDP event routing.
- BI dashboards.

Pulse is designed to make those workflows live together while staying
developer-friendly and self-hostable. The newest core differentiators are:

- **Low-overhead ingestion**: browser SDK batching is on by default and flushes
  with `sendBeacon` where available.
- **Language-neutral batch API**: any stack can POST events to `/api/batch`.
- **Bring-your-own-database BI**: query Pulse data, Postgres databases,
  ClickHouse, or any external warehouse exposed through the `http_json` adapter.
- **Data ownership**: Postgres stores Pulse control-plane and analytics data by
  default; Redis handles buffering, API-key cache, realtime state, and rate
  limiting.
- **Single service deployment**: Rust/Axum server, embedded docs, embedded
  tracking script, migrations on startup, and Docker support.

## Repository Layout

```text
.
|-- crates/
|   |-- pulse-server/          # Rust Axum server, routes, services, templates
|   `-- pulse-common/          # Shared request/response types
|-- sdk/                       # TypeScript SDK and browser auto-tracking script
|-- migrations/                # SQLx migrations
|-- docs/                      # Implementation notes and product roadmap
|-- scripts/                   # VPS setup and project onboarding helpers
|-- Dockerfile
|-- docker-compose.yml
|-- docker-compose.coolify.yml
|-- Makefile
`-- README.md
```

## Architecture

```mermaid
flowchart LR
    A["Website / App"] --> B["Pulse SDK or HTTP API"]
    B --> C["Axum ingestion routes"]
    C --> D["Redis buffers"]
    D --> E["Background flush workers"]
    E --> F["PostgreSQL analytics tables"]
    F --> G["Query API and Dashboard"]
    F --> H["BI layer"]
    H --> I["Postgres / ClickHouse / HTTP JSON adapters"]
    C --> J["Destination outbox"]
    J --> K["Webhooks and external systems"]
```

Key runtime components:

- **Rust server**: Axum routes, SQLx, Redis, background tasks, Askama templates.
- **PostgreSQL**: projects, API keys, sessions, events, pageviews, rollups, BI,
  governance, privacy, identity, replay, and feature data.
- **Redis**: ingestion buffers, realtime visitor sorted sets, API-key cache,
  session resolution, and rate limiting.
- **TypeScript SDK**: browser client, server client, React, Vue, Next.js, and
  React Native helpers.
- **Static script**: `/api/script.js` serves the minified browser tracker from
  `crates/pulse-server/static/pulse.min.js`.

## Quickstart With Docker Compose

The easiest local setup starts Pulse, Postgres, and Redis together.

```bash
make docker-up
```

Then open:

- Home: `http://localhost:8090/`
- Health: `http://localhost:8090/health`
- API docs: `http://localhost:8090/api/docs`
- Dashboard: `http://localhost:8090/dashboard`

The local compose file exposes:

- Pulse: `localhost:8090`
- Postgres: `localhost:5434`
- Redis: `localhost:6381`

Stop everything:

```bash
make docker-down
```

## Manual Local Development

Prerequisites:

- Rust toolchain
- Node.js and npm
- PostgreSQL
- Redis

Build everything:

```bash
make build
```

Run the server against local services:

```bash
export PULSE_PORT=8090
export ENVIRONMENT=development
export DATABASE_URL='postgres://pulse:pulse@localhost:5434/pulse_analytics?sslmode=disable'
export REDIS_URL='redis://localhost:6381/0'
export REDIS_KEY_PREFIX='pulse_analytics:'
export PULSE_ADMIN_TOKEN='dev-admin-token'
export ALLOWED_ORIGINS='*'
export RUST_LOG=debug

cargo run -p pulse-server
```

The server runs SQL migrations automatically on startup.

## Make Commands

```bash
make build          # Build SDK, copy browser script, build Rust release binary
make build-sdk      # Build TypeScript SDK and update static pulse.min.js
make build-server   # Build Rust server in release mode
make dev            # Run pulse-server locally
make dev-watch      # Run with cargo-watch
make check          # cargo check
make docker         # Build Docker image
make docker-up      # Start Docker Compose stack
make docker-down    # Stop Docker Compose stack
make clean          # Remove Rust and SDK build artifacts
make publish-sdk    # Publish npm SDK
```

## Configuration

| Variable | Required | Default | Purpose |
| --- | --- | --- | --- |
| `PULSE_PORT` | No | `8090` | HTTP port for the server. |
| `ENVIRONMENT` | No | `development` | Environment label; use `production` for production. |
| `DATABASE_URL` | Yes | none | PostgreSQL connection string. |
| `REDIS_URL` | Yes | none | Redis connection string. |
| `REDIS_KEY_PREFIX` | No | `pulse_analytics:` | Prefix for Redis keys. |
| `PULSE_ADMIN_TOKEN` | Yes | none | Bearer token for admin routes. |
| `PULSE_COOKIE_SECRET` | Production recommended | generated in memory | Cookie signing secret for dashboard sessions. |
| `ALLOWED_ORIGINS` | No | empty | Comma-separated CORS allow-list. Use `*` only for local/dev. |
| `GEOIP_DB_PATH` | No | none | Optional MaxMind database path. |
| `BUFFER_FLUSH_INTERVAL_SECS` | No | `5` | Redis buffer flush interval. |
| `BUFFER_BATCH_SIZE` | No | `500` | Max items flushed from each buffer per cycle. |
| `RATE_LIMIT_PER_SECOND` | No | `100` | Per-key/IP ingestion rate limit. |
| `DATA_RETENTION_DAYS` | No | `365` | Retention task cutoff. |
| `UMAMI_URL` | No | none | Optional Umami proxy URL. |
| `UMAMI_USER` | No | none | Optional Umami user. |
| `UMAMI_PASS` | No | none | Optional Umami password. |
| `EMAIL_REPORT_WEBHOOK_URL` | No | none | Optional email report delivery webhook. |
| `RUST_LOG` | No | `info` | Rust tracing filter. |

## First Project And API Key

Admin routes use:

```text
Authorization: Bearer <PULSE_ADMIN_TOKEN>
```

Create a project:

```bash
curl -sS -X POST http://localhost:8090/api/admin/projects \
  -H 'Authorization: Bearer dev-admin-token' \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "Demo App",
    "domain": "localhost",
    "settings": {}
  }'
```

Create an ingest key:

```bash
curl -sS -X POST http://localhost:8090/api/admin/projects/<project_id>/keys \
  -H 'Authorization: Bearer dev-admin-token' \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "web-ingest",
    "scopes": ["ingest"]
  }'
```

Create a query key:

```bash
curl -sS -X POST http://localhost:8090/api/admin/projects/<project_id>/keys \
  -H 'Authorization: Bearer dev-admin-token' \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "dashboard-query",
    "scopes": ["query"]
  }'
```

API keys are returned once. Store them immediately.

## Ingestion API

All ingestion routes require an API key with the `ingest` scope. Send it either
as `X-Pulse-Key` or as `?key=...` for beacon-friendly browser requests.

### Single Event

```bash
curl -sS -X POST http://localhost:8090/api/collect \
  -H 'Content-Type: application/json' \
  -H 'X-Pulse-Key: <ingest_key>' \
  -d '{
    "type": "event",
    "visitor_id": "visitor_123",
    "payload": {
      "name": "signup_clicked",
      "path": "/pricing",
      "data": { "plan": "pro" }
    },
    "consent_mode": "analytics",
    "consent_granted": true
  }'
```

### Batch Events

Use `/api/batch` to reduce network overhead or send backend/worker events in
bulk. A batch accepts up to 100 envelopes.

```bash
curl -sS -X POST http://localhost:8090/api/batch \
  -H 'Content-Type: application/json' \
  -H 'X-Pulse-Key: <ingest_key>' \
  -d '{
    "events": [
      {
        "type": "pageview",
        "visitor_id": "visitor_123",
        "payload": {
          "path": "/",
          "title": "Home",
          "referrer": ""
        }
      },
      {
        "type": "event",
        "visitor_id": "visitor_123",
        "payload": {
          "name": "cta_clicked",
          "path": "/",
          "data": { "variant": "hero" }
        }
      }
    ]
  }'
```

Supported event types include:

- `pageview`
- `event`
- `identify`
- `web_vital`
- `scroll_depth`
- `search_query`
- `outlink`
- `js_error`
- `log`
- `click_event`
- `survey_response`
- `session_replay`

## Browser Script Integration

The no-build integration is one script tag:

```html
<script
  async
  src="https://your-pulse-host.com/api/script.js"
  data-key="pa_live_xxx"
  data-api="https://your-pulse-host.com"
  data-batch="true"
  data-batch-size="10"
  data-batch-interval="2000"
  data-vitals="true"
  data-errors="true"
  data-outlinks="true"
></script>
```

Important attributes:

| Attribute | Default | Purpose |
| --- | --- | --- |
| `data-key` | required | Ingest API key. |
| `data-api` | current origin fallback | Pulse API base URL. |
| `data-batch` | `true` | Enable browser event queueing. |
| `data-batch-size` | `10` | Flush once this many events are queued. |
| `data-batch-interval` | `2000` | Flush interval in milliseconds. |
| `data-dnt` | `true` | Respect browser Do Not Track. |
| `data-consent-mode` | `analytics` | Consent mode sent to server. |
| `data-consent-granted` | `true` | Whether tracking consent is granted. |
| `data-utm` | `true` | Persist and attach UTM parameters. |
| `data-scroll` | `false` | Track scroll depth. |
| `data-vitals` | `false` | Track Core Web Vitals. |
| `data-outlinks` | `false` | Track downloads and external links. |
| `data-errors` | `false` | Track JS errors and unhandled rejections. |
| `data-clicks` | `false` | Track click heatmap points. |
| `data-search` | `false` | Track search query param. |
| `data-replay` | `false` | Track sampled session replay events. |

Manual browser calls:

```js
window.pulse("event", "signup_clicked", { plan: "pro" });
window.pulse("identify", "user_123", { email: "user@example.com" });
window.pulse("search", "analytics tools", 42);
window.pulse("log", "error", "Checkout failed", { code: "card_declined" });
window.pulse("consent", true, "analytics");
```

## TypeScript SDK

The SDK package is `@ayushojha/pulse-analytics`.

```bash
cd sdk
npm ci
npm run build
```

Browser client:

```ts
import { createPulse } from "@ayushojha/pulse-analytics";

const pulse = createPulse({
  apiKey: "pa_live_xxx",
  apiUrl: "https://your-pulse-host.com",
  batch: true,
  batchSize: 10,
  batchFlushIntervalMs: 2000,
  trackWebVitals: true,
  trackErrors: true,
});

pulse.event("signup_clicked", { plan: "pro" });
pulse.identify("user_123", { email: "user@example.com" });
await pulse.flush();
```

Server client:

```ts
import { PulseServerClient } from "@ayushojha/pulse-analytics/server";

const pulse = new PulseServerClient({
  apiKey: "pa_live_xxx",
  apiUrl: "https://your-pulse-host.com",
});

await pulse.trackEvent({
  visitorId: "visitor_123",
  eventName: "purchase",
  data: { plan: "pro" },
  revenueAmount: 49,
  revenueCurrency: "USD",
});

await pulse.collectBatch([
  {
    type: "event",
    visitorId: "visitor_123",
    payload: {
      name: "invoice_paid",
      data: { invoice_id: "inv_123" },
    },
  },
]);
```

Framework entrypoints:

- `@ayushojha/pulse-analytics`
- `@ayushojha/pulse-analytics/server`
- `@ayushojha/pulse-analytics/react`
- `@ayushojha/pulse-analytics/vue`
- `@ayushojha/pulse-analytics/next`
- `@ayushojha/pulse-analytics/react-native`

## Query API

Query routes require a key with the `query` scope.

```bash
curl -sS 'http://localhost:8090/api/v1/stats?start_at=2026-01-01T00:00:00Z&end_at=2026-01-31T23:59:59Z' \
  -H 'X-Pulse-Key: <query_key>'
```

Core query endpoints:

- `GET /api/v1/stats`
- `GET /api/v1/stats/timeseries`
- `GET /api/v1/pages`
- `GET /api/v1/referrers`
- `GET /api/v1/events`
- `GET /api/v1/devices`
- `GET /api/v1/geo`
- `GET /api/v1/realtime`

Feature endpoint groups include:

- `/api/v1/funnels`
- `/api/v1/goals`
- `/api/v1/retention`
- `/api/v1/cohorts`
- `/api/v1/paths`
- `/api/v1/campaigns`
- `/api/v1/marketing/*`
- `/api/v1/identity/*`
- `/api/v1/scim/*`
- `/api/v1/segments`
- `/api/v1/governance/*`
- `/api/v1/privacy/*`
- `/api/v1/feature-flags`
- `/api/v1/experiments`
- `/api/v1/session-replay`
- `/api/v1/heatmaps`
- `/api/v1/surveys`
- `/api/v1/guides`
- `/api/v1/errors`
- `/api/v1/logs`
- `/api/v1/alerts`
- `/api/v1/sources`
- `/api/v1/destinations`
- `/api/v1/ai/*`
- `/api/v1/bi/*`

For full request and response examples, run the server and open
`/api/docs`.

## BI And Bring-Your-Own-Database

Pulse can query its own analytics tables and external BI connections.

Supported external connection types:

| Type | Connection string | Notes |
| --- | --- | --- |
| `postgres` | `postgres://...` or `postgresql://...` | Runs in a read-only transaction and applies `allowed_schemas` as the search path. |
| `clickhouse` | `https://user:pass@clickhouse.example.com:8443/?database=analytics` | Uses ClickHouse HTTP API, read-only mode, `JSONEachRow`, and exactly one allowed database. |
| `http_json` | `https://adapter.example.com/query` | Universal adapter for any database/warehouse. Adapter receives SQL, limit, and `allowed_schemas`. |

Create a BI connection:

```bash
curl -sS -X POST http://localhost:8090/api/v1/bi/connections \
  -H 'X-Pulse-Key: <query_or_admin_key>' \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "Warehouse",
    "database_type": "http_json",
    "connection_string": "https://adapter.example.com/query",
    "allowed_schemas": ["analytics"],
    "is_active": true
  }'
```

Run external SQL:

```bash
curl -sS -X POST http://localhost:8090/api/v1/bi/connections/<connection_id>/query \
  -H 'X-Pulse-Key: <query_or_admin_key>' \
  -H 'Content-Type: application/json' \
  -d '{
    "sql_text": "SELECT event_name, count(*) AS events FROM events GROUP BY event_name",
    "limit": 100
  }'
```

HTTP adapter contract:

```json
{
  "sql": "SELECT ...",
  "limit": 100,
  "allowed_schemas": ["analytics"],
  "enforce_allowed_schemas": true
}
```

The adapter may return either:

```json
[{ "column": "value" }]
```

or:

```json
{ "rows": [{ "column": "value" }] }
```

or:

```json
{ "data": [{ "column": "value" }] }
```

Security behavior for HTTP adapters:

- Localhost, private, link-local, multicast, unspecified, and reserved IP ranges
  are blocked.
- Redirects are disabled.
- Userinfo passwords and common secret query parameters are redacted in API
  responses.
- ClickHouse connections must use exactly one allowed database.
- `http_json` adapters must enforce `allowed_schemas` with their own database
  credentials and query sandboxing.

## Modules

Pulse projects use module flags in project settings. `core` is always enabled.

Default enabled modules include:

- `core`
- `identity`
- `segments`
- `dashboards`
- `governance`
- `utm`
- `goals`
- `funnels`
- `retention`
- `exports`
- `integrations`
- `sources`
- `destinations`
- `bi`
- `alerts`
- `feature_flags`
- `revenue`
- `error_tracking`
- `logs`

Available modules also include:

- `cohorts`
- `paths`
- `webvitals`
- `scroll`
- `search`
- `outlinks`
- `sharing`
- `email_reports`
- `ab_testing`
- `session_replay`
- `heatmaps`
- `ai_queries`
- `predictions`
- `surveys`

API keys can also be restricted to specific modules with `allowed_modules`.

## Privacy, Governance, And Compliance

Pulse has built-in controls for:

- IP anonymization.
- DNT and Global Privacy Control handling.
- Consent-required ingestion.
- Allowed consent modes.
- Bot filtering and blocked user agents.
- Data retention windows.
- DSAR export and delete.
- Audit logs.
- Tracking plans.
- Event schema validation.
- Data dictionary entries.
- Quality violation reports.
- Instrumentation health.

The ingestion route accepts skipped events as successful no-ops when privacy
rules reject tracking, so applications do not break when consent is denied.

## Background Tasks

The server starts these background tasks:

- Redis buffer flush worker.
- Daily rollup worker.
- Partition management worker.
- Retention cleanup worker.
- Webhook alert worker.
- Destination delivery worker.
- Email report scheduler.

## Storage Model

The first migrations create:

- `projects`
- `api_keys`
- `sessions`
- `pageviews`
- `events`
- daily rollups for stats, pages, referrers, events, geo, and devices

Later migrations add identity, modules, webhooks, governance, privacy, feature
flags, errors, AI/LLM analytics, BI, embeds, marketing imports, SCIM, sources,
destinations, surveys, guides, and more.

`pageviews` and `events` are partitioned by time, and the partition worker keeps
future partitions ready.

## Deployment

### Docker

```bash
docker build -t pulse-analytics .
docker run --rm -p 8090:8090 \
  -e DATABASE_URL='postgres://...' \
  -e REDIS_URL='redis://...' \
  -e PULSE_ADMIN_TOKEN='change-me' \
  pulse-analytics
```

### Coolify

Use `docker-compose.coolify.yml` as the deployment shape for Coolify-style
stacks. Override secrets through environment variables rather than committing
production credentials.

### Production Checklist

- Set a strong `PULSE_ADMIN_TOKEN`.
- Set a stable `PULSE_COOKIE_SECRET`.
- Restrict `ALLOWED_ORIGINS`.
- Use managed Postgres backups.
- Use Redis persistence or managed Redis where appropriate.
- Put Pulse behind TLS.
- Keep external BI adapter URLs on public, trusted hosts.
- Use least-privilege database credentials for every external BI connection.
- Restrict API key scopes and `allowed_modules`.
- Monitor `/health`.
- Review retention and privacy settings before collecting real user data.

## Testing And Verification

Run all Rust tests:

```bash
cargo test --workspace --no-fail-fast
```

Run type/build verification for the SDK:

```bash
cd sdk
npm run build
```

Run a quick compile check:

```bash
cargo check --workspace
```

After SDK changes, rebuild and copy the browser script:

```bash
make build-sdk
```

## Documentation

- Live API docs: `/api/docs`
- Product positioning and roadmap: `docs/competitive-positioning-and-roadmap.md`
- Phase implementation notes: `docs/phase-*-implementation.md`

## Roadmap

Near-term priorities:

- Publish OpenAPI and copy-paste snippets for JavaScript, Python, Go, Ruby,
  PHP, Java, cURL, Cloudflare Workers, and serverless functions.
- Add official `http_json` adapter examples for MySQL, BigQuery, Snowflake,
  DuckDB, SQLite, and warehouse-first teams.
- Add benchmark tests for SDK payload size, request count, ingestion latency,
  and dashboard query latency.
- Add a ClickHouse storage backend for Pulse hot event tables while keeping
  Postgres as the control plane.
- Expand destinations beyond webhooks into warehouse and database sinks.
- Add setup health checks for tracking, batching, privacy, replay, vitals,
  flags, and external BI.

## License

MIT.

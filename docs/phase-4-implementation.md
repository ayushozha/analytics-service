# Phase 4: Deploy + Migrate — Implementation Documentation

## Overview

Phase 4 handles the deployment preparation for Pulse Analytics and the migration of the portfolio website (`ayush-portfolio`) from Umami to Pulse. This includes VPS setup scripts, DNS/database/Redis provisioning guides, replacing the Umami tracking script with Pulse, rewriting the admin analytics dashboard to use the Pulse query API, and cleaning up all analytics stubs from the portfolio engine service.

## Architecture Decisions

### VPS Setup Script vs Manual Provisioning
Chose an automated shell script (`scripts/setup-vps.sh`) that creates the PostgreSQL database, generates the Redis ACL command, and produces a `.env.production` file. This ensures repeatable deployments and reduces human error during setup.

### Coolify Deployment with Traefik
Reused the existing Coolify + Traefik infrastructure on VPS 72.62.82.57. The `docker-compose.coolify.yml` includes Traefik labels for automatic HTTPS via Let's Encrypt on `pulse.ayushojha.com`. No separate reverse proxy configuration needed.

### Client-Side Analytics Dashboard
Replaced the Umami iframe embed in `AnalyticsView.tsx` with a React client component that fetches directly from the Pulse query API. This gives full control over the dashboard UI and avoids cross-origin iframe issues.

### Clean Break from Umami
Removed all Umami references from the portfolio codebase rather than maintaining backwards compatibility. The portfolio now depends solely on Pulse for analytics. Umami can continue running on the VPS for other projects during the transition period.

## Technical Choices

### Pulse Script Tag Integration
Used Next.js `<Script>` component with `strategy="afterInteractive"` for the Pulse tracking script. The script tag reads `data-key` and `data-api` attributes, matching the SDK's auto-tracking behavior. Environment variables (`NEXT_PUBLIC_PULSE_URL`, `NEXT_PUBLIC_PULSE_KEY`) control whether analytics is enabled.

### Admin Dashboard: fetch() over SDK
The admin analytics dashboard uses raw `fetch()` calls to the Pulse query API rather than importing the TypeScript SDK. This keeps the admin panel lightweight and avoids adding a build dependency on `@ayushojha/pulse-analytics` to the portfolio's web app.

### Engine Service Cleanup
Removed analytics handler, routes, and models from the portfolio's Rust engine service. Also cleaned up unused `chrono` and `uuid` dependencies from Cargo.toml since they were only used by the now-removed analytics models.

## File Inventory

### New Files (Pulse Analytics repo)

| File | Description |
|------|-------------|
| `scripts/setup-vps.sh` | Automated VPS provisioning: creates database, generates Redis ACL, writes .env.production |
| `docker-compose.coolify.yml` | Production Docker Compose with Traefik labels for `pulse.ayushojha.com` |

### Modified Files (Portfolio repo)

| File | Change |
|------|--------|
| `apps/web/src/app/layout.tsx` | Replaced Umami `<Script>` with Pulse script tag using `NEXT_PUBLIC_PULSE_URL` and `NEXT_PUBLIC_PULSE_KEY` |
| `apps/web/src/components/admin/AnalyticsView.tsx` | Complete rewrite: Umami iframe → Pulse API-driven React dashboard with metric cards, change percentages, realtime indicator, and top pages table |
| `apps/web/src/components/admin/DashboardView.tsx` | Quick link changed from "Umami Analytics" → "Pulse Analytics", env var from `NEXT_PUBLIC_UMAMI_URL` → `NEXT_PUBLIC_PULSE_URL` |
| `.env.example` | Replaced `NEXT_PUBLIC_UMAMI_URL`, `NEXT_PUBLIC_UMAMI_WEBSITE_ID`, `UMAMI_SHARE_URL` with `NEXT_PUBLIC_PULSE_URL`, `NEXT_PUBLIC_PULSE_KEY` |
| `services/engine/Cargo.toml` | Removed `chrono` and `uuid` dependencies; removed chrono/uuid feature flags from sqlx |
| `services/engine/src/main.rs` | Removed analytics routes (`track_event`, `track_pageview`); removed unused `StatusCode` and `State` imports |
| `services/engine/src/models.rs` | Removed `AnalyticsEvent`, `PageviewEvent`, `AnalyticsStats`, `PageStats`, `ReferrerStats` structs; removed unused `chrono` and `uuid` imports |
| `services/engine/src/handlers/mod.rs` | Removed `pub mod analytics;` |

### Deleted Files (Portfolio repo)

| File | Description |
|------|-------------|
| `services/engine/src/handlers/analytics.rs` | Analytics handler stubs (was all TODOs, now replaced by Pulse) |

## Deployment Steps

### 1. VPS Database Setup
```bash
ssh ayush@72.62.82.57
# Create pulse_analytics database
PGPASSWORD='i87RfJUBx5HZJuykZt4v9u3zaq10wAqV' psql -h 127.0.0.1 -p 5433 -U admin -d postgres -c "CREATE DATABASE pulse_analytics;"
```

### 2. Redis ACL
```bash
# Add pulse_analytics_user to Redis ACL on VPS
redis-cli -p 6379 -a P0UnWC3CC7fsxV0Dsz2CgyDra19aL5iK
ACL SETUSER pulse_analytics_user on >{generated_password} ~pulse_analytics:* &* +@all
ACL SAVE
```

### 3. DNS
Point `pulse.ayushojha.com` A record → `72.62.82.57`

### 4. Deploy via Coolify
Use `docker-compose.coolify.yml` with environment variables set in Coolify dashboard. Traefik handles HTTPS automatically.

### 5. Run Migrations
```bash
# Via SSH tunnel
ssh -L 5433:127.0.0.1:5433 ayush@72.62.82.57 -N &
DATABASE_URL="postgres://admin:i87RfJUBx5HZJuykZt4v9u3zaq10wAqV@localhost:5433/pulse_analytics?sslmode=disable" \
  sqlx migrate run --source migrations/
```

### 6. Create First Project + API Key
```bash
# Create project
curl -X POST https://pulse.ayushojha.com/api/admin/projects \
  -H "Authorization: Bearer $PULSE_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"ayush-portfolio","domain":"ayushojha.com"}'

# Generate API key (save the returned key — shown only once)
curl -X POST https://pulse.ayushojha.com/api/admin/projects/{project_id}/keys \
  -H "Authorization: Bearer $PULSE_ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"production","scopes":["ingest","query"]}'
```

### 7. Update Portfolio Environment
Set `NEXT_PUBLIC_PULSE_URL=https://pulse.ayushojha.com` and `NEXT_PUBLIC_PULSE_KEY=pa_live_xxx` in the portfolio's production environment, then redeploy.

## Analytics Dashboard Features

The new `AnalyticsView.tsx` provides:

- **6 metric cards**: Pageviews, Visitors, Sessions, Bounce Rate, Avg Duration, Custom Events
- **Period comparison**: Shows percentage change vs previous 30-day period with color coding (green = good, red = bad; inverted for bounce rate)
- **Realtime indicator**: Green dot with active visitor count
- **Top pages table**: Path, views, and unique views for the top 10 pages
- **Graceful degradation**: Shows configuration message if env vars are missing; shows error message on API failure

## What's NOT Included

- **Actual VPS deployment execution**: The scripts and guides are ready but deployment hasn't been performed yet (requires SSH access and DNS propagation)
- **Umami data migration**: Historical Umami pageview data is not migrated into Pulse. The Umami proxy in Pulse's query API will merge historical data transparently when `umami_website_id` is configured on the project
- **Phase 5 polish items**: Built-in dashboard UI, data retention automation, partition management, webhook alerts, onboarding other projects

# Phase 3 Implementation — Client SDK, Script Tag, and Build Pipeline

## Overview

Phase 3 delivers the client-facing integration layer for Pulse Analytics:

1. **TypeScript SDK** (`@ayushojha/pulse-analytics`) — Typed NPM package with browser and server-side clients
2. **Drop-in Script Tag** — Auto-tracking IIFE script served from `/api/script.js` (761 bytes gzipped)
3. **Build Pipeline** — Makefile + multi-stage Dockerfile that builds SDK → embeds in Rust binary

## Architecture Decisions

### SDK as Monorepo Subpackage

The SDK lives inside the Pulse repo under `sdk/` rather than a separate repo. This keeps the tracking script and the server in sync — the Dockerfile builds the SDK first, then embeds the output into the Rust binary via `include_str!`. No runtime file system access needed.

### Three Integration Paths

| Integration | Use Case | Size |
|-------------|----------|------|
| `<script>` tag | Any website, zero dependencies | 761 bytes gzipped |
| NPM SDK (browser) | React/Next.js/Vue apps with typed tracking | 5KB ESM |
| NPM SDK (server) | Node.js backends for server-side events + queries | 3.2KB ESM |

### Script Tag: sendBeacon-First

The tracking script uses `navigator.sendBeacon` as the primary transport. This is reliable on page unload (unlike `fetch`) and doesn't block the UI thread. Falls back to `fetch` with `keepalive: true` for older browsers.

### Privacy by Default

- Visitor ID is a hash of screen resolution + timezone + language + UA — no cookies, no localStorage
- Session-scoped via `sessionStorage` (cleared when tab closes)
- Respects `navigator.doNotTrack` — if enabled, the script loads but sends nothing
- No PII collected

## Technical Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Bundler | tsup 8.x | Zero-config, outputs CJS + ESM + IIFE + .d.ts in one pass |
| Script format | IIFE (self-executing) | No global namespace pollution, runs immediately |
| Minification | esbuild (via tsup) | Fast, produces small output |
| Type declarations | Auto-generated `.d.ts` + `.d.mts` | Full IntelliSense for consumers |

## File Inventory

### SDK Source (`sdk/src/`)

| File | Description |
|------|-------------|
| `index.ts` | NPM entry point — re-exports PulseClient, createPulse, and all types |
| `client.ts` | `PulseClient` class — browser auto-tracking, event/pageview/identify, query API methods |
| `server.ts` | `PulseServerClient` class — server-side event tracking + full query API |
| `auto.ts` | Lightweight IIFE for script tag integration — auto-tracks pageviews, SPA navigation, exposes `window.pulse()` |
| `types.ts` | TypeScript interfaces for all API request/response contracts |

### SDK Build Outputs (`sdk/dist/`)

| File | Format | Size |
|------|--------|------|
| `index.mjs` | ESM | 5.0 KB |
| `index.js` | CJS | 6.0 KB |
| `index.d.ts` | Type declarations | 1.4 KB |
| `server.mjs` | ESM | 3.2 KB |
| `server.js` | CJS | 4.2 KB |
| `server.d.ts` | Type declarations | 1.3 KB |
| `pulse.min.global.js` | IIFE (minified) | 1.3 KB (761 bytes gzipped) |

### Build & Deploy

| File | Description |
|------|-------------|
| `Makefile` | Build commands: `make build`, `make build-sdk`, `make build-server`, `make dev`, `make docker`, `make publish-sdk` |
| `Dockerfile` | 3-stage build: Node (SDK) → Rust (server) → Debian slim (runtime) |
| `sdk/tsup.config.ts` | tsup configuration for CJS/ESM/IIFE builds |
| `sdk/package.json` | NPM package config with proper exports map |

## Build Pipeline

```
make build
  ├── make build-sdk
  │   ├── cd sdk && npm install
  │   ├── npm run build (tsup)
  │   └── cp sdk/dist/pulse.min.global.js → crates/pulse-server/static/pulse.min.js
  └── make build-server
      └── cargo build --release -p pulse-server
          └── include_str!("../../static/pulse.min.js")  ← embedded in binary
```

The Dockerfile follows the same pipeline with 3 stages:
1. **sdk-builder** (node:20-slim) — installs deps, runs tsup
2. **builder** (rust:1.77-slim) — copies SDK output into static/, builds Rust binary
3. **runtime** (debian:bookworm-slim) — copies binary + migrations only

## Integration Examples

### Script Tag (Simplest)

```html
<script
  src="https://pulse.ayushojha.com/api/script.js"
  data-key="pa_live_k8x9f2m3n7p1q4w6"
  defer
></script>

<script>
  // Custom events
  document.querySelector('#cta').addEventListener('click', () => {
    window.pulse('event', 'cta_click', { variant: 'hero' });
  });
</script>
```

Auto-tracks:
- Initial pageview on load
- SPA navigations (pushState, replaceState, popstate)
- Exposes `window.pulse('event', name, data)` and `window.pulse('identify', traits)`

### NPM SDK (Next.js / React)

```typescript
// lib/analytics.ts
import { createPulse } from '@ayushojha/pulse-analytics';

export const pulse = createPulse({
  apiKey: process.env.NEXT_PUBLIC_PULSE_KEY!,
  apiUrl: 'https://pulse.ayushojha.com',
  // autoTrack: true (default in browser)
});

// In a component
pulse.event('signup_click', { source: 'header', plan: 'pro' });

// Query analytics data
const stats = await pulse.getStats({
  startAt: new Date('2026-01-01'),
  endAt: new Date(),
});
```

### Server-Side SDK (Node.js API)

```typescript
// server/analytics.ts
import { PulseServerClient } from '@ayushojha/pulse-analytics/server';

const pulse = new PulseServerClient({
  apiKey: process.env.PULSE_SERVER_KEY!,
});

// Track server-side events with explicit context
await pulse.trackEvent({
  visitorId: 'user_abc123',
  eventName: 'purchase',
  data: { amount: 99, currency: 'USD' },
  ip: req.ip,
  userAgent: req.headers['user-agent'],
});

// Query for dashboard
const pages = await pulse.getPages({
  startAt: '2026-01-01T00:00:00Z',
  endAt: new Date(),
  limit: 10,
});
```

## NPM Publishing

```bash
# Login to npm (one-time)
npm login

# Publish
make publish-sdk
# or: cd sdk && npm publish --access public
```

The package publishes as `@ayushojha/pulse-analytics` with:
- `exports["."]` — browser client with auto-tracking
- `exports["./server"]` — server-side client (no DOM dependencies)

## What's NOT Included (Deferred)

- **React hooks** (`usePulse`, `PulseProvider`) — can be built as a wrapper package later
- **Vue/Svelte plugins** — consumers can use the core SDK directly
- **CDN publishing** — currently served from `/api/script.js`; could add to unpkg/jsdelivr via npm
- **SDK tests** — would add vitest for unit tests in a future iteration

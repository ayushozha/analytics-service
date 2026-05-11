# Pulse Analytics Competitive Positioning

Pulse is already broad: web and product analytics, events, funnels, retention,
cohorts, goals, sessions, replay, heatmaps, feature flags, experiments, surveys,
privacy controls, governance, sources, destinations, AI/LLM analytics, error
tracking, BI, embeds, and a TypeScript SDK. The opportunity is not to copy one
incumbent feature at a time. The sharper product bet is:

1. Install in minutes.
2. Add almost no website overhead.
3. Keep data portable.
4. Let teams use their own database and warehouse.
5. Expose one language-neutral HTTP contract that any stack can call.

## Competitor Pressure

| Competitor set | What they are strong at | Where Pulse was weaker | Product response |
| --- | --- | --- | --- |
| PostHog | All-in-one product engineering suite: analytics, replay, flags, experiments, surveys, data pipelines, error tracking, LLM analytics, and usage-based pricing. See [PostHog product overview](https://newsletter.posthog.com/p/what-is-posthog) and [pricing](https://posthog.com/posthug). | Pulse had comparable breadth in code, but less maturity around cloud scale, polished setup, and managed data pipelines. | Keep breadth, but differentiate with BYODB, transparent self-hosting, and lower client overhead. |
| Amplitude | Mature product analytics, cohorts, experiments, flags, session replay, and startup-friendly self-serve packaging. See [Amplitude pricing](https://amplitude.com/pricing?siteLocation=nav). | Pulse needs stronger guided analysis, activation templates, and product team UX. | Add opinionated dashboards and AI-assisted insight flows on top of existing events/funnels/cohorts. |
| Mixpanel | Fast event analytics, funnels, retention, cohorts, and attractive event-volume pricing. See [Mixpanel pricing](https://mixpanel.com/pricing/?trk=public_post_comment-text). | Pulse needs more benchmarked query performance and query UX polish. | Move hot analytical workloads toward columnar/external adapters and publish performance budgets. |
| Heap | Autocapture and retroactive event definition. | Pulse has visual labels and click capture, but needs smoother no-code retroactive workflows. | Improve visual event labeling and backfill previews. |
| Pendo | Product adoption, in-app guides, onboarding, feedback, and customer-success workflows. | Pulse has surveys and guides, but lacks a deeply integrated adoption workspace. | Connect guides, goals, accounts, churn signals, and lifecycle reporting. |
| Plausible / Matomo | Privacy-first analytics and data ownership. Plausible emphasizes no cookies or persistent identifiers in its [privacy-focused analytics](https://plausible.io/privacy-focused-web-analytics); Matomo emphasizes data ownership and privacy. | Pulse has privacy controls, but should make privacy posture visible during setup. | Ship privacy presets and lightweight cookieless mode docs. |
| Segment / RudderStack | CDP collection, routing, governance, and many destinations. Segment describes collecting first-party data and activating it across hundreds of destinations in [Customer Data pricing](https://segment.com/pricing/customer-data-platform/). RudderStack lists a free tier and many cloud destinations in [pricing](https://www.rudderstack.com/pricing/). | Pulse destinations were webhook-only and BI connections were Postgres-only. | Add batch ingestion, adapter contracts, and database/warehouse connectors. |
| Metabase / Looker | BI, semantic layer, governed dashboards, embeddings, and many database connectors. Metabase positions itself as open-source BI that connects to 20+ data sources on its [home page](https://www.metabase.com/). | Pulse had BI primitives, but limited external database coverage. | Add external BI adapters and keep embedded analytics simple. |

## Completed In This Iteration

- Added `/api/batch` for language-neutral batch ingestion. Any app can POST
  `{ "events": [...] }` using the same event envelope as `/api/collect`.
- Added browser SDK batching with `batch`, `batchSize`, and
  `batchFlushIntervalMs`. Browser batching defaults on to reduce request count.
- Fixed Beacon payloads to use `application/json` blobs instead of plain text.
- Added script-tag batching controls: `data-batch`, `data-batch-size`, and
  `data-batch-interval`.
- Added server SDK `collectBatch()` for backend and worker pipelines.
- Expanded BI connections from Postgres-only to:
  - `postgres`: direct read-only SQL with schema allow-list.
  - `clickhouse`: native HTTP read-only querying with JSONEachRow parsing and
    a single allowed database.
  - `http_json`: universal read-only SQL adapter contract for any database or
    warehouse exposed by a small service in any language. Adapters receive
    `allowed_schemas` and must enforce that scope with their own database
    credentials.
- Hardened HTTP adapter connections by blocking local/private/reserved network
  targets, disabling redirects, and redacting common secret query parameters.

## Next Iterations

1. Publish OpenAPI and copy-paste snippets for JavaScript, Python, Go, Ruby,
   PHP, Java, cURL, Cloudflare Workers, and serverless functions.
2. Add official adapter examples for MySQL, BigQuery, Snowflake, DuckDB, and
   SQLite using the `http_json` contract.
3. Add benchmark tests for SDK payload size, request count, ingestion latency,
   and dashboard query latency.
4. Add a ClickHouse storage backend for Pulse's own hot event tables while
   keeping Postgres as the control plane.
5. Expand destinations beyond webhooks into warehouse/database sinks.
6. Add setup health checks that tell users whether tracking, batching, privacy,
   replay, vitals, flags, and external BI are configured correctly.

# Pulse Analytics — Architecture & Features

> Living design document for the Pulse analytics service. It captures the current architecture, the known-issue fix plan, and the roadmap to make Pulse the most capable **AI-native** product-analytics + LLM-observability platform. Update it as PRs land.

_Status legend:_ ✅ shipped · 🔧 fix in roadmap · 🧠 AI feature · 💡 idea / future

## 1. What Pulse is today

Pulse is a multi-tenant, self-hostable analytics platform written in **Rust (Axum) + PostgreSQL + Redis**. It is deliberately broad — PostHog / Segment / Mixpanel / Plausible class feature coverage — exposed over **~224 HTTP routes**, **41 service modules**, and **28 SQL migrations** (~33k LOC). It compiles clean and ships with a green test suite.

Multi-tenancy is the core invariant: every request resolves a `project_id` from the authenticated API key (ingest / query scopes) or admin token — never from a request parameter — and all SQL is parameterized and project-scoped.

### 1.1 Component map

```
                         ┌──────────────────────────────────────────────┐
  Browser / Server SDK   │                 pulse-server                 │
  (script.js, /collect)  │                                              │
        │                │  middleware:  auth (api-key) · rate_limit     │
        ▼                │               (Redis) · cors                  │
  ┌───────────┐          │                                              │
  │  Ingest   │──events─▶│  routes/ingest  ──▶ services/ingestion ──┐    │
  └───────────┘          │  routes/query   ──▶ services/query       │    │
  ┌───────────┐          │  routes/features(_ext) ──▶ 40+ services  │    │
  │  Query /  │◀─stats──│  routes/dashboard (HTMX UI)               │    │
  │ Dashboard │          │  routes/admin   (admin-token)            │    │
  └───────────┘          └───────────────────────────────┬─────────┼────┘
                              background tasks:           │         │
                              flush · rollup · partition  ▼         ▼
                              retention · webhook ·   ┌────────┐ ┌────────┐
                              destinations · email     │ Redis  │ │Postgres│
                                                       │ buffer │ │ (part- │
                                                       │+session│ │itioned)│
                                                       └────────┘ └────────┘
```

### 1.2 Ingestion → storage data flow

1. SDK / server POSTs a discriminated-union envelope to `POST /api/collect` (or `/api/batch`), API-key + rate-limit gated. Source/CDP webhooks arrive at `POST /api/source/{id}/collect` (per-source token).
2. **12 event variants** are accepted: `pageview`, `event` (+revenue), `identify` (user + account/company traits + aliasing), `web_vital`, `scroll_depth`, `search_query`, `outlink`, `js_error`, `log`, `click_event`, `survey_response`, `session_replay`.
3. Privacy (DNT / GPC / consent), per-project module gating, and tracking-plan governance run inline; GeoIP + UA parsing enrich the event.
4. High-throughput types are **buffered in Redis lists** and flushed to **partitioned Postgres tables** every 5s; `identify` / `survey_response` / `session_replay` write synchronously.
5. Background tasks roll daily aggregates (`daily_stats`, `daily_pages`, …), manage partitions, enforce retention, and deliver webhooks / destinations / email reports.

## 2. Current capabilities (what Pulse tracks & answers today)

**Web & product analytics**

- Pageviews (path/title/referrer, UTM ×5, screen, language) + entry/exit pages
- Custom events with arbitrary JSON properties and per-event revenue (basic ecommerce)
- Sessions (30-min rolling, device/browser/OS via UA parse, geo via MaxMind, bounce, duration)
- Funnels, goals, retention, cohorts, path analysis, stickiness, lifecycle, activation, impact
- Real-time visitors (Redis sorted-set), timeseries, pages, referrers, devices, geo

**Identity & B2B**

- Identify / user profiles (trait merge), user aliases (anon↔known)
- Group/account (B2B) analytics: account profiles, memberships, per-account aggregates
- SCIM 2.0 users & groups synced into an identity graph

**Experimentation & feedback**

- A/B experiments + assignments, feature flags + evaluations
- Surveys / NPS, heatmaps (click coords), session replay recording

**Marketing**

- Campaigns, UTM sources/mediums, marketing channels, attribution, ecommerce
- AI-referrer detection (ChatGPT / Perplexity / Claude / Gemini / Copilot traffic)
- Marketing CSV imports, reverse-ETL destinations, integrations marketplace

**Observability & BI**

- Error tracking (fingerprint, release, environment), Web Vitals, structured logs
- Alerts, scheduled email reports, CSV exports, signed webhooks
- BI layer: SQL editor, visual query, drill-through, semantic metrics, row policies, external DB connections (Postgres/ClickHouse/HTTP), embeds, CSV uploads

**LLM telemetry (partial)**

- LLM observability recorder: traces / generations / evaluations + flat stats (Langfuse-style, manual-record)
- Natural-language query endpoint + insights — **currently keyword heuristics, not real AI** (see §4)

## 3. Known issues & fix plan 🔧

From a 16-agent audit of every subsystem. Severity-ordered; each is a small, contained PR.

| Severity | Issue | Fix summary |
|---|---|---|
| **CRITICAL** | BI free-form SQL has no table allowlist — full cross-tenant read of any table incl. api_keys & secrets <br>`crates/pulse-server/src/services/bi.rs:1962-1971, 1084-1098` | run_ad_hoc_sql/run_saved_query -> prepare_safe_sql (bi.rs:1962-1971, verified) only requires the literal '{{project_id}}' substring then substitutes the caller … |
| **CRITICAL** | External BI database connection strings stored in plaintext (no encryption at rest) <br>`crates/pulse-server/migrations/024_bi_database_connections.up.sql:6` | bi_database_connections.connection_string is TEXT (migration 024) and written/read verbatim; mask_connection_string only masks API responses, not the at-rest co… |
| **HIGH** | Buffer flush is lossy: LPOP before insert, batch-poisoning, and whole-cycle abort on first error <br>`crates/pulse-server/src/services/ingestion.rs:144-256, 266-318, 538-595` | Three compounding ingestion bugs: (a) flush_* LPOPs up to 500 items out of Redis BEFORE batch_insert; on any insert failure the popped events are gone forever (… |
| **HIGH** | Revoked/downgraded API keys keep working up to 5 minutes (auth cache never invalidated) <br>`crates/pulse-server/src/routes/admin.rs:136-152` | auth_middleware caches resolved keys in Redis under apikey:{sha256} for 300s (auth.rs:72, verified). revoke_api_key only sets is_active=false in Postgres (admin… |
| **HIGH** | Alerts only evaluated on ingest — drop/outage ('lt') alerts can structurally never fire <br>`crates/pulse-server/src/routes/ingest.rs:770-779` | evaluate_alerts runs exclusively via evaluate_alerts_async inside ingest handlers; main.rs starts flush/rollup/partition/retention/webhook/destination/email tas… |
| **HIGH** | Unique-visitor metrics summed across daily rollups — headline metric inflated on every multi-day range <br>`crates/pulse-server/src/services/query.rs:35-39, 391-393, 449-451` | daily_stats.visitors stores per-day COUNT(DISTINCT visitor_id); fetch_stats does SUM(visitors) across the range, same pattern in daily_devices/daily_geo/daily_r… |
| **HIGH** | No SSRF protection on reverse-ETL destinations and alert/webhook URLs <br>`crates/pulse-server/src/services/destinations.rs:558-562, 437-469` | create/update_destination accept any http(s) URL with no host allowlist or private-range check; the delivery worker POSTs project-controlled payloads to it and … |
| **HIGH** | "AI" surface performs no AI/LLM inference — keyword routing + static thresholds, no provider wired <br>`crates/pulse-server/src/services/ai.rs:210-312, 880-900` | answer_query runs detect_intent (ai.rs:880-900, verified: substring match into 8 fixed intents) and dispatches to a built-in query returning a format!() templat… |
| **HIGH** | Experiment results count pre-assignment conversions and assignments can duplicate — wrong A/B winners <br>`crates/pulse-server/src/services/experiments.rs:311-333` | get_experiment_results joins goal_conversions to experiment_assignments on visitor_id only with no gc.created_at >= ea.created_at constraint, crediting conversi… |

## 4. The AI vision — four pillars 🧠

Pulse already exposes an `/api/v1/ai/*` surface, but the natural-language query is **keyword intent-matching** (`detect_intent` → 8 fixed reports) and insights are **static thresholds** — there is **no LLM wired anywhere** (no provider in `Cargo.toml`, no AI env vars in `config.rs`). The good news: the AI service already delegates to the audited `qsvc::fetch_*` query functions, and `reqwest` is already a dependency — so real AI is a **bolt-on, not a rewrite**.

### 4.0 Foundation: pluggable LLM provider (keystone)

A single `services/llm/` module defining an async `LlmProvider` trait (`complete` + `complete_json`/function-calling) with **OpenAI, Anthropic, and Gemini** impls over the existing `reqwest` dep. New optional env: `PULSE_LLM_PROVIDER`, `PULSE_LLM_API_KEY`, `PULSE_LLM_MODEL`, `PULSE_LLM_BASE_URL`. Fully gated — ships dark, every feature below degrades gracefully to the existing deterministic path when unset. **Everything in §4 depends on this.**

### 4.1 AI Analyst — Natural-Language Query & Narrative Insights

_Today:_ Pulse's "AI" is pure keyword heuristics with no LLM anywhere in the repo (grep for gemini/anthropic/openai/generativelanguage/LLM client across crates+migrations returns empty). In services/ai.rs, answer_query (lines 193-337) calls detect_intent (lines 880-900), which is substring matching ("error"/"page"/"referrer"/"event"/"device"/"country"/"trend").

_Benchmarked against:_ Amplitude Ask Amplitude, Julius, Mixpanel Spark, PostHog Max AI, ThoughtSpot (Spotter/Sage)

_Gaps:_
- No LLM is ever invoked: detect_intent (ai.rs:880) is 7 hardcoded substring buckets; questions like 'compare signups from Google vs direct last week' or 'why did mobile bounce rate spike' cannot be expressed.
- Answers are fixed string templates (e.g. ai.rs:223 'The top page was {path}.') — no natural-language narrative, no reasoning, no follow-up.
- Insights are 3 static threshold rules (ai.rs:626-651) with no anomaly detection, no causal narrative, no prioritization — far below PostHog Max / Amplitude's generated explanations.
- No text-to-SQL / text-to-query: the safe execution engine in bi.rs (prepare_safe_sql + execute_sql) exists but is only driven by human-written SQL or the visual query builder — nothing translates a question into a scoped query.
- No streaming: every AI response is a single blocking JSON body (ai.rs returns AiQueryResponse). Competitors stream tokens/sections.
- No provider abstraction: even though the Gemini key is available in env, there is no trait/module to call any model, and no place in Config/state.rs to hold a model client.
- No grounding/guardrails specific to LLM output — bi.rs guardrails are battle-tested but not wired to any model that could emit unsafe SQL; no schema-card prompt, no allow-listed tables passed to a model, no validation that model SQL is scoped to project_id before execution.
- No way to suggest follow-up questions, chart specs, or to cite which rows/metrics support an answer (no grounding evidence beyond the raw result JSON).

_Proposed features:_

| Effort | Feature | Value | PR sketch |
|---|---|---|---|
| `S` | **Pluggable LLM provider abstraction (Gemini default)** | One small, swappable seam so every later AI feature shares retries, timeouts, and key handling instead of hand-rolling HTTP per fe… | New file crates/pulse-server/src/services/llm.rs defining `pub trait LlmProvider { async fn complete(&self, system: &str, user: &str) -> AppResult<String>; }` and a `GeminiProvider` impl that POSTs to… |
| `S` | **LLM narrative insight generation over computed stats** | Replaces the 3 static threshold rules (ai.rs:626-651) with a real written explanation of what changed and why, like Amplitude/Mixp… | In services/ai.rs, after overview() computes current/previous stats and top_pages (already available as the `result` json at ai.rs:657), add `narrate_insights(state, result) -> AppResult<String>` that… |
| `S` | **Grounding guardrails + LLM-SQL audit trail** | Hardens the text-to-query feature to ThoughtSpot-level governance: makes the model's SQL fully auditable and double-validated befo… | New migration 029_ai_llm_sql.up.sql adding columns to ai_query_runs: generated_sql TEXT, model VARCHAR(64), provider VARCHAR(32), grounded BOOLEAN NOT NULL DEFAULT false (down migration drops them). I… |
| `M` | **LLM text-to-query grounded on the existing safe-SQL engine** | This is the headline AI-native gap vs PostHog Max / ThoughtSpot: turn an arbitrary question into a scoped, read-only SQL query and… | Add `services::ai::answer_query_llm(state, project_id, question, start, end)`: (1) build a fixed schema card string listing only the whitelisted datasets already in bi.rs (pageviews, events, sessions,… |
| `M` | **Streaming answers via SSE** | Matches the token-streaming UX of Max AI / Julius so long narratives render progressively instead of one blocking JSON body (curre… | Add `async fn complete_stream(...) -> AppResult<impl Stream<Item=AppResult<String>>>` to the LlmProvider trait (Gemini :streamGenerateContent with alt=sse). New route POST /api/v1/ai/query/stream in m… |

### 4.2 Anomaly Detection & Forecasting

_Today:_ Pulse has only static-threshold alerting and hardcoded heuristic "insights" — no statistical anomaly detection, no forecasting, no root-cause analysis, and no real LLM client anywhere in the codebase.

ALERTS (crates/pulse-server/src/services/alerts.rs, 540 lines): AlertRule has a fixed operator (one of gt/lt/gte/lte/eq).

_Benchmarked against:_ Amplitude, Amplitude / Mixpanel, Anodot, Datadog, PostHog

_Gaps:_
- No statistical baseline: alerts compare to a hand-set constant (alerts.rs:20,276) — users must guess the right number and re-tune it constantly. No z-score, EWMA, or moving-average band.
- No seasonality awareness: a normal Monday-morning traffic spike or weekend dip will false-trigger any static threshold; competitors (Datadog/Anodot/Amplitude) all model day-of-week/hour-of-day baselines.
- No forecasting: nothing projects a KPI forward (no Holt-Winters/linear trend), so Pulse cannot warn 'visitors trending to halve by Friday' or 'error_count will breach X in 2 days' the way Datadog forecast monitors do.
- No 'what changed' root-cause: when pageviews/visitors drop, Pulse has all the breakdown rollups (daily_pages, daily_geo, daily_devices, daily_referrers, daily_campaigns) but no service that diffs current-vs-baseline per dimension to rank contributors — a core Amplitude/Anodot feature.
- No anomaly-condition alert type: alert_rules only supports operator+threshold (migrations/004:351-352); there is no 'alert when metric deviates > N sigma from expected' or '% change vs previous period' rule, unlike PostHog/Datadog.
- No AI-written explanations: AiInsight text is string-formatted from fixed rules (ai.rs:633,642); there is no LLM call to turn an anomaly + its root-cause breakdown into a plain-English narrative, despite reqwest already being available.

_Proposed features:_

| Effort | Feature | Value | PR sketch |
|---|---|---|---|
| `S` | **Seasonal (day-of-week) baseline for anomaly scoring** | Stops weekend/Monday false positives — matches Datadog 'seasonal' and Anodot auto-seasonality. Cheap: group the same metric by wee… | In anomaly.rs add `fn seasonal_expected(history: &[(NaiveDate,f64)], target_weekday) -> (f64,f64)` that filters to matching weekday() then reuses zscore/ewma_band. Add a `seasonal: bool` param (or con… |
| `M` | **Statistical anomaly scoring on daily rollups (z-score + EWMA)** | Turns Pulse's existing daily_stats history into Datadog/Amplitude-style 'is this value abnormal?' detection with no new ingestion … | Add crates/pulse-server/src/services/anomaly.rs with pure fns: `fn zscore(history: &[f64], current: f64) -> f64` (mean+sample stddev) and `fn ewma_band(history: &[f64], alpha: f64) -> (f64,f64)` (leve… |
| `M` | **Anomaly-condition alert rules (deviation & % change)** | Lets users say 'alert when error_count is >3 sigma above its 14-day baseline' or '>30% below previous period' instead of guessing … | Migration 029_alert_conditions.up.sql: `ALTER TABLE alert_rules ADD COLUMN condition_type VARCHAR(20) NOT NULL DEFAULT 'threshold', ADD COLUMN baseline_days INT, ADD COLUMN sensitivity DOUBLE PRECISIO… |
| `M` | **'What changed' root-cause breakdown diffing** | When a KPI moves, ranks which pages/countries/devices/referrers/campaigns drove it — the headline Amplitude/Anodot capability. Pul… | Add services/root_cause.rs: `pub async fn explain_change(db, project_id, metric, dimension, current_range, baseline_range) -> Vec<Contributor>`. For dimension in {path,country,device,referrer_domain,u… |
| `M` | **KPI forecasting (linear trend + Holt-Winters)** | Projects pageviews/visitors/events forward with a confidence band and supports 'will breach threshold in N days' — the Datadog for… | Add services/forecast.rs with pure fns: `fn linear_forecast(series: &[f64], horizon) -> Vec<f64>` (least-squares slope/intercept) and `fn holt_winters_add(series, alpha,beta,gamma, season_len, horizon… |
| `M` | **AI-explained anomalies (optional LLM narrative)** | Turns an anomaly + its root-cause table into a plain-English 'Visitors dropped 40% on May 28, driven mostly by /pricing (-2,100) a… | Add `pub anthropic_api_key: Option<String>` (env ANTHROPIC_API_KEY) to config.rs (next to existing Option fields ~line 11-21). Add services/ai_explain.rs::`async fn explain_anomaly(cfg, anomaly: &Anom… |

### 4.3 LLM Observability (Langfuse-class)

_Today:_ Pulse has a minimal but real LLM-observability slice, all added in migration 020_llm_analytics.up.sql and served by services/ai.rs (LLM* functions) via routes in features_ext.rs wired under /api/v1/ai/llm/* in main.rs (lines 446-467). It is gated behind the "ai_queries" module and the "query" API-key scope.

Three tabl

_Benchmarked against:_ Arize Phoenix, Helicone, LangSmith, Langfuse, OpenLLMetry (Traceloop)

_Gaps:_
- No nested spans/observations: llm_generations.trace_id links straight to a flat llm_traces row (020_llm_analytics.up.sql:34). There is no parent_observation_id/span_id/type column, so multi-step agent/chain/tool/retrieval trees cannot be represented or rendered as a span waterfall — a baseline feature of Langfuse, LangSmith, and Phoenix.
- No per-model / per-user cost & token dashboards: get_llm_stats (ai.rs:555-588) returns a single SUM/COUNT/AVG row with no GROUP BY. No breakdown by provider/model, by user_id, or over time, even though indexes idx_llm_generations_model and idx_llm_traces_user already exist to support it.
- No latency percentiles: get_llm_stats computes only AVG(latency_ms) (ai.rs:566). No p50/p90/p95/p99 (percentile_cont), which competitors treat as the primary latency SLO metric.
- No prompt versioning/management: there is no prompts table or prompt_version linkage. llm_generations.prompt is raw JSONB (020:39); generations cannot reference a managed, versioned prompt template the way Langfuse/Helicone do.
- No datasets + experiment eval runs: llm_evaluations (020:70) stores ad-hoc per-generation scores only. No datasets/dataset_items/experiment_runs schema, so offline batch eval over a fixed test set with experiment comparison is impossible.
- No online evals / scoring webhooks: evaluations must be POSTed manually to /api/v1/ai/llm/evaluations. There is no async evaluator that scores new generations automatically, and the existing webhook system (003_webhooks) is not fired on LLM events.
- No OpenTelemetry / OpenLLMetry ingestion: routes/ingest.rs has zero OTel/OTLP/gen_ai handling (grep found none); there is no /v1/traces OTLP endpoint. Competitors accept OTLP/OpenInference spans; Pulse only accepts its own bespoke JSON via /api/v1/ai/llm/*.
- No trace/generation search: list_llm_traces and list_llm_generations accept only AiHistoryQuery {limit, offset} (features_ext.rs:88, 117, 177). No filtering by model, provider, user_id, status, trace name, time range, cost, or full-text on prompt/completion — making the data un-navigable at scale.
- No PII redaction on LLM bodies: record_llm_generation (ai.rs:431-469) stores prompt/completion JSONB verbatim. services/privacy.rs offers only anonymize_ip / strip_geo_precision / is_bot_user_agent / visitor DSAR — nothing redacts prompt/completion text, and privacy is never invoked in the LLM record path.
- No cost alerting: alerts.rs VALID_METRICS = [pageviews, visitors, bounce_rate, error_count, avg_duration] (alerts.rs:34-40) and fetch_metric_value has no LLM branch — so spend, token volume, error rate, or latency on llm_generations cannot trigger an alert.
- No cached-token / reasoning-token accounting: llm_generations has only input_tokens/output_tokens/total_tokens (020:41-43). No cached_input_tokens or reasoning_tokens columns, so prompt-cache savings (a headline Helicone/Anthropic/OpenAI metric) cannot be tracked.
- No server-side cost computation: cost_usd is whatever the client sends (ai.rs:120, 462). There is no model pricing table to derive cost from tokens, so cost accuracy depends entirely on every caller doing the math correctly.

_Proposed features:_

| Effort | Feature | Value | PR sketch |
|---|---|---|---|
| `S` | **Grouped cost/token/latency stats with percentiles** | Turns the single flat stat into real dashboards: spend and tokens per model/provider/user plus p50/p90/p95/p99 latency — the exact… | In ai.rs add get_llm_stats_by(db, project_id, start, end, group_by) where group_by in {model, provider, user_id, day}; SQL GROUP BY with COUNT, SUM(total_tokens), SUM(cost_usd), and percentile_cont(0.… |
| `S` | **Trace & generation search/filter** | Makes captured data usable at volume; matches the filter-by-model/user/status/time search every competitor ships. | Add LlmTraceFilter/LlmGenerationFilter Query structs in features_ext.rs (model, provider, user_id, status, trace_key, min_cost, start_at, end_at, q for ILIKE on name/error_message). Thread optional WH… |
| `S` | **LLM cost & error alerting** | Closes the cost-control gap (Helicone cost alerts); lets teams get paged when spend, token burn, error rate, or latency spikes. | In alerts.rs append llm_cost, llm_tokens, llm_error_rate, llm_avg_latency to VALID_METRICS and add matching arms in fetch_metric_value querying llm_generations over the window (SUM(cost_usd), SUM(tota… |
| `S` | **PII redaction on prompt/completion at ingest** | Lets privacy-sensitive teams adopt LLM tracing safely; competitors gate adoption on exactly this. Reuses Pulse's existing privacy … | Add redact_text/redact_json (email, phone, credit-card, API-key regexes) to services/privacy.rs and a per-project flag redact_llm_io (extend PrivacySettings + 011_privacy_controls or a small migration… |
| `M` | **Nested spans via parent_observation_id** | Unlocks the core trace-tree waterfall that defines Langfuse/LangSmith/Phoenix; lets agent/chain/tool/retrieval steps render hierar… | New migration 029_llm_spans.up.sql adding to llm_generations: parent_id UUID NULL REFERENCES llm_generations(id) ON DELETE SET NULL, observation_type VARCHAR(32) DEFAULT 'generation' (span\|generation… |
| `M` | **Server-side cost + cached-token accounting** | Makes cost trustworthy instead of client-supplied and surfaces prompt-cache savings — a headline Helicone/OpenAI/Anthropic metric … | Migration 030: add cached_input_tokens INT DEFAULT 0 and reasoning_tokens INT DEFAULT 0 to llm_generations, plus a model_prices table (provider, model, input_price_per_1k, cached_input_price_per_1k, o… |
| `M` | **Online evaluators + scoring webhooks on new generations** | Automatic continuous scoring of production traffic plus webhook fan-out — matches LangSmith/Langfuse online evals and gives extern… | Reuse 003_webhooks: after record_llm_generation succeeds in ai.rs, enqueue an 'llm.generation.created' event via services/webhook.rs so external evaluators can score and POST back to /api/v1/ai/llm/ev… |
| `L` | **OpenTelemetry / OpenLLMetry (gen_ai) ingest endpoint** | Lets any OpenLLMetry/OpenInference-instrumented app send spans with zero Pulse-specific code — the de-facto integration path for t… | New routes/otel.rs handling POST /api/v1/ai/otel/v1/traces (OTLP/JSON ResourceSpans). Map gen_ai.* span attributes (gen_ai.request.model, gen_ai.usage.input_tokens/output_tokens, prompts/completions, … |
| `L` | **Datasets + experiment eval runs** | Adds offline batch evaluation over a fixed test set with experiment comparison — the eval half of Langfuse/LangSmith/Phoenix that … | Migration 031: datasets, dataset_items (input JSONB, expected_output JSONB), experiment_runs (dataset_id, name, status), experiment_results (experiment_run_id, dataset_item_id, generation_id, score, p… |

### 4.4 Autocapture & Session Intelligence

_Today:_ Pulse is a Rust/Axum + Postgres + Redis analytics service. Its tracker (sdk/src/auto.ts) is an EXPLICIT, opt-in collector, not autocapture: every interaction signal is gated behind a data-attribute that defaults OFF — trackClicks (data-clicks), trackErrors, trackOutlinks, trackScrollDepth, trackWebVitals, trackSessionR

_Benchmarked against:_ FullStory, Heap (auto-track), Pendo, PostHog

_Gaps:_
- No autocapture: every click/interaction is opt-in via data-attributes (default OFF) and custom events are 100% hand-coded pulse() calls. Heap/PostHog/Pendo/FullStory all auto-capture every click/change/submit/pageview with zero instrumentation.
- selector() in auto.ts captures only tag+id+first-2-classes; it drops element text, href, name, aria-label, data-* and a stable DOM path, so raw clicks cannot be turned into meaningful events later (vs PostHog's $autocapture elements chain + text).
- No-code event definition is fully manual (visual_event_labels needs a human-typed selector + name). Nothing suggests which selectors/texts are worth naming (vs Heap auto-track 'Defined events', Pendo tag-less feature tagging).
- AI is rule-based string templates — no LLM integration at all, so no narrative session summaries (vs PostHog session-replay AI summaries, FullStory AI session summaries).
- Frustration signals are limited to rage-click recompute-on-read; no dead-click, no error-rage, no persistence, no link to the session recording timeline (vs FullStory rage/dead/error/thrash clicks as first-class signals).
- No AI-suggested events or funnels from raw traffic — funnels must be hand-built (services/funnels.rs). Heap auto-track + PostHog suggested events build funnels from captured clicks automatically.
- No smart/auto segment discovery — saved_segments are hand-authored JSONB definitions; nothing surfaces high-value or anomalous cohorts (vs Pendo/Heap behavioral cohort suggestions).
- Session recordings have no full-text/semantic search and no friction-density scoring, so finding the 'worst' sessions at scale is impossible (list is recency-only).

_Proposed features:_

| Effort | Feature | Value | PR sketch |
|---|---|---|---|
| `S` | **Friction-density scoring + 'worst sessions' ordering on recordings** | Pendo/FullStory surface high-friction sessions automatically; Pulse lists recordings by recency only. A frustration_score column l… | migration 031_recording_scores.up.sql: ALTER TABLE session_recordings ADD COLUMN frustration_score INT NOT NULL DEFAULT 0. Compute it in session_replay.rs (in record_replay_events or a small recompute… |
| `M` | **Rich autocapture in the SDK (elements chain) + server-side raw-interaction store** | Turns Pulse from opt-in instrumentation into true autocapture parity with PostHog/Heap: every click/submit/change is captured with… | sdk/src/auto.ts: add cfg.autocapture (data-autocapture, default off for back-compat); add buildElementsChain(el) walking up to 5 ancestors emitting {tag,text(<=255, masked if maskReplayText),href,name… |
| `M` | **AI-suggested events from raw autocapture/click traffic** | Matches Heap auto-track + PostHog suggested events: instead of a human typing a selector into visual_event_labels, Pulse mines the… | New services/autocapture_suggest.rs: suggest_events(db, project_id, start, end) ranks autocapture_events (fallback click_events) by COUNT(*)/COUNT(DISTINCT visitor_id) grouped by (path, elements->>tex… |
| `M` | **AI session summaries (LLM narrative + friction points per recording)** | Direct parity with PostHog/FullStory AI session summaries — the single biggest session-intelligence gap. Converts an opaque events… | config.rs: add llm_api_key/llm_base_url/llm_model env (e.g. ANTHROPIC_API_KEY) — reqwest already a dep. New services/session_summary.rs: summarize_recording(db, state, project_id, recording_id) loads … |
| `M` | **Expanded frustration signals: dead-click + error-rage, persisted and replay-linked** | Brings Pulse to FullStory-level frustration coverage (rage already exists). Dead clicks (no following nav/event) and error-rage (c… | services/heatmaps.rs: extend detect_friction_signals to also emit 'dead_click' (click_events with no pageview AND no event within 3s for same session, via LEFT JOIN pageviews/events on session_id + ti… |
| `M` | **Smart segment discovery (auto-proposed cohorts from behavior)** | Matches Heap/Pendo behavioral cohort suggestions: Pulse has hand-authored saved_segments (JSONB definitions) but nothing proposes … | New services/segment_discovery.rs: discover_segments(db, project_id, start, end) generates candidate SegmentDefinition JSONB (reusing segments.rs SegmentDefinition/SegmentCondition shapes) from (a) to… |

## 5. Additional features to keep in the architecture 💡

Useful capabilities surfaced by the audit that aren't yet pillared — kept here so they stay on the radar as the schema evolves.

- Real LLM-backed natural-language querying (text-to-intent/params or text-to-SQL with function-calling) — current /ai/query is keyword routing only; no provider wired despite the query-service plumbing already existing
- Server-side LLM cost computation from a model-pricing table + token counting — cost_usd and tokens are entirely client-trusted, so total_cost_usd is only as accurate as each caller's own math and defaults to 0 when omitted
- Grouped/time-bucketed LLM stats (by model/provider/operation/day) — get_llm_stats returns one flat aggregate though idx_llm_generations_model already exists to support per-model queries
- Batch dedup / idempotency (messageId / insert_id / Idempotency-Key) — no unique constraint on event tables, so client retries (sendBeacon, network retries) double-count
- Explicit alias/merge endpoint (Segment alias / Mixpanel $merge) to stitch anonymous->known identities and back-fill historical events
- First-class server-side capture-to-storage API — the CDP source webhook never lands events in the events/pageviews tables, so server-emitted events can't be queried as analytics
- Generic breakdown/group-by on arbitrary properties (event_data JSONB keys, utm_*, path segment) and query-time filtering/segmentation — only fixed pre-rolled dimensions are queryable; a segments table exists but no core-metric endpoint applies it
- Sequential/ordered funnels with conversion windows — current funnel is unordered set-membership (dropoff can go negative)
- Per-project timezone-aware reporting — all bucketing is hard-coded UTC, shifting 'today' and every daily bucket by up to a full day for most of the world
- Anomaly/spike detection as a configurable alert condition (z-score/percent-change/seasonal) and AI anomaly detection on cost/latency/error rate — only static thresholds exist; the 14-day baseline logic is webhook-only
- Source-map content upload + server-side stack symbolication — source maps are URL-only metadata; minified stacks are never de-minified
- Cross-session / visitor-level multi-touch attribution with a conversion window — attribution is single-session-scoped, so first_touch == last_touch for single-pageview sessions
- Embeddings / pgvector / semantic search over events/prompts/traces — entirely absent (a differentiator for 'best AI tracking platform')
- AI-specific data retention + PII redaction for stored prompts/completions — no TTL on llm_* tables, raw prompts stored verbatim
- Webhook/alert delivery history, retry/backoff, dead-letter, and signed-timestamp replay protection; destination dispatcher lacks FOR UPDATE SKIP LOCKED so multi-instance/slow batches double-deliver
- Public shared-dashboard view route — create/list/delete exist but resolve_shared_token/verify_shared_password have no consumer route, so minted tokens can never be viewed (BI embeds have the correct public-route pattern to mirror)
- Audit-log coverage beyond 3 GDPR paths and real actor identity (key id/prefix) — key create/revoke, webhook/destination/flag/BI-connection changes are unaudited
- Least-privilege scopes: LLM-telemetry writes and SDK evaluate/active/guide-event endpoints sit behind read-oriented 'query' scope; SCIM mutations behind 'query' not admin
- Statement timeout + read-only transaction on internal BI SQL execution (external path already does SET TRANSACTION READ ONLY; internal path runs on the shared read-write pool with no timeout — pg_sleep DoS)
- Persistent client SDK identity (localStorage/cookie device id) and durable offline event queue — current visitor_id is a per-tab UA fingerprint and the queue is in-memory only

### 5.1 Differentiators worth designing toward

- **Embeddings / pgvector**: semantic search over events, prompts, and traces; ‘find sessions like this’, dedupe error groups semantically, RAG over a project's own data.
- **AI session summaries**: LLM turns a replay into a narrative + ranked friction points.
- **‘What changed’ root-cause**: when a KPI moves, auto-rank which pages/countries/devices/campaigns drove it, then have the LLM explain it in one sentence.
- **AI-suggested events / funnels / cohorts**: mine raw autocapture traffic to propose instrumentation instead of requiring hand-authored selectors.
- **OpenLLMetry / OpenTelemetry gen_ai ingest**: accept standard LLM spans so any instrumented app reports to Pulse with zero Pulse-specific code.

## 6. PR roadmap (small, ordered, one-at-a-time)

Each PR is one-sitting-sized and independently shippable. Fixes before features; the LLM foundation (PR 7) before AI features that depend on it.

| # | PR | Category | Size | Risk | Depends |
|---|---|---|---|---|---|
| 1 | Restrict BI ad-hoc/saved SQL to an allowlisted set of tenant tables (or run via low-priv RLS role) | fix | M | medium | — |
| 2 | Encrypt BI external-DB connection strings at rest (AES-256-GCM, env KMS key) | fix | M | medium | 1 |
| 3 | Make buffer flush durable: validate/truncate fields at ingest, per-row fallback, isolate per-key errors | fix | M | medium | — |
| 4 | Invalidate API-key auth cache on revoke/scope change + reuse SSRF guard on destinations/alerts/webhooks | fix | S | low | — |
| 5 | Fix unique-visitor overcounting on multi-day rollups (raw-table uniques or HLL) | fix | M | medium | — |
| 6 | Add scheduled alert evaluator background task; debounce per-event evaluation | fix | S | low | — |
| 7 | FOUNDATION: pluggable LLM provider abstraction + env config (no behavior change yet) | ai-foundation | M | low | — |
| 8 | Real LLM-backed NL query: optional LLM intent+param extraction feeding existing query services | ai-feature | M | low | 7 |
| 9 | Server-side LLM cost computation from a model-pricing table | ai-feature | S | low | — |
| 10 | Grouped + time-bucketed LLM stats (by model/provider/operation/day) | ai-feature | S | low | 9 |
| 11 | AI anomaly detection on metrics (z-score/percent-change) as a configurable alert condition + AI summary | ai-feature | M | low | 6 |
| 12 | Batch idempotency / dedup (messageId / Idempotency-Key) on ingest | coverage | M | medium | 3 |
| 13 | Fix experiment conversion timing + add unique assignment constraint | fix | M | medium | — |
| 14 | AI prompt retention + PII redaction controls; least-privilege scope fixes | coverage | M | low | 9 |
| 15 | Wire public shared-dashboard resolve route (mirror BI embed pattern) | coverage | S | low | — |

### Status
- **PR 1** ✅ open — BI SQL table allowlist + read-only transaction (closes the cross-tenant breach).
- PR 2+ — pending review of PR 1.

## 7. Cross-cutting architecture decisions

- **LLM provider is an env-gated trait** (`services/llm/`) — no hard dependency, no SDK bloat (reuses `reqwest`); deterministic fallbacks everywhere so self-hosters without a key lose nothing.
- **AI reuses the audited query layer** — text-to-query extracts `{intent, range, dimension, limit}` and dispatches to the SAME `qsvc::fetch_*` functions, so the LLM never emits raw SQL against the tenant DB (defense-in-depth, and it inherits all existing project scoping).
- **BI SQL is allowlisted + read-only** (PR 1) — any future LLM-generated SQL path must go through the same chokepoint.
- **Cost/quality numbers computed server-side** — never trust client-reported token/cost; derive from a `llm_model_pricing` table.
- **PII & retention for AI data** — prompts/completions get a size cap, optional redaction, and a shorter retention window than raw analytics.


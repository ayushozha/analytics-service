import type {
  PulseConfig,
  StatsQuery,
  PaginatedQuery,
  StatsResponse,
  PageData,
  ReferrerData,
  EventData,
  DeviceData,
  GeoData,
  RealtimeResponse,
  ListResponse,
  UserProfile,
  UserAlias,
  IdentityGraph,
  IdentityGraphQuery,
  AccountProfile,
  AccountMembership,
  AccountAnalytics,
  ScimUser,
  ScimUserInput,
  ScimGroup,
  ScimGroupInput,
  ScimGroupWithMembers,
  IdentifyAccountOptions,
  SessionRecording,
  SessionRecordingSummary,
  EmailReportListResponse,
  SavedSegment,
  SegmentDefinition,
  SegmentEvaluation,
  SegmentCompareRow,
  SegmentBreakdownRow,
  TrackingPlan,
  EventSchemaDefinition,
  EventSchemaStatus,
  DataDictionaryEntry,
  EventQualityViolation,
  GovernanceHealth,
  PrivacySettings,
  FeatureFlag,
  FeatureFlagType,
  FeatureFlagVariant,
  TargetingRules,
  FeatureFlagEvaluationContext,
  FeatureFlagEvaluationResult,
  FeatureFlagEvaluation,
  RemoteConfigEntry,
  RemoteConfigEvaluationResult,
  Funnel,
  FunnelResult,
  Goal,
  GoalStats,
  RetentionCohort,
  CohortGroup,
  PageFlow,
  CampaignStat,
  MarketingChannelStat,
  AttributionRow,
  EcommerceReport,
  AiReferrerStat,
  MarketingImport,
  MarketingImportInput,
  MarketingImportRow,
  MarketingImportSummary,
  WebVitalSummary,
  ClickHeatmapPoint,
  PageClickStats,
  FrictionSignal,
  VisualEventLabel,
  VisualEventLabelInput,
  VisualEventLabelStats,
  ErrorGroup,
  ErrorInstance,
  AppRelease,
  SourceMapArtifact,
  LogEntry,
  LogStats,
  AiInsight,
  AiQueryRun,
  AiQueryResponse,
  LlmTrace,
  LlmTraceInput,
  LlmGeneration,
  LlmGenerationInput,
  LlmEvaluation,
  LlmEvaluationInput,
  LlmStats,
  CustomDashboard,
  SavedReport,
  QueryExplorerRequest,
  QueryExplorerResponse,
  QueryExplorerRun,
  ProductStickinessReport,
  ProductLifecycleReport,
  ProductActivationRequest,
  ProductActivationReport,
  ProductImpactRequest,
  ProductImpactReport,
  Integration,
  IntegrationFilter,
  EventSource,
  SourceInput,
  SourceWithToken,
  SourceIngestion,
  SourceIngestResponse,
  Destination,
  DestinationInput,
  DestinationDelivery,
  DestinationHealth,
  SemanticMetric,
  SemanticMetricInput,
  BiConnectionTestResponse,
  BiDatabaseConnection,
  BiDatabaseConnectionInput,
  BiEmbed,
  BiEmbedInput,
  BiEmbedResolved,
  BiEmbedWithToken,
  BiExternalSqlRunRequest,
  BiRowPolicy,
  BiRowPolicyInput,
  SavedSqlQuery,
  SavedSqlInput,
  BiQueryRun,
  BiQueryResponse,
  BiSqlRunRequest,
  BiVisualQueryRequest,
  BiDrillThroughRequest,
  CsvUpload,
  CsvUploadInput,
  AlertInput,
  AlertRule,
  Experiment,
  ExperimentInput,
  ExperimentAssignment,
  ExperimentResults,
  Survey,
  SurveyNpsReport,
  SurveySentimentReport,
  InAppGuide,
  GuideInput,
  GuideEvent,
  GuideEventInput,
  GuideStats,
  SharedDashboard,
} from "./types";

const DEFAULT_API_URL = "https://pulse.ayushojha.com";

function toISO(d: string | Date): string {
  return d instanceof Date ? d.toISOString() : d;
}

function evaluationPayload(ctx: FeatureFlagEvaluationContext): Record<string, unknown> {
  return {
    visitor_id: ctx.visitorId,
    user_id: ctx.userId,
    traits: ctx.traits || {},
    context: ctx.context || {},
  };
}

function sourcePayload(input: SourceInput): Record<string, unknown> {
  return {
    name: input.name,
    source_type: input.sourceType || "webhook",
    description: input.description,
    schema: input.schema || {},
    config: input.config || {},
    is_active: input.isActive ?? true,
  };
}

function scimUserPayload(input: ScimUserInput): Record<string, unknown> {
  return {
    user_name: input.userName,
    external_id: input.externalId,
    active: input.active ?? true,
    display_name: input.displayName,
    given_name: input.givenName,
    family_name: input.familyName,
    emails: input.emails || [],
    traits: input.traits || {},
  };
}

function scimGroupPayload(input: ScimGroupInput): Record<string, unknown> {
  return {
    display_name: input.displayName,
    external_id: input.externalId,
    traits: input.traits || {},
    members: input.members || [],
  };
}

function destinationPayload(input: DestinationInput): Record<string, unknown> {
  return {
    name: input.name,
    destination_type: input.destinationType || "webhook",
    endpoint_url: input.endpointUrl,
    secret: input.secret,
    headers: input.headers || {},
    event_types: input.eventTypes || [],
    transform: input.transform || {},
    is_active: input.isActive ?? true,
  };
}

function marketingImportPayload(input: MarketingImportInput): Record<string, unknown> {
  return {
    provider: input.provider,
    name: input.name,
    rows: input.rows.map((row) => ({
      date: row.date,
      dimensions: row.dimensions || {},
      metrics: row.metrics || {},
      raw_row: row.rawRow || {},
    })),
    imported_by: input.importedBy,
    metadata: input.metadata || {},
  };
}

function semanticMetricPayload(input: SemanticMetricInput): Record<string, unknown> {
  return {
    key: input.key,
    name: input.name,
    description: input.description,
    dataset: input.dataset,
    expression: input.expression,
    filters: input.filters || {},
    is_active: input.isActive ?? true,
  };
}

function biDatabaseConnectionPayload(input: BiDatabaseConnectionInput): Record<string, unknown> {
  return {
    name: input.name,
    database_type: input.databaseType || "postgres",
    connection_string: input.connectionString,
    allowed_schemas: input.allowedSchemas || ["public"],
    is_active: input.isActive ?? true,
    created_by: input.createdBy,
  };
}

function biEmbedPayload(input: BiEmbedInput): Record<string, unknown> {
  return {
    name: input.name,
    resource_type: input.resourceType,
    resource_id: input.resourceId,
    resource_config: input.resourceConfig || {},
    allowed_origins: input.allowedOrigins || [],
    theme: input.theme || {},
    is_active: input.isActive ?? true,
    expires_at: input.expiresAt ? toISO(input.expiresAt) : undefined,
    created_by: input.createdBy,
  };
}

function savedSqlPayload(input: SavedSqlInput): Record<string, unknown> {
  return {
    name: input.name,
    description: input.description,
    sql_text: input.sqlText,
    parameters: input.parameters || {},
    created_by: input.createdBy,
  };
}

function biRowPolicyPayload(input: BiRowPolicyInput): Record<string, unknown> {
  return {
    name: input.name,
    dataset: input.dataset,
    field: input.field,
    operator: input.operator || "eq",
    values: input.values || [],
    is_active: input.isActive ?? true,
    created_by: input.createdBy,
  };
}

function csvUploadPayload(input: CsvUploadInput): Record<string, unknown> {
  return {
    name: input.name,
    description: input.description,
    columns: input.columns,
    rows: input.rows,
    uploaded_by: input.uploadedBy,
  };
}

function guidePayload(input: GuideInput): Record<string, unknown> {
  return {
    name: input.name,
    guide_type: input.guideType || "tour",
    steps: input.steps || [],
    targeting: input.targeting || {},
    appearance: input.appearance || {},
    priority: input.priority || 0,
  };
}

function guideEventPayload(input: GuideEventInput): Record<string, unknown> {
  return {
    visitor_id: input.visitorId,
    event_type: input.eventType,
    step_id: input.stepId,
    metadata: input.metadata ?? {},
    path: input.path,
  };
}

function llmTracePayload(input: LlmTraceInput): Record<string, unknown> {
  return {
    trace_key: input.traceKey,
    name: input.name,
    user_id: input.userId,
    visitor_id: input.visitorId,
    session_id: input.sessionId,
    metadata: input.metadata || {},
    status: input.status || "success",
    started_at: input.startedAt ? toISO(input.startedAt) : undefined,
    ended_at: input.endedAt ? toISO(input.endedAt) : undefined,
    duration_ms: input.durationMs,
  };
}

function llmGenerationPayload(input: LlmGenerationInput): Record<string, unknown> {
  return {
    trace_id: input.traceId,
    trace_key: input.traceKey,
    provider: input.provider,
    model: input.model,
    operation: input.operation || "chat_completion",
    prompt: input.prompt ?? {},
    completion: input.completion ?? {},
    input_tokens: input.inputTokens ?? 0,
    output_tokens: input.outputTokens ?? 0,
    total_tokens: input.totalTokens,
    latency_ms: input.latencyMs,
    cost_usd: input.costUsd ?? 0,
    status: input.status || "success",
    error_message: input.errorMessage,
    metadata: input.metadata ?? {},
  };
}

function llmEvaluationPayload(input: LlmEvaluationInput): Record<string, unknown> {
  return {
    generation_id: input.generationId,
    trace_id: input.traceId,
    trace_key: input.traceKey,
    evaluator: input.evaluator,
    metric: input.metric,
    score: input.score,
    label: input.label,
    passed: input.passed,
    metadata: input.metadata ?? {},
  };
}

function visualEventLabelPayload(input: VisualEventLabelInput): Record<string, unknown> {
  return {
    name: input.name,
    event_name: input.eventName,
    path_pattern: input.pathPattern || "*",
    element_selector: input.elementSelector,
    properties: input.properties ?? {},
    status: input.status || "active",
    created_by: input.createdBy,
  };
}

function experimentPayload(input: ExperimentInput): Record<string, unknown> {
  return {
    name: input.name,
    description: input.description,
    variants: input.variants,
    goal_id: input.goalId,
  };
}

function generateVisitorId(): string {
  if (typeof window === "undefined") return "server";
  const fp = [
    screen.width,
    screen.height,
    Intl.DateTimeFormat().resolvedOptions().timeZone,
    navigator.language,
    navigator.userAgent,
  ].join("|");
  let hash = 0;
  for (let i = 0; i < fp.length; i++) {
    hash = (hash << 5) - hash + fp.charCodeAt(i);
    hash |= 0;
  }
  const vid = "v_" + Math.abs(hash).toString(36);
  try {
    const stored = sessionStorage.getItem("_pv");
    if (stored) return stored;
    sessionStorage.setItem("_pv", vid);
  } catch {}
  return vid;
}

function jsonBeaconBody(body: string): BodyInit {
  return typeof Blob !== "undefined" ? new Blob([body], { type: "application/json" }) : body;
}

function saneNumber(value: number | undefined, fallback: number, min: number, max: number): number {
  if (typeof value !== "number" || !Number.isFinite(value)) return fallback;
  return Math.max(min, Math.min(Math.floor(value), max));
}

export class PulseClient {
  private config: Required<PulseConfig>;
  private visitorId: string;
  private queue: Record<string, unknown>[] = [];
  private queueTimer?: ReturnType<typeof setTimeout>;
  private maxScroll = 0;
  private lastPath = "";
  private replayEnabled = false;
  private replayStartedAt = 0;
  private replayEvents: Record<string, unknown>[] = [];
  private replayFlushTimer?: ReturnType<typeof setInterval>;

  constructor(config: PulseConfig) {
    this.config = {
      apiKey: config.apiKey,
      apiUrl: config.apiUrl || DEFAULT_API_URL,
      autoTrack: config.autoTrack ?? (typeof window !== "undefined"),
      respectDnt: config.respectDnt ?? true,
      consentMode: config.consentMode ?? "analytics",
      consentGranted: config.consentGranted ?? true,
      release: config.release ?? "",
      environment: config.environment ?? "production",
      debug: config.debug ?? false,
      batch: config.batch ?? (typeof window !== "undefined"),
      batchSize: saneNumber(config.batchSize, 10, 1, 100),
      batchFlushIntervalMs: saneNumber(config.batchFlushIntervalMs, 2000, 250, Number.MAX_SAFE_INTEGER),
      trackUtm: config.trackUtm ?? true,
      trackScrollDepth: config.trackScrollDepth ?? false,
      trackWebVitals: config.trackWebVitals ?? false,
      trackOutlinks: config.trackOutlinks ?? false,
      trackErrors: config.trackErrors ?? false,
      trackClicks: config.trackClicks ?? false,
      trackSearch: config.trackSearch ?? false,
      trackSessionReplay: config.trackSessionReplay ?? false,
      sessionReplaySampleRate: config.sessionReplaySampleRate ?? 1,
      maskReplayText: config.maskReplayText ?? true,
      searchParam: config.searchParam ?? "q",
    };
    this.visitorId = generateVisitorId();

    if (typeof window !== "undefined") {
      if (this.config.batch) this.setupBatchFlush();
      if (this.config.autoTrack) this.setupAutoTracking();
      if (this.config.trackScrollDepth) this.setupScrollTracking();
      if (this.config.trackWebVitals) this.setupWebVitals();
      if (this.config.trackOutlinks) this.setupOutlinkTracking();
      if (this.config.trackErrors) this.setupErrorTracking();
      if (this.config.trackClicks) this.setupClickTracking();
      if (this.config.trackSessionReplay) this.setupSessionReplay();
    }
  }

  private isDnt(): boolean {
    return this.config.respectDnt && typeof navigator !== "undefined" && navigator.doNotTrack === "1";
  }

  private log(...args: unknown[]) {
    if (this.config.debug) console.log("[pulse]", ...args);
  }

  private async send(type: string, payload: Record<string, unknown>) {
    if (this.isDnt()) return;

    const envelope = {
      type,
      payload,
      visitor_id: this.visitorId,
      consent_mode: this.config.consentMode,
      consent_granted: this.config.consentGranted,
    };

    if (this.config.batch) {
      this.enqueue(envelope);
      return;
    }

    const body = JSON.stringify(envelope);

    const url = `${this.config.apiUrl}/api/collect`;
    this.log("send", type, payload);

    try {
      if (typeof navigator !== "undefined" && navigator.sendBeacon) {
        navigator.sendBeacon(`${url}?key=${this.config.apiKey}`, jsonBeaconBody(body));
      } else {
        await fetch(url, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "X-Pulse-Key": this.config.apiKey,
          },
          body,
          keepalive: true,
        });
      }
    } catch (err) {
      this.log("send error", err);
    }
  }

  private setupBatchFlush() {
    window.addEventListener("pagehide", () => this.flushQueue(true));
    document.addEventListener("visibilitychange", () => {
      if (document.visibilityState === "hidden") this.flushQueue(true);
    });
  }

  private enqueue(envelope: Record<string, unknown>) {
    this.queue.push(envelope);
    if (this.queue.length >= this.config.batchSize) {
      this.flushQueue(false);
      return;
    }
    if (!this.queueTimer) {
      this.queueTimer = setTimeout(() => this.flushQueue(false), this.config.batchFlushIntervalMs);
    }
  }

  async flush(): Promise<void> {
    await this.flushQueue(false);
  }

  private async flushQueue(useBeacon: boolean): Promise<void> {
    if (this.queueTimer) {
      clearTimeout(this.queueTimer);
      this.queueTimer = undefined;
    }
    const url = `${this.config.apiUrl}/api/batch`;

    while (this.queue.length > 0) {
      const events = this.queue.splice(0, this.config.batchSize);
      const body = JSON.stringify({ events });
      this.log("flush", events.length);

      try {
        if (typeof navigator !== "undefined" && navigator.sendBeacon) {
          const sent = navigator.sendBeacon(`${url}?key=${this.config.apiKey}`, jsonBeaconBody(body));
          if (sent) continue;
          if (useBeacon) {
            this.queue.unshift(...events);
            break;
          }
        }
        await fetch(url, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "X-Pulse-Key": this.config.apiKey,
          },
          body,
          keepalive: true,
        });
      } catch (err) {
        this.log("flush error", err);
      }
    }
  }

  private async query<T>(path: string, params?: Record<string, string>): Promise<T> {
    const url = new URL(`${this.config.apiUrl}${path}`);
    if (params) {
      Object.entries(params).forEach(([k, v]) => url.searchParams.set(k, v));
    }
    const res = await fetch(url.toString(), {
      headers: { "X-Pulse-Key": this.config.apiKey },
    });
    if (!res.ok) throw new Error(`Pulse API error: ${res.status}`);
    return res.json();
  }

  private async mutate<T>(path: string, method: string, body?: unknown): Promise<T> {
    const res = await fetch(`${this.config.apiUrl}${path}`, {
      method,
      headers: {
        "Content-Type": "application/json",
        "X-Pulse-Key": this.config.apiKey,
      },
      body: body === undefined ? undefined : JSON.stringify(body),
      keepalive: method === "POST",
    });
    if (!res.ok) throw new Error(`Pulse API error: ${res.status}`);
    const text = await res.text();
    return (text ? JSON.parse(text) : undefined) as T;
  }

  private getUtmParams(): Record<string, string> {
    if (!this.config.trackUtm) return {};
    const params: Record<string, string> = {};
    try {
      const sp = new URLSearchParams(location.search);
      for (const key of ["utm_source", "utm_medium", "utm_campaign", "utm_content", "utm_term"]) {
        const val = sp.get(key);
        if (val) params[key] = val;
      }
      if (Object.keys(params).length > 0) {
        sessionStorage.setItem("_putm", JSON.stringify(params));
      } else {
        const stored = sessionStorage.getItem("_putm");
        if (stored) Object.assign(params, JSON.parse(stored));
      }
    } catch {}
    return params;
  }

  private setupAutoTracking() {
    this.pageview();

    const origPush = history.pushState;
    history.pushState = (...args) => {
      origPush.apply(history, args);
      this.pageview();
    };

    const origReplace = history.replaceState;
    history.replaceState = (...args) => {
      origReplace.apply(history, args);
      this.pageview();
    };

    window.addEventListener("popstate", () => this.pageview());
  }

  private setupScrollTracking() {
    this.lastPath = typeof location !== "undefined" ? location.pathname : "";
    window.addEventListener("scroll", () => {
      const docH = Math.max(document.body.scrollHeight, document.documentElement.scrollHeight);
      const winH = window.innerHeight;
      if (docH <= winH) { this.maxScroll = 100; return; }
      const pct = Math.round(((window.scrollY || document.documentElement.scrollTop) / (docH - winH)) * 100);
      if (pct > this.maxScroll) this.maxScroll = pct;
    }, { passive: true });
    window.addEventListener("beforeunload", () => {
      if (this.maxScroll > 0) this.send("scroll_depth", { path: this.lastPath, max_depth: this.maxScroll });
    });
  }

  private setupWebVitals() {
    const sv = (name: string, value: number, rating?: string) => {
      this.send("web_vital", { name, value: Math.round(value * 1000) / 1000, rating, path: location.pathname });
    };
    try {
      if (typeof PerformanceObserver !== "undefined") {
        new PerformanceObserver((list) => {
          const entries = list.getEntries();
          const last = entries[entries.length - 1] as any;
          if (last) {
            const v = last.startTime;
            sv("LCP", v, v <= 2500 ? "good" : v <= 4000 ? "needs-improvement" : "poor");
          }
        }).observe({ type: "largest-contentful-paint", buffered: true });

        new PerformanceObserver((list) => {
          for (const entry of list.getEntries()) {
            if (entry.name === "first-contentful-paint") {
              const v = entry.startTime;
              sv("FCP", v, v <= 1800 ? "good" : v <= 3000 ? "needs-improvement" : "poor");
            }
          }
        }).observe({ type: "paint", buffered: true });

        let clsVal = 0;
        new PerformanceObserver((list) => {
          for (const e of list.getEntries()) if (!(e as any).hadRecentInput) clsVal += (e as any).value;
        }).observe({ type: "layout-shift", buffered: true });
        window.addEventListener("beforeunload", () => {
          sv("CLS", clsVal, clsVal <= 0.1 ? "good" : clsVal <= 0.25 ? "needs-improvement" : "poor");
        });

        let inpVal = 0;
        new PerformanceObserver((list) => {
          for (const e of list.getEntries()) { const d = (e as any).duration || 0; if (d > inpVal) inpVal = d; }
        }).observe({ type: "event", buffered: true });
        window.addEventListener("beforeunload", () => {
          if (inpVal > 0) sv("INP", inpVal, inpVal <= 200 ? "good" : inpVal <= 500 ? "needs-improvement" : "poor");
        });
      }
      if (performance?.getEntriesByType) {
        const nav = performance.getEntriesByType("navigation")[0] as PerformanceNavigationTiming | undefined;
        if (nav) {
          const ttfb = nav.responseStart - nav.requestStart;
          sv("TTFB", ttfb, ttfb <= 800 ? "good" : ttfb <= 1800 ? "needs-improvement" : "poor");
        }
      }
    } catch {}
  }

  private setupOutlinkTracking() {
    const dlExts = new Set(["pdf", "zip", "rar", "7z", "gz", "tar", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "csv", "exe", "dmg", "apk", "ipa"]);
    document.addEventListener("click", (ev) => {
      const el = (ev.target as Element)?.closest("a") as HTMLAnchorElement | null;
      if (!el?.href) return;
      try {
        const url = new URL(el.href, location.href);
        if (url.hostname !== location.hostname) {
          this.send("outlink", { url: el.href, link_type: "outlink", path: location.pathname });
        } else {
          const ext = url.pathname.split(".").pop()?.toLowerCase();
          if (ext && dlExts.has(ext)) this.send("outlink", { url: el.href, link_type: "download", path: location.pathname });
        }
      } catch {}
    }, true);
  }

  private setupErrorTracking() {
    window.addEventListener("error", (ev) => {
      this.send("js_error", {
        message: ev.message || "Unknown error",
        filename: ev.filename,
        lineno: ev.lineno,
        colno: ev.colno,
        stack: ev.error?.stack,
        path: location.pathname,
        release: this.config.release || undefined,
        environment: this.config.environment,
      });
    });
    window.addEventListener("unhandledrejection", (ev) => {
      const r = ev.reason;
      this.send("js_error", {
        message: r?.message || String(r) || "Unhandled promise rejection",
        stack: r?.stack,
        path: location.pathname,
        release: this.config.release || undefined,
        environment: this.config.environment,
      });
    });
  }

  private setupClickTracking() {
    document.addEventListener("click", (ev) => {
      const selector = this.getElementSelector(ev.target as Element | null);
      this.send("click_event", {
        path: location.pathname,
        x: ev.clientX / window.innerWidth,
        y: (ev.clientY + window.scrollY) / Math.max(document.documentElement.scrollHeight, 1),
        element_selector: selector,
        viewport_width: window.innerWidth,
        viewport_height: window.innerHeight,
      });
    });
  }

  private setupSessionReplay() {
    const sampleRate = Math.max(0, Math.min(1, this.config.sessionReplaySampleRate));
    if (sampleRate <= 0 || Math.random() > sampleRate) return;

    this.replayEnabled = true;
    this.replayStartedAt = Date.now();
    this.recordReplayEvent("page", {
      path: location.pathname + location.search,
      title: document.title,
      width: window.innerWidth,
      height: window.innerHeight,
    });

    document.addEventListener("click", (ev) => {
      this.recordReplayEvent("click", {
        selector: this.getElementSelector(ev.target as Element | null),
        x: ev.clientX / window.innerWidth,
        y: (ev.clientY + window.scrollY) / Math.max(document.documentElement.scrollHeight, 1),
      });
    }, true);

    document.addEventListener("input", (ev) => {
      const target = ev.target as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement | null;
      if (!target) return;
      const value = "value" in target && typeof target.value === "string" ? target.value : "";
      this.recordReplayEvent("input", {
        selector: this.getElementSelector(target),
        masked: this.config.maskReplayText,
        value_length: this.config.maskReplayText ? value.length : undefined,
        value: this.config.maskReplayText ? undefined : value,
      });
    }, true);

    let lastScroll = 0;
    window.addEventListener("scroll", () => {
      const now = Date.now();
      if (now - lastScroll < 250) return;
      lastScroll = now;
      this.recordReplayEvent("scroll", {
        x: window.scrollX,
        y: window.scrollY,
        max_y: Math.max(document.documentElement.scrollHeight, document.body.scrollHeight),
      });
    }, { passive: true });

    window.addEventListener("visibilitychange", () => {
      this.recordReplayEvent("visibility", { state: document.visibilityState });
      if (document.visibilityState === "hidden") this.flushSessionReplay(false);
    });
    window.addEventListener("beforeunload", () => this.flushSessionReplay(true));
    this.replayFlushTimer = setInterval(() => this.flushSessionReplay(false), 5000);
  }

  private getElementSelector(target: Element | null): string {
    if (!target) return "";
    try {
      let selector = target.tagName?.toLowerCase() || "";
      if (target.id) selector += "#" + target.id;
      else if (target.className && typeof target.className === "string") {
        selector += "." + target.className.trim().split(/\s+/).slice(0, 2).join(".");
      }
      return selector;
    } catch {
      return "";
    }
  }

  private recordReplayEvent(type: string, data: Record<string, unknown>) {
    if (!this.replayEnabled) return;
    this.replayEvents.push({
      type,
      t: Date.now() - this.replayStartedAt,
      ...data,
    });
    if (this.replayEvents.length >= 50) this.flushSessionReplay(false);
  }

  private flushSessionReplay(isComplete: boolean) {
    if (!this.replayEnabled) return;
    if (this.replayEvents.length === 0 && !isComplete) return;
    const events = this.replayEvents.splice(0);
    this.send("session_replay", {
      events,
      started_at: this.replayStartedAt,
      duration_ms: Date.now() - this.replayStartedAt,
      entry_page: typeof location !== "undefined" ? location.pathname : undefined,
      screen: typeof screen !== "undefined" ? `${screen.width}x${screen.height}` : undefined,
      is_complete: isComplete,
    });
    if (isComplete && this.replayFlushTimer) clearInterval(this.replayFlushTimer);
  }

  // --- Core tracking methods ---

  setConsent(granted: boolean, mode = "analytics") {
    this.config.consentGranted = granted;
    this.config.consentMode = mode;
  }

  pageview(path?: string) {
    const utm = this.getUtmParams();
    this.send("pageview", {
      path: path || (typeof location !== "undefined" ? location.pathname + location.search : "/"),
      title: typeof document !== "undefined" ? document.title : undefined,
      referrer: typeof document !== "undefined" ? document.referrer : undefined,
      screen: typeof screen !== "undefined" ? `${screen.width}x${screen.height}` : undefined,
      language: typeof navigator !== "undefined" ? navigator.language : undefined,
      ...utm,
    });
    if (this.config.trackSearch) {
      try {
        const sp = new URLSearchParams(location.search);
        const q = sp.get(this.config.searchParam);
        if (q) this.send("search_query", { query: q, path: location.pathname });
      } catch {}
    }
  }

  event(name: string, data?: Record<string, unknown>, revenueAmount?: number, revenueCurrency?: string) {
    const payload: Record<string, unknown> = { name, data: data || {}, path: typeof location !== "undefined" ? location.pathname : undefined };
    if (revenueAmount != null) {
      payload.revenue_amount = revenueAmount;
      payload.revenue_currency = revenueCurrency || "USD";
    }
    this.send("event", payload);
  }

  identify(userIdOrTraits: string | Record<string, unknown>, traits?: Record<string, unknown>, account?: IdentifyAccountOptions) {
    const payload =
      typeof userIdOrTraits === "string"
        ? { user_id: userIdOrTraits, traits: traits || {} }
        : { traits: userIdOrTraits };
    if (account?.accountId) {
      Object.assign(payload, {
        account_id: account.accountId,
        account_name: account.accountName,
        account_traits: account.accountTraits || {},
        account_role: account.accountRole,
      });
    }
    this.send("identify", payload);
  }

  searchQuery(query: string, resultsCount?: number) {
    this.send("search_query", { query, results_count: resultsCount, path: typeof location !== "undefined" ? location.pathname : undefined });
  }

  surveyResponse(surveyId: string, answers: unknown[], completed = true) {
    this.send("survey_response", { survey_id: surveyId, answers, completed, path: typeof location !== "undefined" ? location.pathname : undefined });
  }

  trackLog(level: "trace" | "debug" | "info" | "warn" | "error" | "fatal" | string, message: string, body?: Record<string, unknown>) {
    this.send("log", {
      level,
      message,
      body: body || {},
      path: typeof location !== "undefined" ? location.pathname : undefined,
      release: this.config.release || undefined,
      environment: this.config.environment,
    });
  }

  // --- Query methods ---

  async getStats(params: StatsQuery): Promise<StatsResponse> {
    return this.query("/api/v1/stats", { start_at: toISO(params.startAt), end_at: toISO(params.endAt) });
  }

  async getPages(params: PaginatedQuery): Promise<ListResponse<PageData>> {
    return this.query("/api/v1/pages", { start_at: toISO(params.startAt), end_at: toISO(params.endAt), ...(params.limit ? { limit: String(params.limit) } : {}), ...(params.offset ? { offset: String(params.offset) } : {}) });
  }

  async getReferrers(params: PaginatedQuery): Promise<ListResponse<ReferrerData>> {
    return this.query("/api/v1/referrers", { start_at: toISO(params.startAt), end_at: toISO(params.endAt), ...(params.limit ? { limit: String(params.limit) } : {}) });
  }

  async getEvents(params: PaginatedQuery): Promise<ListResponse<EventData>> {
    return this.query("/api/v1/events", { start_at: toISO(params.startAt), end_at: toISO(params.endAt), ...(params.limit ? { limit: String(params.limit) } : {}) });
  }

  async getDevices(params: PaginatedQuery): Promise<ListResponse<DeviceData>> {
    return this.query("/api/v1/devices", { start_at: toISO(params.startAt), end_at: toISO(params.endAt), ...(params.limit ? { limit: String(params.limit) } : {}) });
  }

  async getGeo(params: PaginatedQuery): Promise<ListResponse<GeoData>> {
    return this.query("/api/v1/geo", { start_at: toISO(params.startAt), end_at: toISO(params.endAt), ...(params.limit ? { limit: String(params.limit) } : {}) });
  }

  async getRealtime(): Promise<RealtimeResponse> {
    return this.query("/api/v1/realtime");
  }

  async getUserProfiles(params?: { limit?: number; offset?: number }): Promise<ListResponse<UserProfile>> {
    return this.query("/api/v1/identity/users", {
      ...(params?.limit ? { limit: String(params.limit) } : {}),
      ...(params?.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async getUserProfile(visitorId: string): Promise<UserProfile> {
    return this.query(`/api/v1/identity/users/${encodeURIComponent(visitorId)}`);
  }

  async getUserAliases(userId: string): Promise<ListResponse<UserAlias>> {
    return this.query(`/api/v1/identity/aliases/${encodeURIComponent(userId)}`);
  }

  async getIdentityGraph(params: IdentityGraphQuery): Promise<IdentityGraph> {
    return this.query("/api/v1/identity/graph", {
      ...(params.visitorId ? { visitor_id: params.visitorId } : {}),
      ...(params.userId ? { user_id: params.userId } : {}),
      ...(params.accountId ? { account_id: params.accountId } : {}),
      ...(params.limit ? { limit: String(params.limit) } : {}),
    });
  }

  async getAccountProfiles(params?: { limit?: number; offset?: number }): Promise<ListResponse<AccountProfile>> {
    return this.query("/api/v1/identity/accounts", {
      ...(params?.limit ? { limit: String(params.limit) } : {}),
      ...(params?.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async getAccountProfile(accountId: string): Promise<AccountProfile> {
    return this.query(`/api/v1/identity/accounts/${encodeURIComponent(accountId)}`);
  }

  async getAccountMembers(accountId: string, params?: { limit?: number; offset?: number }): Promise<ListResponse<AccountMembership>> {
    return this.query(`/api/v1/identity/accounts/${encodeURIComponent(accountId)}/members`, {
      ...(params?.limit ? { limit: String(params.limit) } : {}),
      ...(params?.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async getAccountAnalytics(accountId: string, params?: { startAt?: string | Date; endAt?: string | Date }): Promise<AccountAnalytics> {
    return this.query(`/api/v1/identity/accounts/${encodeURIComponent(accountId)}/analytics`, {
      ...(params?.startAt ? { start_at: toISO(params.startAt) } : {}),
      ...(params?.endAt ? { end_at: toISO(params.endAt) } : {}),
    });
  }

  async getScimUsers(params?: { limit?: number; offset?: number }): Promise<ListResponse<ScimUser>> {
    return this.query("/api/v1/scim/users", {
      ...(params?.limit ? { limit: String(params.limit) } : {}),
      ...(params?.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async getScimUser(userId: string): Promise<ScimUser> {
    return this.query(`/api/v1/scim/users/${encodeURIComponent(userId)}`);
  }

  async createScimUser(input: ScimUserInput): Promise<ScimUser> {
    return this.mutate("/api/v1/scim/users", "POST", scimUserPayload(input));
  }

  async updateScimUser(userId: string, input: ScimUserInput): Promise<ScimUser> {
    return this.mutate(`/api/v1/scim/users/${encodeURIComponent(userId)}`, "PUT", scimUserPayload(input));
  }

  async deleteScimUser(userId: string): Promise<void> {
    return this.mutate(`/api/v1/scim/users/${encodeURIComponent(userId)}`, "DELETE");
  }

  async getScimGroups(params?: { limit?: number; offset?: number }): Promise<ListResponse<ScimGroup>> {
    return this.query("/api/v1/scim/groups", {
      ...(params?.limit ? { limit: String(params.limit) } : {}),
      ...(params?.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async getScimGroup(groupId: string): Promise<ScimGroupWithMembers> {
    return this.query(`/api/v1/scim/groups/${encodeURIComponent(groupId)}`);
  }

  async createScimGroup(input: ScimGroupInput): Promise<ScimGroupWithMembers> {
    return this.mutate("/api/v1/scim/groups", "POST", scimGroupPayload(input));
  }

  async updateScimGroup(groupId: string, input: ScimGroupInput): Promise<ScimGroupWithMembers> {
    return this.mutate(`/api/v1/scim/groups/${encodeURIComponent(groupId)}`, "PUT", scimGroupPayload(input));
  }

  async deleteScimGroup(groupId: string): Promise<void> {
    return this.mutate(`/api/v1/scim/groups/${encodeURIComponent(groupId)}`, "DELETE");
  }

  async getSessionRecordings(params: PaginatedQuery): Promise<ListResponse<SessionRecordingSummary>> {
    return this.query("/api/v1/session-replay", {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
      ...(params.limit ? { limit: String(params.limit) } : {}),
      ...(params.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async getSessionRecording(recordingId: string): Promise<SessionRecording> {
    return this.query(`/api/v1/session-replay/${encodeURIComponent(recordingId)}`);
  }

  async getEmailReports(): Promise<EmailReportListResponse> {
    return this.query("/api/v1/email-reports");
  }

  async getPrivacySettings(): Promise<PrivacySettings> {
    return this.query("/api/v1/privacy/settings");
  }

  async updatePrivacySettings(params: Partial<Pick<PrivacySettings,
    "anonymize_ip" |
    "respect_dnt" |
    "bot_filtering" |
    "consent_required" |
    "allowed_consent_modes" |
    "blocked_user_agents"
  >>): Promise<PrivacySettings> {
    return this.mutate("/api/v1/privacy/settings", "PUT", params);
  }

  async getSegments(): Promise<ListResponse<SavedSegment>> {
    return this.query("/api/v1/segments");
  }

  async createSegment(params: { name: string; description?: string; definition: SegmentDefinition }): Promise<SavedSegment> {
    return this.mutate("/api/v1/segments", "POST", params);
  }

  async updateSegment(segmentId: string, params: {
    name: string;
    description?: string;
    definition: SegmentDefinition;
    isActive?: boolean;
  }): Promise<SavedSegment> {
    return this.mutate(`/api/v1/segments/${encodeURIComponent(segmentId)}`, "PUT", {
      name: params.name,
      description: params.description,
      definition: params.definition,
      is_active: params.isActive ?? true,
    });
  }

  async deleteSegment(segmentId: string): Promise<void> {
    return this.mutate(`/api/v1/segments/${encodeURIComponent(segmentId)}`, "DELETE");
  }

  async evaluateSegment(segmentId: string, params: PaginatedQuery): Promise<SegmentEvaluation> {
    return this.query(`/api/v1/segments/${encodeURIComponent(segmentId)}/evaluate`, {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
      ...(params.limit ? { limit: String(params.limit) } : {}),
      ...(params.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async compareSegments(segmentIds: string[], params: StatsQuery): Promise<ListResponse<SegmentCompareRow>> {
    return this.query("/api/v1/segments/compare", {
      segment_ids: segmentIds.join(","),
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
    });
  }

  async breakdownSegment(segmentId: string, params: StatsQuery & { property: string; limit?: number }): Promise<ListResponse<SegmentBreakdownRow>> {
    return this.query(`/api/v1/segments/${encodeURIComponent(segmentId)}/breakdown`, {
      property: params.property,
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
      ...(params.limit ? { limit: String(params.limit) } : {}),
    });
  }

  async getTrackingPlans(): Promise<ListResponse<TrackingPlan>> {
    return this.query("/api/v1/governance/tracking-plans");
  }

  async getTrackingPlan(planId: string): Promise<TrackingPlan> {
    return this.query(`/api/v1/governance/tracking-plans/${encodeURIComponent(planId)}`);
  }

  async createTrackingPlan(params: {
    name: string;
    description?: string;
    enforcementMode?: "observe" | "reject" | string;
    isActive?: boolean;
  }): Promise<TrackingPlan> {
    return this.mutate("/api/v1/governance/tracking-plans", "POST", {
      name: params.name,
      description: params.description,
      enforcement_mode: params.enforcementMode ?? "observe",
      is_active: params.isActive ?? true,
    });
  }

  async updateTrackingPlan(planId: string, params: {
    name: string;
    description?: string;
    enforcementMode?: "observe" | "reject" | string;
    isActive?: boolean;
  }): Promise<TrackingPlan> {
    return this.mutate(`/api/v1/governance/tracking-plans/${encodeURIComponent(planId)}`, "PUT", {
      name: params.name,
      description: params.description,
      enforcement_mode: params.enforcementMode ?? "observe",
      is_active: params.isActive ?? true,
    });
  }

  async deleteTrackingPlan(planId: string): Promise<void> {
    return this.mutate(`/api/v1/governance/tracking-plans/${encodeURIComponent(planId)}`, "DELETE");
  }

  async getEventSchemas(params?: { trackingPlanId?: string }): Promise<ListResponse<EventSchemaDefinition>> {
    return this.query("/api/v1/governance/event-schemas", {
      ...(params?.trackingPlanId ? { tracking_plan_id: params.trackingPlanId } : {}),
    });
  }

  async getEventSchema(schemaId: string): Promise<EventSchemaDefinition> {
    return this.query(`/api/v1/governance/event-schemas/${encodeURIComponent(schemaId)}`);
  }

  async createEventSchema(params: {
    trackingPlanId?: string;
    eventName: string;
    description?: string;
    status?: EventSchemaStatus;
    requiredProperties?: string[];
    propertySchema?: EventSchemaDefinition["property_schema"];
  }): Promise<EventSchemaDefinition> {
    return this.mutate("/api/v1/governance/event-schemas", "POST", {
      tracking_plan_id: params.trackingPlanId,
      event_name: params.eventName,
      description: params.description,
      status: params.status ?? "draft",
      required_properties: params.requiredProperties ?? [],
      property_schema: params.propertySchema ?? {},
    });
  }

  async updateEventSchema(schemaId: string, params: {
    trackingPlanId?: string;
    eventName: string;
    description?: string;
    status?: EventSchemaStatus;
    requiredProperties?: string[];
    propertySchema?: EventSchemaDefinition["property_schema"];
  }): Promise<EventSchemaDefinition> {
    return this.mutate(`/api/v1/governance/event-schemas/${encodeURIComponent(schemaId)}`, "PUT", {
      tracking_plan_id: params.trackingPlanId,
      event_name: params.eventName,
      description: params.description,
      status: params.status ?? "draft",
      required_properties: params.requiredProperties ?? [],
      property_schema: params.propertySchema ?? {},
    });
  }

  async updateEventSchemaStatus(schemaId: string, status: EventSchemaStatus): Promise<EventSchemaDefinition> {
    return this.mutate(`/api/v1/governance/event-schemas/${encodeURIComponent(schemaId)}/status`, "PUT", { status });
  }

  async deleteEventSchema(schemaId: string): Promise<void> {
    return this.mutate(`/api/v1/governance/event-schemas/${encodeURIComponent(schemaId)}`, "DELETE");
  }

  async getDataDictionaryEntries(params?: { entryType?: string }): Promise<ListResponse<DataDictionaryEntry>> {
    return this.query("/api/v1/governance/data-dictionary", {
      ...(params?.entryType ? { entry_type: params.entryType } : {}),
    });
  }

  async createDataDictionaryEntry(params: {
    entryType: string;
    name: string;
    dataType?: string;
    description?: string;
    owner?: string;
    isPii?: boolean;
  }): Promise<DataDictionaryEntry> {
    return this.mutate("/api/v1/governance/data-dictionary", "POST", {
      entry_type: params.entryType,
      name: params.name,
      data_type: params.dataType,
      description: params.description,
      owner: params.owner,
      is_pii: params.isPii ?? false,
    });
  }

  async updateDataDictionaryEntry(entryId: string, params: {
    entryType: string;
    name: string;
    dataType?: string;
    description?: string;
    owner?: string;
    isPii?: boolean;
  }): Promise<DataDictionaryEntry> {
    return this.mutate(`/api/v1/governance/data-dictionary/${encodeURIComponent(entryId)}`, "PUT", {
      entry_type: params.entryType,
      name: params.name,
      data_type: params.dataType,
      description: params.description,
      owner: params.owner,
      is_pii: params.isPii ?? false,
    });
  }

  async deleteDataDictionaryEntry(entryId: string): Promise<void> {
    return this.mutate(`/api/v1/governance/data-dictionary/${encodeURIComponent(entryId)}`, "DELETE");
  }

  async getQualityViolations(params?: {
    eventName?: string;
    violationType?: string;
    limit?: number;
    offset?: number;
  }): Promise<ListResponse<EventQualityViolation>> {
    return this.query("/api/v1/governance/violations", {
      ...(params?.eventName ? { event_name: params.eventName } : {}),
      ...(params?.violationType ? { violation_type: params.violationType } : {}),
      ...(params?.limit ? { limit: String(params.limit) } : {}),
      ...(params?.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async getGovernanceHealth(): Promise<GovernanceHealth> {
    return this.query("/api/v1/governance/health");
  }

  async getFeatureFlags(): Promise<ListResponse<FeatureFlag>> {
    return this.query("/api/v1/feature-flags");
  }

  async createFeatureFlag(params: {
    key: string;
    name: string;
    description?: string;
    enabled?: boolean;
    flagType?: FeatureFlagType;
    defaultValue?: unknown;
    variants?: FeatureFlagVariant[];
    rolloutPercentage?: number;
    targetingRules?: TargetingRules;
    remoteConfig?: Record<string, unknown>;
    experimentId?: string;
    guardrailMetrics?: unknown[];
  }): Promise<FeatureFlag> {
    return this.mutate("/api/v1/feature-flags", "POST", {
      key: params.key,
      name: params.name,
      description: params.description,
      enabled: params.enabled ?? false,
      flag_type: params.flagType ?? "boolean",
      default_value: params.defaultValue ?? false,
      variants: params.variants ?? [],
      rollout_percentage: params.rolloutPercentage ?? 100,
      targeting_rules: params.targetingRules ?? { match: "all", conditions: [] },
      remote_config: params.remoteConfig ?? {},
      experiment_id: params.experimentId,
      guardrail_metrics: params.guardrailMetrics ?? [],
    });
  }

  async updateFeatureFlag(flagId: string, params: {
    key: string;
    name: string;
    description?: string;
    enabled?: boolean;
    flagType?: FeatureFlagType;
    defaultValue?: unknown;
    variants?: FeatureFlagVariant[];
    rolloutPercentage?: number;
    targetingRules?: TargetingRules;
    remoteConfig?: Record<string, unknown>;
    experimentId?: string;
    guardrailMetrics?: unknown[];
  }): Promise<FeatureFlag> {
    return this.mutate(`/api/v1/feature-flags/${encodeURIComponent(flagId)}`, "PUT", {
      key: params.key,
      name: params.name,
      description: params.description,
      enabled: params.enabled ?? false,
      flag_type: params.flagType ?? "boolean",
      default_value: params.defaultValue ?? false,
      variants: params.variants ?? [],
      rollout_percentage: params.rolloutPercentage ?? 100,
      targeting_rules: params.targetingRules ?? { match: "all", conditions: [] },
      remote_config: params.remoteConfig ?? {},
      experiment_id: params.experimentId,
      guardrail_metrics: params.guardrailMetrics ?? [],
    });
  }

  async deleteFeatureFlag(flagId: string): Promise<void> {
    return this.mutate(`/api/v1/feature-flags/${encodeURIComponent(flagId)}`, "DELETE");
  }

  async evaluateFeatureFlag(key: string, ctx?: Partial<FeatureFlagEvaluationContext>): Promise<FeatureFlagEvaluationResult> {
    return this.mutate(`/api/v1/feature-flags/${encodeURIComponent(key)}/evaluate`, "POST", evaluationPayload({
      visitorId: ctx?.visitorId || this.visitorId,
      userId: ctx?.userId,
      traits: ctx?.traits,
      context: ctx?.context,
    }));
  }

  async getFeatureFlagEvaluations(flagId: string, params?: { limit?: number; offset?: number }): Promise<ListResponse<FeatureFlagEvaluation>> {
    return this.query(`/api/v1/feature-flags/${encodeURIComponent(flagId)}/evaluations`, {
      ...(params?.limit ? { limit: String(params.limit) } : {}),
      ...(params?.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async getRemoteConfigs(): Promise<ListResponse<RemoteConfigEntry>> {
    return this.query("/api/v1/remote-config");
  }

  async createRemoteConfig(params: {
    key: string;
    description?: string;
    value?: unknown;
    targetingRules?: TargetingRules;
    isActive?: boolean;
  }): Promise<RemoteConfigEntry> {
    return this.mutate("/api/v1/remote-config", "POST", {
      key: params.key,
      description: params.description,
      value: params.value ?? {},
      targeting_rules: params.targetingRules ?? { match: "all", conditions: [] },
      is_active: params.isActive ?? true,
    });
  }

  async updateRemoteConfig(entryId: string, params: {
    key: string;
    description?: string;
    value?: unknown;
    targetingRules?: TargetingRules;
    isActive?: boolean;
  }): Promise<RemoteConfigEntry> {
    return this.mutate(`/api/v1/remote-config/${encodeURIComponent(entryId)}`, "PUT", {
      key: params.key,
      description: params.description,
      value: params.value ?? {},
      targeting_rules: params.targetingRules ?? { match: "all", conditions: [] },
      is_active: params.isActive ?? true,
    });
  }

  async deleteRemoteConfig(entryId: string): Promise<void> {
    return this.mutate(`/api/v1/remote-config/${encodeURIComponent(entryId)}`, "DELETE");
  }

  async evaluateRemoteConfig(key: string, ctx?: Partial<FeatureFlagEvaluationContext>): Promise<RemoteConfigEvaluationResult> {
    return this.mutate(`/api/v1/remote-config/${encodeURIComponent(key)}/evaluate`, "POST", evaluationPayload({
      visitorId: ctx?.visitorId || this.visitorId,
      userId: ctx?.userId,
      traits: ctx?.traits,
      context: ctx?.context,
    }));
  }

  // --- Module query methods ---

  async getFunnels(): Promise<ListResponse<Funnel>> {
    return this.query("/api/v1/funnels");
  }

  async analyzeFunnel(funnelId: string, params: StatsQuery): Promise<ListResponse<FunnelResult>> {
    return this.query(`/api/v1/funnels/${funnelId}/analyze`, { start_at: toISO(params.startAt), end_at: toISO(params.endAt) });
  }

  async getGoals(): Promise<ListResponse<Goal>> {
    return this.query("/api/v1/goals");
  }

  async getGoalStats(goalId: string, params: StatsQuery): Promise<GoalStats> {
    return this.query(`/api/v1/goals/${goalId}/stats`, { start_at: toISO(params.startAt), end_at: toISO(params.endAt) });
  }

  async getRetention(params: StatsQuery & { period?: string }): Promise<{ cohorts: RetentionCohort[] }> {
    return this.query("/api/v1/retention", { start_at: toISO(params.startAt), end_at: toISO(params.endAt), ...(params.period ? { period: params.period } : {}) });
  }

  async getCohorts(params: StatsQuery & { groupBy?: string; metric?: string }): Promise<ListResponse<CohortGroup>> {
    return this.query("/api/v1/cohorts", { start_at: toISO(params.startAt), end_at: toISO(params.endAt), ...(params.groupBy ? { group_by: params.groupBy } : {}), ...(params.metric ? { metric: params.metric } : {}) });
  }

  async getPaths(params: StatsQuery & { path: string; direction?: string; limit?: number }): Promise<ListResponse<PageFlow>> {
    return this.query("/api/v1/paths", { start_at: toISO(params.startAt), end_at: toISO(params.endAt), path: params.path, ...(params.direction ? { direction: params.direction } : {}), ...(params.limit ? { limit: String(params.limit) } : {}) });
  }

  async getCampaigns(params: StatsQuery): Promise<ListResponse<CampaignStat>> {
    return this.query("/api/v1/campaigns", { start_at: toISO(params.startAt), end_at: toISO(params.endAt) });
  }

  async getMarketingChannels(params: StatsQuery): Promise<ListResponse<MarketingChannelStat>> {
    return this.query("/api/v1/marketing/channels", {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
    });
  }

  async getMarketingAttribution(params: StatsQuery & { model?: "first_touch" | "last_touch" | "linear" | string }): Promise<ListResponse<AttributionRow>> {
    return this.query("/api/v1/marketing/attribution", {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
      ...(params.model ? { model: params.model } : {}),
    });
  }

  async getEcommerceReport(params: StatsQuery): Promise<EcommerceReport> {
    return this.query("/api/v1/marketing/ecommerce", {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
    });
  }

  async getAiReferrers(params: StatsQuery): Promise<ListResponse<AiReferrerStat>> {
    return this.query("/api/v1/marketing/ai-referrers", {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
    });
  }

  async getMarketingImports(params: { provider?: string; limit?: number; offset?: number } = {}): Promise<ListResponse<MarketingImport>> {
    return this.query("/api/v1/marketing/imports", {
      ...(params.provider ? { provider: params.provider } : {}),
      ...(params.limit ? { limit: String(params.limit) } : {}),
      ...(params.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async createMarketingImport(input: MarketingImportInput): Promise<MarketingImport> {
    return this.mutate("/api/v1/marketing/imports", "POST", marketingImportPayload(input));
  }

  async getMarketingImportRows(importId: string, params: { limit?: number; offset?: number } = {}): Promise<ListResponse<MarketingImportRow>> {
    return this.query(`/api/v1/marketing/imports/${encodeURIComponent(importId)}/rows`, {
      ...(params.limit ? { limit: String(params.limit) } : {}),
      ...(params.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async deleteMarketingImport(importId: string): Promise<void> {
    return this.mutate(`/api/v1/marketing/imports/${encodeURIComponent(importId)}`, "DELETE");
  }

  async getMarketingImportSummary(params: StatsQuery & { provider?: string }): Promise<MarketingImportSummary> {
    return this.query("/api/v1/marketing/imports/summary", {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
      ...(params.provider ? { provider: params.provider } : {}),
    });
  }

  async getWebVitals(params: StatsQuery): Promise<ListResponse<WebVitalSummary>> {
    return this.query("/api/v1/webvitals", { start_at: toISO(params.startAt), end_at: toISO(params.endAt) });
  }

  async getClickHeatmap(params: StatsQuery & { path: string }): Promise<ListResponse<ClickHeatmapPoint>> {
    return this.query("/api/v1/heatmaps", {
      path: params.path,
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
    });
  }

  async getClickStats(params: StatsQuery & { limit?: number }): Promise<ListResponse<PageClickStats>> {
    return this.query("/api/v1/heatmaps/stats", {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
      ...(params.limit ? { limit: String(params.limit) } : {}),
    });
  }

  async getFrictionSignals(params: StatsQuery & { path?: string; limit?: number }): Promise<ListResponse<FrictionSignal>> {
    return this.query("/api/v1/heatmaps/friction", {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
      ...(params.path ? { path: params.path } : {}),
      ...(params.limit ? { limit: String(params.limit) } : {}),
    });
  }

  async getVisualEventLabels(): Promise<ListResponse<VisualEventLabel>> {
    return this.query("/api/v1/heatmaps/labels");
  }

  async createVisualEventLabel(input: VisualEventLabelInput): Promise<VisualEventLabel> {
    return this.mutate("/api/v1/heatmaps/labels", "POST", visualEventLabelPayload(input));
  }

  async updateVisualEventLabel(labelId: string, input: VisualEventLabelInput): Promise<VisualEventLabel> {
    return this.mutate(`/api/v1/heatmaps/labels/${encodeURIComponent(labelId)}`, "PUT", visualEventLabelPayload(input));
  }

  async deleteVisualEventLabel(labelId: string): Promise<void> {
    return this.mutate(`/api/v1/heatmaps/labels/${encodeURIComponent(labelId)}`, "DELETE");
  }

  async getVisualEventLabelMatches(params: StatsQuery & { limit?: number }): Promise<ListResponse<VisualEventLabelStats>> {
    return this.query("/api/v1/heatmaps/labels/stats", {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
      ...(params.limit ? { limit: String(params.limit) } : {}),
    });
  }

  async getVisualEventLabelStats(labelId: string, params: StatsQuery): Promise<VisualEventLabelStats> {
    return this.query(`/api/v1/heatmaps/labels/${encodeURIComponent(labelId)}/stats`, {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
    });
  }

  async getErrors(params: PaginatedQuery): Promise<ListResponse<ErrorGroup>> {
    return this.query("/api/v1/errors", { start_at: toISO(params.startAt), end_at: toISO(params.endAt), ...(params.limit ? { limit: String(params.limit) } : {}), ...(params.offset ? { offset: String(params.offset) } : {}) });
  }

  async getErrorDetail(params: StatsQuery & { message?: string; fingerprint?: string; limit?: number }): Promise<ListResponse<ErrorInstance>> {
    return this.query("/api/v1/errors/detail", {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
      ...(params.message ? { message: params.message } : {}),
      ...(params.fingerprint ? { fingerprint: params.fingerprint } : {}),
      ...(params.limit ? { limit: String(params.limit) } : {}),
    });
  }

  async getReleases(params?: { limit?: number; offset?: number }): Promise<ListResponse<AppRelease>> {
    return this.query("/api/v1/releases", {
      ...(params?.limit ? { limit: String(params.limit) } : {}),
      ...(params?.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async createRelease(params: {
    version: string;
    environment?: string;
    commitSha?: string;
    deployedAt?: string | Date;
    metadata?: Record<string, unknown>;
  }): Promise<AppRelease> {
    return this.mutate("/api/v1/releases", "POST", {
      version: params.version,
      environment: params.environment,
      commit_sha: params.commitSha,
      deployed_at: params.deployedAt ? toISO(params.deployedAt) : undefined,
      metadata: params.metadata || {},
    });
  }

  async deleteRelease(releaseId: string): Promise<void> {
    return this.mutate(`/api/v1/releases/${encodeURIComponent(releaseId)}`, "DELETE");
  }

  async getSourceMaps(params?: { releaseVersion?: string; limit?: number; offset?: number }): Promise<ListResponse<SourceMapArtifact>> {
    return this.query("/api/v1/source-maps", {
      ...(params?.releaseVersion ? { release_version: params.releaseVersion } : {}),
      ...(params?.limit ? { limit: String(params.limit) } : {}),
      ...(params?.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async registerSourceMap(params: {
    releaseVersion: string;
    environment?: string;
    minifiedUrl: string;
    sourceMapUrl?: string;
    artifacts?: Record<string, unknown>;
    uploadedBy?: string;
  }): Promise<SourceMapArtifact> {
    return this.mutate("/api/v1/source-maps", "POST", {
      release_version: params.releaseVersion,
      environment: params.environment,
      minified_url: params.minifiedUrl,
      source_map_url: params.sourceMapUrl,
      artifacts: params.artifacts || {},
      uploaded_by: params.uploadedBy,
    });
  }

  async deleteSourceMap(sourceMapId: string): Promise<void> {
    return this.mutate(`/api/v1/source-maps/${encodeURIComponent(sourceMapId)}`, "DELETE");
  }

  async getLogs(params: PaginatedQuery & { level?: string; release?: string; environment?: string; search?: string }): Promise<ListResponse<LogEntry>> {
    return this.query("/api/v1/logs", {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
      ...(params.level ? { level: params.level } : {}),
      ...(params.release ? { release: params.release } : {}),
      ...(params.environment ? { environment: params.environment } : {}),
      ...(params.search ? { search: params.search } : {}),
      ...(params.limit ? { limit: String(params.limit) } : {}),
      ...(params.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async getLogStats(params: StatsQuery): Promise<LogStats> {
    return this.query("/api/v1/logs/stats", { start_at: toISO(params.startAt), end_at: toISO(params.endAt) });
  }

  async getDashboards(): Promise<ListResponse<CustomDashboard>> {
    return this.query("/api/v1/dashboards");
  }

  async getDashboard(dashboardId: string): Promise<CustomDashboard> {
    return this.query(`/api/v1/dashboards/${encodeURIComponent(dashboardId)}`);
  }

  async createDashboard(params: {
    name: string;
    description?: string;
    layout?: Record<string, unknown>;
    widgets?: unknown[];
    isDefault?: boolean;
  }): Promise<CustomDashboard> {
    return this.mutate("/api/v1/dashboards", "POST", {
      name: params.name,
      description: params.description,
      layout: params.layout || {},
      widgets: params.widgets || [],
      is_default: params.isDefault ?? false,
    });
  }

  async updateDashboard(dashboardId: string, params: {
    name: string;
    description?: string;
    layout?: Record<string, unknown>;
    widgets?: unknown[];
    isDefault?: boolean;
  }): Promise<CustomDashboard> {
    return this.mutate(`/api/v1/dashboards/${encodeURIComponent(dashboardId)}`, "PUT", {
      name: params.name,
      description: params.description,
      layout: params.layout || {},
      widgets: params.widgets || [],
      is_default: params.isDefault ?? false,
    });
  }

  async deleteDashboard(dashboardId: string): Promise<void> {
    return this.mutate(`/api/v1/dashboards/${encodeURIComponent(dashboardId)}`, "DELETE");
  }

  async getSavedReports(): Promise<ListResponse<SavedReport>> {
    return this.query("/api/v1/reports");
  }

  async getSavedReport(reportId: string): Promise<SavedReport> {
    return this.query(`/api/v1/reports/${encodeURIComponent(reportId)}`);
  }

  async createSavedReport(params: {
    name: string;
    description?: string;
    reportType: string;
    params?: Record<string, unknown>;
    visualization?: string;
    isActive?: boolean;
  }): Promise<SavedReport> {
    return this.mutate("/api/v1/reports", "POST", {
      name: params.name,
      description: params.description,
      report_type: params.reportType,
      params: params.params || {},
      visualization: params.visualization || "table",
      is_active: params.isActive ?? true,
    });
  }

  async updateSavedReport(reportId: string, params: {
    name: string;
    description?: string;
    reportType: string;
    params?: Record<string, unknown>;
    visualization?: string;
    isActive?: boolean;
  }): Promise<SavedReport> {
    return this.mutate(`/api/v1/reports/${encodeURIComponent(reportId)}`, "PUT", {
      name: params.name,
      description: params.description,
      report_type: params.reportType,
      params: params.params || {},
      visualization: params.visualization || "table",
      is_active: params.isActive ?? true,
    });
  }

  async deleteSavedReport(reportId: string): Promise<void> {
    return this.mutate(`/api/v1/reports/${encodeURIComponent(reportId)}`, "DELETE");
  }

  async runSavedReport(reportId: string, params?: { startAt?: string | Date; endAt?: string | Date }): Promise<QueryExplorerResponse> {
    const qs = new URLSearchParams();
    if (params?.startAt) qs.set("start_at", toISO(params.startAt));
    if (params?.endAt) qs.set("end_at", toISO(params.endAt));
    const suffix = qs.toString() ? `?${qs.toString()}` : "";
    return this.mutate(`/api/v1/reports/${encodeURIComponent(reportId)}/run${suffix}`, "POST");
  }

  async runQueryExplorer(input: QueryExplorerRequest): Promise<QueryExplorerResponse> {
    return this.mutate("/api/v1/query-explorer", "POST", input);
  }

  async getQueryExplorerHistory(params?: { limit?: number; offset?: number }): Promise<ListResponse<QueryExplorerRun>> {
    return this.query("/api/v1/query-explorer/history", {
      ...(params?.limit ? { limit: String(params.limit) } : {}),
      ...(params?.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async getProductStickiness(params: StatsQuery): Promise<ProductStickinessReport> {
    return this.query("/api/v1/product/stickiness", {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
    });
  }

  async getProductLifecycle(params: StatsQuery): Promise<ProductLifecycleReport> {
    return this.query("/api/v1/product/lifecycle", {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
    });
  }

  async getProductActivation(input: ProductActivationRequest): Promise<ProductActivationReport> {
    return this.mutate("/api/v1/product/activation", "POST", {
      start_at: toISO(input.startAt),
      end_at: toISO(input.endAt),
      event_names: input.eventNames || [],
      paths: input.paths || [],
    });
  }

  async getProductImpact(input: ProductImpactRequest): Promise<ProductImpactReport> {
    return this.mutate("/api/v1/product/impact", "POST", {
      metric: input.metric,
      split_at: toISO(input.splitAt),
      window_days: input.windowDays,
      event_name: input.eventName,
    });
  }

  async getBiConnections(): Promise<ListResponse<BiDatabaseConnection>> {
    return this.query("/api/v1/bi/connections");
  }

  async getBiConnection(connectionId: string): Promise<BiDatabaseConnection> {
    return this.query(`/api/v1/bi/connections/${encodeURIComponent(connectionId)}`);
  }

  async createBiConnection(input: BiDatabaseConnectionInput): Promise<BiDatabaseConnection> {
    return this.mutate("/api/v1/bi/connections", "POST", biDatabaseConnectionPayload(input));
  }

  async updateBiConnection(connectionId: string, input: BiDatabaseConnectionInput): Promise<BiDatabaseConnection> {
    return this.mutate(`/api/v1/bi/connections/${encodeURIComponent(connectionId)}`, "PUT", biDatabaseConnectionPayload(input));
  }

  async deleteBiConnection(connectionId: string): Promise<void> {
    return this.mutate(`/api/v1/bi/connections/${encodeURIComponent(connectionId)}`, "DELETE");
  }

  async testBiConnection(connectionId: string): Promise<BiConnectionTestResponse> {
    return this.mutate(`/api/v1/bi/connections/${encodeURIComponent(connectionId)}/test`, "POST");
  }

  async runBiConnectionSql(connectionId: string, input: BiExternalSqlRunRequest): Promise<BiQueryResponse> {
    return this.mutate(`/api/v1/bi/connections/${encodeURIComponent(connectionId)}/query`, "POST", {
      sql_text: input.sqlText,
      limit: input.limit,
    });
  }

  async getBiEmbeds(): Promise<ListResponse<BiEmbed>> {
    return this.query("/api/v1/bi/embeds");
  }

  async getBiEmbed(embedId: string): Promise<BiEmbed> {
    return this.query(`/api/v1/bi/embeds/${encodeURIComponent(embedId)}`);
  }

  async createBiEmbed(input: BiEmbedInput): Promise<BiEmbedWithToken> {
    return this.mutate("/api/v1/bi/embeds", "POST", biEmbedPayload(input));
  }

  async updateBiEmbed(embedId: string, input: BiEmbedInput): Promise<BiEmbed> {
    return this.mutate(`/api/v1/bi/embeds/${encodeURIComponent(embedId)}`, "PUT", biEmbedPayload(input));
  }

  async deleteBiEmbed(embedId: string): Promise<void> {
    return this.mutate(`/api/v1/bi/embeds/${encodeURIComponent(embedId)}`, "DELETE");
  }

  async rotateBiEmbedToken(embedId: string): Promise<BiEmbedWithToken> {
    return this.mutate(`/api/v1/bi/embeds/${encodeURIComponent(embedId)}/rotate-token`, "POST");
  }

  async resolveBiEmbed(token: string): Promise<BiEmbedResolved> {
    return this.query(`/api/embed/bi/${encodeURIComponent(token)}`);
  }

  async getBiMetrics(): Promise<ListResponse<SemanticMetric>> {
    return this.query("/api/v1/bi/metrics");
  }

  async getBiMetric(metricId: string): Promise<SemanticMetric> {
    return this.query(`/api/v1/bi/metrics/${encodeURIComponent(metricId)}`);
  }

  async createBiMetric(input: SemanticMetricInput): Promise<SemanticMetric> {
    return this.mutate("/api/v1/bi/metrics", "POST", semanticMetricPayload(input));
  }

  async updateBiMetric(metricId: string, input: SemanticMetricInput): Promise<SemanticMetric> {
    return this.mutate(`/api/v1/bi/metrics/${encodeURIComponent(metricId)}`, "PUT", semanticMetricPayload(input));
  }

  async deleteBiMetric(metricId: string): Promise<void> {
    return this.mutate(`/api/v1/bi/metrics/${encodeURIComponent(metricId)}`, "DELETE");
  }

  async getBiRowPolicies(): Promise<ListResponse<BiRowPolicy>> {
    return this.query("/api/v1/bi/row-policies");
  }

  async createBiRowPolicy(input: BiRowPolicyInput): Promise<BiRowPolicy> {
    return this.mutate("/api/v1/bi/row-policies", "POST", biRowPolicyPayload(input));
  }

  async updateBiRowPolicy(policyId: string, input: BiRowPolicyInput): Promise<BiRowPolicy> {
    return this.mutate(`/api/v1/bi/row-policies/${encodeURIComponent(policyId)}`, "PUT", biRowPolicyPayload(input));
  }

  async deleteBiRowPolicy(policyId: string): Promise<void> {
    return this.mutate(`/api/v1/bi/row-policies/${encodeURIComponent(policyId)}`, "DELETE");
  }

  async runBiSql(input: BiSqlRunRequest): Promise<BiQueryResponse> {
    return this.mutate("/api/v1/bi/sql", "POST", {
      sql_text: input.sqlText,
      limit: input.limit,
    });
  }

  async getBiSavedQueries(): Promise<ListResponse<SavedSqlQuery>> {
    return this.query("/api/v1/bi/sql-queries");
  }

  async getBiSavedQuery(queryId: string): Promise<SavedSqlQuery> {
    return this.query(`/api/v1/bi/sql-queries/${encodeURIComponent(queryId)}`);
  }

  async createBiSavedQuery(input: SavedSqlInput): Promise<SavedSqlQuery> {
    return this.mutate("/api/v1/bi/sql-queries", "POST", savedSqlPayload(input));
  }

  async updateBiSavedQuery(queryId: string, input: SavedSqlInput): Promise<SavedSqlQuery> {
    return this.mutate(`/api/v1/bi/sql-queries/${encodeURIComponent(queryId)}`, "PUT", savedSqlPayload(input));
  }

  async deleteBiSavedQuery(queryId: string): Promise<void> {
    return this.mutate(`/api/v1/bi/sql-queries/${encodeURIComponent(queryId)}`, "DELETE");
  }

  async runBiSavedQuery(queryId: string, params?: { limit?: number }): Promise<BiQueryResponse> {
    return this.mutate(`/api/v1/bi/sql-queries/${encodeURIComponent(queryId)}/run${params?.limit ? `?limit=${encodeURIComponent(String(params.limit))}` : ""}`, "POST");
  }

  async runBiVisualQuery(input: BiVisualQueryRequest): Promise<BiQueryResponse> {
    return this.mutate("/api/v1/bi/visual-query", "POST", {
      dataset: input.dataset,
      dimensions: input.dimensions || [],
      metrics: input.metrics || [],
      start_at: input.startAt ? toISO(input.startAt) : undefined,
      end_at: input.endAt ? toISO(input.endAt) : undefined,
      limit: input.limit,
    });
  }

  async runBiDrillThrough(input: BiDrillThroughRequest): Promise<BiQueryResponse> {
    return this.mutate("/api/v1/bi/drill-through", "POST", {
      dataset: input.dataset,
      filters: input.filters || {},
      start_at: input.startAt ? toISO(input.startAt) : undefined,
      end_at: input.endAt ? toISO(input.endAt) : undefined,
      limit: input.limit,
    });
  }

  async getBiQueryRuns(params?: { limit?: number; offset?: number }): Promise<ListResponse<BiQueryRun>> {
    return this.query("/api/v1/bi/query-runs", {
      ...(params?.limit ? { limit: String(params.limit) } : {}),
      ...(params?.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async getCsvUploads(): Promise<ListResponse<CsvUpload>> {
    return this.query("/api/v1/bi/csv-uploads");
  }

  async createCsvUpload(input: CsvUploadInput): Promise<CsvUpload> {
    return this.mutate("/api/v1/bi/csv-uploads", "POST", csvUploadPayload(input));
  }

  async getCsvUploadRows(uploadId: string, params?: { limit?: number; offset?: number }): Promise<ListResponse<Record<string, unknown>>> {
    return this.query(`/api/v1/bi/csv-uploads/${encodeURIComponent(uploadId)}/rows`, {
      ...(params?.limit ? { limit: String(params.limit) } : {}),
      ...(params?.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async deleteCsvUpload(uploadId: string): Promise<void> {
    return this.mutate(`/api/v1/bi/csv-uploads/${encodeURIComponent(uploadId)}`, "DELETE");
  }

  async getIntegrations(filter?: IntegrationFilter): Promise<ListResponse<Integration>> {
    return this.query("/api/v1/integrations", {
      ...(filter?.category ? { category: filter.category } : {}),
      ...(filter?.capability ? { capability: filter.capability } : {}),
      ...(filter?.status ? { status: filter.status } : {}),
    });
  }

  async getIntegration(key: string): Promise<Integration> {
    return this.query(`/api/v1/integrations/${encodeURIComponent(key)}`);
  }

  async getSources(): Promise<ListResponse<EventSource>> {
    return this.query("/api/v1/sources");
  }

  async getSource(sourceId: string): Promise<EventSource> {
    return this.query(`/api/v1/sources/${encodeURIComponent(sourceId)}`);
  }

  async createSource(input: SourceInput): Promise<SourceWithToken> {
    return this.mutate("/api/v1/sources", "POST", sourcePayload(input));
  }

  async updateSource(sourceId: string, input: SourceInput): Promise<EventSource> {
    return this.mutate(`/api/v1/sources/${encodeURIComponent(sourceId)}`, "PUT", sourcePayload(input));
  }

  async deleteSource(sourceId: string): Promise<void> {
    return this.mutate(`/api/v1/sources/${encodeURIComponent(sourceId)}`, "DELETE");
  }

  async getSourceIngestions(sourceId: string, params?: { limit?: number; offset?: number }): Promise<ListResponse<SourceIngestion>> {
    return this.query(`/api/v1/sources/${encodeURIComponent(sourceId)}/ingestions`, {
      ...(params?.limit ? { limit: String(params.limit) } : {}),
      ...(params?.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async ingestSourceWebhook(sourceId: string, token: string, payload: Record<string, unknown>): Promise<SourceIngestResponse> {
    const res = await fetch(`${this.config.apiUrl}/api/source/${encodeURIComponent(sourceId)}/collect`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Pulse-Source-Token": token,
      },
      body: JSON.stringify(payload),
      keepalive: true,
    });
    if (!res.ok) throw new Error(`Pulse API error: ${res.status}`);
    return res.json();
  }

  async getDestinations(): Promise<ListResponse<Destination>> {
    return this.query("/api/v1/destinations");
  }

  async getDestination(destinationId: string): Promise<Destination> {
    return this.query(`/api/v1/destinations/${encodeURIComponent(destinationId)}`);
  }

  async createDestination(input: DestinationInput): Promise<Destination> {
    return this.mutate("/api/v1/destinations", "POST", destinationPayload(input));
  }

  async updateDestination(destinationId: string, input: DestinationInput): Promise<Destination> {
    return this.mutate(`/api/v1/destinations/${encodeURIComponent(destinationId)}`, "PUT", destinationPayload(input));
  }

  async deleteDestination(destinationId: string): Promise<void> {
    return this.mutate(`/api/v1/destinations/${encodeURIComponent(destinationId)}`, "DELETE");
  }

  async getDestinationDeliveries(params?: { status?: string; limit?: number; offset?: number }): Promise<ListResponse<DestinationDelivery>> {
    return this.query("/api/v1/destination-deliveries", {
      ...(params?.status ? { status: params.status } : {}),
      ...(params?.limit ? { limit: String(params.limit) } : {}),
      ...(params?.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async retryDestinationDelivery(deliveryId: string): Promise<{ ok: boolean }> {
    return this.mutate(`/api/v1/destination-deliveries/${encodeURIComponent(deliveryId)}/retry`, "POST");
  }

  async getDestinationHealth(): Promise<ListResponse<DestinationHealth>> {
    return this.query("/api/v1/destination-health");
  }

  async askAiQuery(params: {
    question: string;
    startAt?: string | Date;
    endAt?: string | Date;
    limit?: number;
  }): Promise<AiQueryResponse> {
    return this.mutate("/api/v1/ai/query", "POST", {
      question: params.question,
      start_at: params.startAt ? toISO(params.startAt) : undefined,
      end_at: params.endAt ? toISO(params.endAt) : undefined,
      limit: params.limit,
    });
  }

  async getAiInsights(params?: { startAt?: string | Date; endAt?: string | Date }): Promise<ListResponse<AiInsight>> {
    return this.query("/api/v1/ai/insights", {
      ...(params?.startAt ? { start_at: toISO(params.startAt) } : {}),
      ...(params?.endAt ? { end_at: toISO(params.endAt) } : {}),
    });
  }

  async getAiQueryHistory(params?: { limit?: number; offset?: number }): Promise<ListResponse<AiQueryRun>> {
    return this.query("/api/v1/ai/history", {
      ...(params?.limit ? { limit: String(params.limit) } : {}),
      ...(params?.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async getLlmTraces(params?: { limit?: number; offset?: number }): Promise<ListResponse<LlmTrace>> {
    return this.query("/api/v1/ai/llm/traces", {
      ...(params?.limit ? { limit: String(params.limit) } : {}),
      ...(params?.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async recordLlmTrace(input: LlmTraceInput): Promise<LlmTrace> {
    return this.mutate("/api/v1/ai/llm/traces", "POST", llmTracePayload(input));
  }

  async getLlmTrace(traceId: string): Promise<LlmTrace> {
    return this.query(`/api/v1/ai/llm/traces/${encodeURIComponent(traceId)}`);
  }

  async getLlmGenerations(params?: { limit?: number; offset?: number }): Promise<ListResponse<LlmGeneration>> {
    return this.query("/api/v1/ai/llm/generations", {
      ...(params?.limit ? { limit: String(params.limit) } : {}),
      ...(params?.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async recordLlmGeneration(input: LlmGenerationInput): Promise<LlmGeneration> {
    return this.mutate("/api/v1/ai/llm/generations", "POST", llmGenerationPayload(input));
  }

  async getLlmEvaluations(params?: { limit?: number; offset?: number }): Promise<ListResponse<LlmEvaluation>> {
    return this.query("/api/v1/ai/llm/evaluations", {
      ...(params?.limit ? { limit: String(params.limit) } : {}),
      ...(params?.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async recordLlmEvaluation(input: LlmEvaluationInput): Promise<LlmEvaluation> {
    return this.mutate("/api/v1/ai/llm/evaluations", "POST", llmEvaluationPayload(input));
  }

  async getLlmStats(params: StatsQuery): Promise<LlmStats> {
    return this.query("/api/v1/ai/llm/stats", {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
    });
  }

  async getAlerts(): Promise<ListResponse<AlertRule>> {
    return this.query("/api/v1/alerts");
  }

  async createAlert(input: AlertInput): Promise<AlertRule> {
    return this.mutate("/api/v1/alerts", "POST", input);
  }

  async updateAlert(alertId: string, input: AlertInput): Promise<AlertRule> {
    return this.mutate(`/api/v1/alerts/${encodeURIComponent(alertId)}`, "PUT", input);
  }

  async deleteAlert(alertId: string): Promise<void> {
    return this.mutate(`/api/v1/alerts/${encodeURIComponent(alertId)}`, "DELETE");
  }

  async toggleAlert(alertId: string): Promise<AlertRule> {
    return this.mutate(`/api/v1/alerts/${encodeURIComponent(alertId)}/toggle`, "POST");
  }

  async getExperiments(): Promise<ListResponse<Experiment>> {
    return this.query("/api/v1/experiments");
  }

  async createExperiment(input: ExperimentInput): Promise<Experiment> {
    return this.mutate("/api/v1/experiments", "POST", experimentPayload(input));
  }

  async getExperiment(experimentId: string): Promise<Experiment> {
    return this.query(`/api/v1/experiments/${encodeURIComponent(experimentId)}`);
  }

  async updateExperimentStatus(experimentId: string, status: string): Promise<Experiment> {
    return this.mutate(`/api/v1/experiments/${encodeURIComponent(experimentId)}/status`, "PUT", { status });
  }

  async deleteExperiment(experimentId: string): Promise<void> {
    return this.mutate(`/api/v1/experiments/${encodeURIComponent(experimentId)}`, "DELETE");
  }

  async getExperimentResults(experimentId: string, params: StatsQuery): Promise<ExperimentResults> {
    return this.query(`/api/v1/experiments/${encodeURIComponent(experimentId)}/results`, {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
    });
  }

  async assignExperiment(experimentId: string): Promise<ExperimentAssignment> {
    const res = await fetch(`${this.config.apiUrl}/api/v1/experiments/${encodeURIComponent(experimentId)}/assign`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Pulse-Key": this.config.apiKey,
      },
      body: JSON.stringify({ visitor_id: this.visitorId }),
      keepalive: true,
    });
    if (!res.ok) throw new Error(`Pulse API error: ${res.status}`);
    return res.json();
  }

  async getActiveSurveys(): Promise<ListResponse<Survey>> {
    return this.query("/api/v1/surveys/active");
  }

  async getSurveyNps(surveyId: string, params?: { questionId?: string }): Promise<SurveyNpsReport> {
    return this.query(`/api/v1/surveys/${encodeURIComponent(surveyId)}/nps`, {
      ...(params?.questionId ? { question_id: params.questionId } : {}),
    });
  }

  async getSurveySentiment(surveyId: string, params?: { questionId?: string }): Promise<SurveySentimentReport> {
    return this.query(`/api/v1/surveys/${encodeURIComponent(surveyId)}/sentiment`, {
      ...(params?.questionId ? { question_id: params.questionId } : {}),
    });
  }

  async getGuides(): Promise<ListResponse<InAppGuide>> {
    return this.query("/api/v1/guides");
  }

  async getActiveGuides(): Promise<ListResponse<InAppGuide>> {
    return this.query("/api/v1/guides/active");
  }

  async getGuide(guideId: string): Promise<InAppGuide> {
    return this.query(`/api/v1/guides/${encodeURIComponent(guideId)}`);
  }

  async createGuide(input: GuideInput): Promise<InAppGuide> {
    return this.mutate("/api/v1/guides", "POST", guidePayload(input));
  }

  async updateGuide(guideId: string, input: GuideInput): Promise<InAppGuide> {
    return this.mutate(`/api/v1/guides/${encodeURIComponent(guideId)}`, "PUT", guidePayload(input));
  }

  async deleteGuide(guideId: string): Promise<void> {
    return this.mutate(`/api/v1/guides/${encodeURIComponent(guideId)}`, "DELETE");
  }

  async updateGuideStatus(guideId: string, status: string): Promise<InAppGuide> {
    return this.mutate(`/api/v1/guides/${encodeURIComponent(guideId)}/status`, "PUT", { status });
  }

  async recordGuideEvent(guideId: string, input: GuideEventInput): Promise<GuideEvent> {
    return this.mutate(`/api/v1/guides/${encodeURIComponent(guideId)}/events`, "POST", guideEventPayload(input));
  }

  async getGuideEvents(guideId: string, params?: { limit?: number; offset?: number }): Promise<ListResponse<GuideEvent>> {
    return this.query(`/api/v1/guides/${encodeURIComponent(guideId)}/events`, {
      ...(params?.limit ? { limit: String(params.limit) } : {}),
      ...(params?.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async getGuideStats(guideId: string): Promise<GuideStats> {
    return this.query(`/api/v1/guides/${encodeURIComponent(guideId)}/stats`);
  }
}

export function createPulse(config: PulseConfig): PulseClient {
  return new PulseClient(config);
}

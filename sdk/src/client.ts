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
  Funnel,
  FunnelResult,
  Goal,
  GoalStats,
  RetentionCohort,
  CohortGroup,
  PageFlow,
  CampaignStat,
  WebVitalSummary,
  ErrorGroup,
  AlertRule,
  Experiment,
  Survey,
  SharedDashboard,
} from "./types";

const DEFAULT_API_URL = "https://pulse.ayushojha.com";

function toISO(d: string | Date): string {
  return d instanceof Date ? d.toISOString() : d;
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

export class PulseClient {
  private config: Required<PulseConfig>;
  private visitorId: string;
  private maxScroll = 0;
  private lastPath = "";

  constructor(config: PulseConfig) {
    this.config = {
      apiKey: config.apiKey,
      apiUrl: config.apiUrl || DEFAULT_API_URL,
      autoTrack: config.autoTrack ?? (typeof window !== "undefined"),
      respectDnt: config.respectDnt ?? true,
      debug: config.debug ?? false,
      trackUtm: config.trackUtm ?? true,
      trackScrollDepth: config.trackScrollDepth ?? false,
      trackWebVitals: config.trackWebVitals ?? false,
      trackOutlinks: config.trackOutlinks ?? false,
      trackErrors: config.trackErrors ?? false,
      trackClicks: config.trackClicks ?? false,
      trackSearch: config.trackSearch ?? false,
      searchParam: config.searchParam ?? "q",
    };
    this.visitorId = generateVisitorId();

    if (typeof window !== "undefined") {
      if (this.config.autoTrack) this.setupAutoTracking();
      if (this.config.trackScrollDepth) this.setupScrollTracking();
      if (this.config.trackWebVitals) this.setupWebVitals();
      if (this.config.trackOutlinks) this.setupOutlinkTracking();
      if (this.config.trackErrors) this.setupErrorTracking();
      if (this.config.trackClicks) this.setupClickTracking();
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

    const body = JSON.stringify({
      type,
      payload,
      visitor_id: this.visitorId,
    });

    const url = `${this.config.apiUrl}/api/collect`;
    this.log("send", type, payload);

    try {
      if (typeof navigator !== "undefined" && navigator.sendBeacon) {
        navigator.sendBeacon(`${url}?key=${this.config.apiKey}`, body);
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
      });
    });
    window.addEventListener("unhandledrejection", (ev) => {
      const r = ev.reason;
      this.send("js_error", {
        message: r?.message || String(r) || "Unhandled promise rejection",
        stack: r?.stack,
        path: location.pathname,
      });
    });
  }

  private setupClickTracking() {
    document.addEventListener("click", (ev) => {
      const target = ev.target as Element;
      let selector = "";
      try {
        selector = target.tagName?.toLowerCase() || "";
        if (target.id) selector += "#" + target.id;
        else if (target.className && typeof target.className === "string") {
          selector += "." + target.className.trim().split(/\s+/).slice(0, 2).join(".");
        }
      } catch {}
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

  // --- Core tracking methods ---

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

  identify(traits: Record<string, unknown>) {
    this.send("identify", { traits });
  }

  searchQuery(query: string, resultsCount?: number) {
    this.send("search_query", { query, results_count: resultsCount, path: typeof location !== "undefined" ? location.pathname : undefined });
  }

  surveyResponse(surveyId: string, answers: unknown[], completed = true) {
    this.send("survey_response", { survey_id: surveyId, answers, completed, path: typeof location !== "undefined" ? location.pathname : undefined });
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

  async getWebVitals(params: StatsQuery): Promise<ListResponse<WebVitalSummary>> {
    return this.query("/api/v1/webvitals", { start_at: toISO(params.startAt), end_at: toISO(params.endAt) });
  }

  async getErrors(params: PaginatedQuery): Promise<ListResponse<ErrorGroup>> {
    return this.query("/api/v1/errors", { start_at: toISO(params.startAt), end_at: toISO(params.endAt), ...(params.limit ? { limit: String(params.limit) } : {}) });
  }

  async getActiveSurveys(): Promise<ListResponse<Survey>> {
    return this.query("/api/v1/surveys/active");
  }
}

export function createPulse(config: PulseConfig): PulseClient {
  return new PulseClient(config);
}

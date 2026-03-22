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

function toISO(d: string | Date): string {
  return d instanceof Date ? d.toISOString() : d;
}

const DEFAULT_API_URL = "https://pulse.ayushojha.com";

export class PulseServerClient {
  private apiKey: string;
  private apiUrl: string;

  constructor(config: PulseConfig) {
    this.apiKey = config.apiKey;
    this.apiUrl = config.apiUrl || DEFAULT_API_URL;
  }

  private async request<T>(path: string, options?: RequestInit & { params?: Record<string, string> }): Promise<T> {
    const url = new URL(`${this.apiUrl}${path}`);
    if (options?.params) {
      Object.entries(options.params).forEach(([k, v]) => url.searchParams.set(k, v));
    }
    const res = await fetch(url.toString(), {
      ...options,
      headers: {
        "X-Pulse-Key": this.apiKey,
        "Content-Type": "application/json",
        ...options?.headers,
      },
    });
    if (!res.ok) throw new Error(`Pulse API error: ${res.status}`);
    return res.json();
  }

  private collect(type: string, payload: Record<string, unknown>, visitorId: string, headers?: Record<string, string>) {
    return this.request("/api/collect", {
      method: "POST",
      headers: headers || {},
      body: JSON.stringify({ type, payload, visitor_id: visitorId }),
    });
  }

  // --- Ingestion methods ---

  async trackPageview(params: {
    visitorId: string;
    path: string;
    title?: string;
    referrer?: string;
    utmSource?: string;
    utmMedium?: string;
    utmCampaign?: string;
    utmContent?: string;
    utmTerm?: string;
    ip?: string;
    userAgent?: string;
  }) {
    const headers: Record<string, string> = {};
    if (params.ip) headers["X-Forwarded-For"] = params.ip;
    if (params.userAgent) headers["User-Agent"] = params.userAgent;
    return this.collect("pageview", {
      path: params.path,
      title: params.title,
      referrer: params.referrer,
      utm_source: params.utmSource,
      utm_medium: params.utmMedium,
      utm_campaign: params.utmCampaign,
      utm_content: params.utmContent,
      utm_term: params.utmTerm,
    }, params.visitorId, headers);
  }

  async trackEvent(params: {
    visitorId: string;
    eventName: string;
    data?: Record<string, unknown>;
    path?: string;
    revenueAmount?: number;
    revenueCurrency?: string;
    ip?: string;
    userAgent?: string;
  }) {
    const headers: Record<string, string> = {};
    if (params.ip) headers["X-Forwarded-For"] = params.ip;
    if (params.userAgent) headers["User-Agent"] = params.userAgent;
    return this.collect("event", {
      name: params.eventName,
      data: params.data || {},
      path: params.path,
      revenue_amount: params.revenueAmount,
      revenue_currency: params.revenueCurrency,
    }, params.visitorId, headers);
  }

  async trackWebVital(params: { visitorId: string; name: string; value: number; rating?: string; path?: string }) {
    return this.collect("web_vital", { name: params.name, value: params.value, rating: params.rating, path: params.path }, params.visitorId);
  }

  async trackError(params: { visitorId: string; message: string; stack?: string; filename?: string; lineno?: number; colno?: number; path?: string }) {
    return this.collect("js_error", params, params.visitorId);
  }

  async trackSearchQuery(params: { visitorId: string; query: string; resultsCount?: number; path?: string }) {
    return this.collect("search_query", { query: params.query, results_count: params.resultsCount, path: params.path }, params.visitorId);
  }

  async trackSurveyResponse(params: { visitorId: string; surveyId: string; answers: unknown[]; completed?: boolean; path?: string }) {
    return this.collect("survey_response", { survey_id: params.surveyId, answers: params.answers, completed: params.completed !== false, path: params.path }, params.visitorId);
  }

  // --- Core query methods ---

  async getStats(params: StatsQuery): Promise<StatsResponse> {
    return this.request("/api/v1/stats", { params: { start_at: toISO(params.startAt), end_at: toISO(params.endAt) } });
  }

  async getPages(params: PaginatedQuery): Promise<ListResponse<PageData>> {
    return this.request("/api/v1/pages", { params: { start_at: toISO(params.startAt), end_at: toISO(params.endAt), ...(params.limit ? { limit: String(params.limit) } : {}) } });
  }

  async getReferrers(params: PaginatedQuery): Promise<ListResponse<ReferrerData>> {
    return this.request("/api/v1/referrers", { params: { start_at: toISO(params.startAt), end_at: toISO(params.endAt), ...(params.limit ? { limit: String(params.limit) } : {}) } });
  }

  async getEvents(params: PaginatedQuery): Promise<ListResponse<EventData>> {
    return this.request("/api/v1/events", { params: { start_at: toISO(params.startAt), end_at: toISO(params.endAt), ...(params.limit ? { limit: String(params.limit) } : {}) } });
  }

  async getDevices(params: PaginatedQuery): Promise<ListResponse<DeviceData>> {
    return this.request("/api/v1/devices", { params: { start_at: toISO(params.startAt), end_at: toISO(params.endAt), ...(params.limit ? { limit: String(params.limit) } : {}) } });
  }

  async getGeo(params: PaginatedQuery): Promise<ListResponse<GeoData>> {
    return this.request("/api/v1/geo", { params: { start_at: toISO(params.startAt), end_at: toISO(params.endAt), ...(params.limit ? { limit: String(params.limit) } : {}) } });
  }

  async getRealtime(): Promise<RealtimeResponse> {
    return this.request("/api/v1/realtime");
  }

  // --- Module query methods ---

  async getFunnels(): Promise<ListResponse<Funnel>> { return this.request("/api/v1/funnels"); }
  async analyzeFunnel(funnelId: string, params: StatsQuery): Promise<ListResponse<FunnelResult>> {
    return this.request(`/api/v1/funnels/${funnelId}/analyze`, { params: { start_at: toISO(params.startAt), end_at: toISO(params.endAt) } });
  }
  async getGoals(): Promise<ListResponse<Goal>> { return this.request("/api/v1/goals"); }
  async getGoalStats(goalId: string, params: StatsQuery): Promise<GoalStats> {
    return this.request(`/api/v1/goals/${goalId}/stats`, { params: { start_at: toISO(params.startAt), end_at: toISO(params.endAt) } });
  }
  async getRetention(params: StatsQuery & { period?: string }): Promise<{ cohorts: RetentionCohort[] }> {
    return this.request("/api/v1/retention", { params: { start_at: toISO(params.startAt), end_at: toISO(params.endAt), ...(params.period ? { period: params.period } : {}) } });
  }
  async getCohorts(params: StatsQuery & { groupBy?: string; metric?: string }): Promise<ListResponse<CohortGroup>> {
    return this.request("/api/v1/cohorts", { params: { start_at: toISO(params.startAt), end_at: toISO(params.endAt), ...(params.groupBy ? { group_by: params.groupBy } : {}), ...(params.metric ? { metric: params.metric } : {}) } });
  }
  async getPaths(params: StatsQuery & { path: string; direction?: string }): Promise<ListResponse<PageFlow>> {
    return this.request("/api/v1/paths", { params: { start_at: toISO(params.startAt), end_at: toISO(params.endAt), path: params.path, ...(params.direction ? { direction: params.direction } : {}) } });
  }
  async getCampaigns(params: StatsQuery): Promise<ListResponse<CampaignStat>> {
    return this.request("/api/v1/campaigns", { params: { start_at: toISO(params.startAt), end_at: toISO(params.endAt) } });
  }
  async getWebVitals(params: StatsQuery): Promise<ListResponse<WebVitalSummary>> {
    return this.request("/api/v1/webvitals", { params: { start_at: toISO(params.startAt), end_at: toISO(params.endAt) } });
  }
  async getErrors(params: PaginatedQuery): Promise<ListResponse<ErrorGroup>> {
    return this.request("/api/v1/errors", { params: { start_at: toISO(params.startAt), end_at: toISO(params.endAt), ...(params.limit ? { limit: String(params.limit) } : {}) } });
  }
  async getAlerts(): Promise<ListResponse<AlertRule>> { return this.request("/api/v1/alerts"); }
  async getExperiments(): Promise<ListResponse<Experiment>> { return this.request("/api/v1/experiments"); }
  async getActiveSurveys(): Promise<ListResponse<Survey>> { return this.request("/api/v1/surveys/active"); }
}

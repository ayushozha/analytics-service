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
  SessionReplayEvent,
  EmailReportConfig,
  EmailReportListResponse,
  DsarDeleteResult,
  AuditLog,
  PrivacySettings,
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
    metadata: input.metadata || {},
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
    metadata: input.metadata ?? {},
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

const DEFAULT_API_URL = "https://pulse.ayushojha.com";

type CollectPrivacy = {
  consentMode?: string;
  consentGranted?: boolean;
};

export class PulseServerClient {
  private apiKey: string;
  private apiUrl: string;
  private consentMode: string;
  private consentGranted: boolean;
  private release: string;
  private environment: string;

  constructor(config: PulseConfig) {
    this.apiKey = config.apiKey;
    this.apiUrl = config.apiUrl || DEFAULT_API_URL;
    this.consentMode = config.consentMode ?? "analytics";
    this.consentGranted = config.consentGranted ?? true;
    this.release = config.release ?? "";
    this.environment = config.environment ?? "production";
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
    const text = await res.text();
    return (text ? JSON.parse(text) : undefined) as T;
  }

  private collect(
    type: string,
    payload: Record<string, unknown>,
    visitorId: string,
    headers?: Record<string, string>,
    privacy?: CollectPrivacy,
  ) {
    return this.request("/api/collect", {
      method: "POST",
      headers: headers || {},
      body: JSON.stringify({
        type,
        payload,
        visitor_id: visitorId,
        consent_mode: privacy?.consentMode ?? this.consentMode,
        consent_granted: privacy?.consentGranted ?? this.consentGranted,
      }),
    });
  }

  // --- Ingestion methods ---

  setConsent(granted: boolean, mode = "analytics") {
    this.consentGranted = granted;
    this.consentMode = mode;
  }

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
    consentMode?: string;
    consentGranted?: boolean;
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
    }, params.visitorId, headers, {
      consentMode: params.consentMode,
      consentGranted: params.consentGranted,
    });
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
    consentMode?: string;
    consentGranted?: boolean;
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
    }, params.visitorId, headers, {
      consentMode: params.consentMode,
      consentGranted: params.consentGranted,
    });
  }

  async identify(params: {
    visitorId: string;
    userId?: string;
    traits: Record<string, unknown>;
    account?: IdentifyAccountOptions;
    ip?: string;
    userAgent?: string;
    consentMode?: string;
    consentGranted?: boolean;
  }) {
    const headers: Record<string, string> = {};
    if (params.ip) headers["X-Forwarded-For"] = params.ip;
    if (params.userAgent) headers["User-Agent"] = params.userAgent;
    return this.collect("identify", {
      user_id: params.userId,
      traits: params.traits,
      account_id: params.account?.accountId,
      account_name: params.account?.accountName,
      account_traits: params.account?.accountTraits,
      account_role: params.account?.accountRole,
    }, params.visitorId, headers, {
      consentMode: params.consentMode,
      consentGranted: params.consentGranted,
    });
  }

  async trackWebVital(params: { visitorId: string; name: string; value: number; rating?: string; path?: string; consentMode?: string; consentGranted?: boolean }) {
    return this.collect("web_vital", { name: params.name, value: params.value, rating: params.rating, path: params.path }, params.visitorId, undefined, {
      consentMode: params.consentMode,
      consentGranted: params.consentGranted,
    });
  }

  async trackError(params: { visitorId: string; message: string; stack?: string; filename?: string; lineno?: number; colno?: number; path?: string; release?: string; environment?: string; consentMode?: string; consentGranted?: boolean }) {
    const { visitorId, consentMode, consentGranted, release, environment, ...payload } = params;
    return this.collect("js_error", {
      ...payload,
      release: (release ?? this.release) || undefined,
      environment: environment ?? this.environment,
    }, visitorId, undefined, {
      consentMode,
      consentGranted,
    });
  }

  async trackLog(params: {
    visitorId: string;
    level: "trace" | "debug" | "info" | "warn" | "error" | "fatal" | string;
    message: string;
    body?: Record<string, unknown>;
    path?: string;
    release?: string;
    environment?: string;
    ip?: string;
    userAgent?: string;
    consentMode?: string;
    consentGranted?: boolean;
  }) {
    const headers: Record<string, string> = {};
    if (params.ip) headers["X-Forwarded-For"] = params.ip;
    if (params.userAgent) headers["User-Agent"] = params.userAgent;
    return this.collect("log", {
      level: params.level,
      message: params.message,
      body: params.body || {},
      path: params.path,
      release: (params.release ?? this.release) || undefined,
      environment: params.environment ?? this.environment,
    }, params.visitorId, headers, {
      consentMode: params.consentMode,
      consentGranted: params.consentGranted,
    });
  }

  async trackSearchQuery(params: { visitorId: string; query: string; resultsCount?: number; path?: string; consentMode?: string; consentGranted?: boolean }) {
    return this.collect("search_query", { query: params.query, results_count: params.resultsCount, path: params.path }, params.visitorId, undefined, {
      consentMode: params.consentMode,
      consentGranted: params.consentGranted,
    });
  }

  async trackSurveyResponse(params: { visitorId: string; surveyId: string; answers: unknown[]; completed?: boolean; path?: string; consentMode?: string; consentGranted?: boolean }) {
    return this.collect("survey_response", { survey_id: params.surveyId, answers: params.answers, completed: params.completed !== false, path: params.path }, params.visitorId, undefined, {
      consentMode: params.consentMode,
      consentGranted: params.consentGranted,
    });
  }

  async trackSessionReplay(params: {
    visitorId: string;
    events: SessionReplayEvent[];
    startedAt?: Date | number;
    durationMs?: number;
    entryPage?: string;
    screen?: string;
    isComplete?: boolean;
    ip?: string;
    userAgent?: string;
    consentMode?: string;
    consentGranted?: boolean;
  }) {
    const headers: Record<string, string> = {};
    if (params.ip) headers["X-Forwarded-For"] = params.ip;
    if (params.userAgent) headers["User-Agent"] = params.userAgent;
    const startedAt =
      params.startedAt instanceof Date ? params.startedAt.getTime() : params.startedAt;
    return this.collect("session_replay", {
      events: params.events,
      started_at: startedAt,
      duration_ms: params.durationMs,
      entry_page: params.entryPage,
      screen: params.screen,
      is_complete: params.isComplete,
    }, params.visitorId, headers, {
      consentMode: params.consentMode,
      consentGranted: params.consentGranted,
    });
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

  async getUserProfiles(params?: { limit?: number; offset?: number }): Promise<ListResponse<UserProfile>> {
    return this.request("/api/v1/identity/users", {
      params: {
        ...(params?.limit ? { limit: String(params.limit) } : {}),
        ...(params?.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }

  async getUserProfile(visitorId: string): Promise<UserProfile> {
    return this.request(`/api/v1/identity/users/${encodeURIComponent(visitorId)}`);
  }

  async getUserAliases(userId: string): Promise<ListResponse<UserAlias>> {
    return this.request(`/api/v1/identity/aliases/${encodeURIComponent(userId)}`);
  }
  async getIdentityGraph(params: IdentityGraphQuery): Promise<IdentityGraph> {
    return this.request("/api/v1/identity/graph", {
      params: {
        ...(params.visitorId ? { visitor_id: params.visitorId } : {}),
        ...(params.userId ? { user_id: params.userId } : {}),
        ...(params.accountId ? { account_id: params.accountId } : {}),
        ...(params.limit ? { limit: String(params.limit) } : {}),
      },
    });
  }
  async getAccountProfiles(params?: { limit?: number; offset?: number }): Promise<ListResponse<AccountProfile>> {
    return this.request("/api/v1/identity/accounts", {
      params: {
        ...(params?.limit ? { limit: String(params.limit) } : {}),
        ...(params?.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }
  async getAccountProfile(accountId: string): Promise<AccountProfile> {
    return this.request(`/api/v1/identity/accounts/${encodeURIComponent(accountId)}`);
  }
  async getAccountMembers(accountId: string, params?: { limit?: number; offset?: number }): Promise<ListResponse<AccountMembership>> {
    return this.request(`/api/v1/identity/accounts/${encodeURIComponent(accountId)}/members`, {
      params: {
        ...(params?.limit ? { limit: String(params.limit) } : {}),
        ...(params?.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }
  async getAccountAnalytics(accountId: string, params?: { startAt?: string | Date; endAt?: string | Date }): Promise<AccountAnalytics> {
    return this.request(`/api/v1/identity/accounts/${encodeURIComponent(accountId)}/analytics`, {
      params: {
        ...(params?.startAt ? { start_at: toISO(params.startAt) } : {}),
        ...(params?.endAt ? { end_at: toISO(params.endAt) } : {}),
      },
    });
  }

  async getScimUsers(params?: { limit?: number; offset?: number }): Promise<ListResponse<ScimUser>> {
    return this.request("/api/v1/scim/users", {
      params: {
        ...(params?.limit ? { limit: String(params.limit) } : {}),
        ...(params?.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }
  async getScimUser(userId: string): Promise<ScimUser> {
    return this.request(`/api/v1/scim/users/${encodeURIComponent(userId)}`);
  }
  async createScimUser(input: ScimUserInput): Promise<ScimUser> {
    return this.request("/api/v1/scim/users", {
      method: "POST",
      body: JSON.stringify(scimUserPayload(input)),
    });
  }
  async updateScimUser(userId: string, input: ScimUserInput): Promise<ScimUser> {
    return this.request(`/api/v1/scim/users/${encodeURIComponent(userId)}`, {
      method: "PUT",
      body: JSON.stringify(scimUserPayload(input)),
    });
  }
  async deleteScimUser(userId: string): Promise<void> {
    await this.request(`/api/v1/scim/users/${encodeURIComponent(userId)}`, {
      method: "DELETE",
    });
  }
  async getScimGroups(params?: { limit?: number; offset?: number }): Promise<ListResponse<ScimGroup>> {
    return this.request("/api/v1/scim/groups", {
      params: {
        ...(params?.limit ? { limit: String(params.limit) } : {}),
        ...(params?.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }
  async getScimGroup(groupId: string): Promise<ScimGroupWithMembers> {
    return this.request(`/api/v1/scim/groups/${encodeURIComponent(groupId)}`);
  }
  async createScimGroup(input: ScimGroupInput): Promise<ScimGroupWithMembers> {
    return this.request("/api/v1/scim/groups", {
      method: "POST",
      body: JSON.stringify(scimGroupPayload(input)),
    });
  }
  async updateScimGroup(groupId: string, input: ScimGroupInput): Promise<ScimGroupWithMembers> {
    return this.request(`/api/v1/scim/groups/${encodeURIComponent(groupId)}`, {
      method: "PUT",
      body: JSON.stringify(scimGroupPayload(input)),
    });
  }
  async deleteScimGroup(groupId: string): Promise<void> {
    await this.request(`/api/v1/scim/groups/${encodeURIComponent(groupId)}`, {
      method: "DELETE",
    });
  }

  async getSessionRecordings(params: PaginatedQuery): Promise<ListResponse<SessionRecordingSummary>> {
    return this.request("/api/v1/session-replay", {
      params: {
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
        ...(params.limit ? { limit: String(params.limit) } : {}),
        ...(params.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }

  async getSessionRecording(recordingId: string): Promise<SessionRecording> {
    return this.request(`/api/v1/session-replay/${encodeURIComponent(recordingId)}`);
  }

  async getEmailReports(): Promise<EmailReportListResponse> {
    return this.request("/api/v1/email-reports");
  }

  async createEmailReport(params: {
    name: string;
    recipients: string[];
    schedule: "daily" | "weekly" | "monthly";
    modules?: string[];
    isActive?: boolean;
  }): Promise<EmailReportConfig> {
    return this.request("/api/v1/email-reports", {
      method: "POST",
      body: JSON.stringify({
        name: params.name,
        recipients: params.recipients,
        schedule: params.schedule,
        modules: params.modules || [],
        is_active: params.isActive ?? true,
      }),
    });
  }

  async updateEmailReport(reportId: string, params: {
    name: string;
    recipients: string[];
    schedule: "daily" | "weekly" | "monthly";
    modules?: string[];
    isActive?: boolean;
  }): Promise<EmailReportConfig> {
    return this.request(`/api/v1/email-reports/${encodeURIComponent(reportId)}`, {
      method: "PUT",
      body: JSON.stringify({
        name: params.name,
        recipients: params.recipients,
        schedule: params.schedule,
        modules: params.modules || [],
        is_active: params.isActive ?? true,
      }),
    });
  }

  async deleteEmailReport(reportId: string): Promise<void> {
    await this.request(`/api/v1/email-reports/${encodeURIComponent(reportId)}`, {
      method: "DELETE",
    });
  }

  async sendTestEmailReport(reportId: string): Promise<{ ok: true }> {
    return this.request(`/api/v1/email-reports/${encodeURIComponent(reportId)}/test`, {
      method: "POST",
    });
  }

  async exportVisitorData(visitorId: string): Promise<Record<string, unknown>> {
    return this.request(`/api/v1/privacy/users/${encodeURIComponent(visitorId)}/export`);
  }

  async deleteVisitorData(visitorId: string): Promise<DsarDeleteResult> {
    return this.request(`/api/v1/privacy/users/${encodeURIComponent(visitorId)}`, {
      method: "DELETE",
    });
  }

  async getAuditLogs(params?: { limit?: number; offset?: number }): Promise<ListResponse<AuditLog>> {
    return this.request("/api/v1/audit-logs", {
      params: {
        ...(params?.limit ? { limit: String(params.limit) } : {}),
        ...(params?.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }

  async getPrivacySettings(): Promise<PrivacySettings> {
    return this.request("/api/v1/privacy/settings");
  }

  async updatePrivacySettings(params: Partial<Pick<PrivacySettings,
    "anonymize_ip" |
    "respect_dnt" |
    "bot_filtering" |
    "consent_required" |
    "allowed_consent_modes" |
    "blocked_user_agents"
  >>): Promise<PrivacySettings> {
    return this.request("/api/v1/privacy/settings", {
      method: "PUT",
      body: JSON.stringify(params),
    });
  }

  async getSegments(): Promise<ListResponse<SavedSegment>> {
    return this.request("/api/v1/segments");
  }

  async createSegment(params: {
    name: string;
    description?: string;
    definition: SegmentDefinition;
  }): Promise<SavedSegment> {
    return this.request("/api/v1/segments", {
      method: "POST",
      body: JSON.stringify(params),
    });
  }

  async updateSegment(segmentId: string, params: {
    name: string;
    description?: string;
    definition: SegmentDefinition;
    isActive?: boolean;
  }): Promise<SavedSegment> {
    return this.request(`/api/v1/segments/${encodeURIComponent(segmentId)}`, {
      method: "PUT",
      body: JSON.stringify({
        name: params.name,
        description: params.description,
        definition: params.definition,
        is_active: params.isActive ?? true,
      }),
    });
  }

  async deleteSegment(segmentId: string): Promise<void> {
    await this.request(`/api/v1/segments/${encodeURIComponent(segmentId)}`, {
      method: "DELETE",
    });
  }

  async evaluateSegment(segmentId: string, params: PaginatedQuery): Promise<SegmentEvaluation> {
    return this.request(`/api/v1/segments/${encodeURIComponent(segmentId)}/evaluate`, {
      params: {
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
        ...(params.limit ? { limit: String(params.limit) } : {}),
        ...(params.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }

  async compareSegments(segmentIds: string[], params: StatsQuery): Promise<ListResponse<SegmentCompareRow>> {
    return this.request("/api/v1/segments/compare", {
      params: {
        segment_ids: segmentIds.join(","),
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
      },
    });
  }

  async breakdownSegment(segmentId: string, params: StatsQuery & { property: string; limit?: number }): Promise<ListResponse<SegmentBreakdownRow>> {
    return this.request(`/api/v1/segments/${encodeURIComponent(segmentId)}/breakdown`, {
      params: {
        property: params.property,
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
        ...(params.limit ? { limit: String(params.limit) } : {}),
      },
    });
  }

  async getTrackingPlans(): Promise<ListResponse<TrackingPlan>> {
    return this.request("/api/v1/governance/tracking-plans");
  }

  async getTrackingPlan(planId: string): Promise<TrackingPlan> {
    return this.request(`/api/v1/governance/tracking-plans/${encodeURIComponent(planId)}`);
  }

  async createTrackingPlan(params: {
    name: string;
    description?: string;
    enforcementMode?: "observe" | "reject" | string;
    isActive?: boolean;
  }): Promise<TrackingPlan> {
    return this.request("/api/v1/governance/tracking-plans", {
      method: "POST",
      body: JSON.stringify({
        name: params.name,
        description: params.description,
        enforcement_mode: params.enforcementMode ?? "observe",
        is_active: params.isActive ?? true,
      }),
    });
  }

  async updateTrackingPlan(planId: string, params: {
    name: string;
    description?: string;
    enforcementMode?: "observe" | "reject" | string;
    isActive?: boolean;
  }): Promise<TrackingPlan> {
    return this.request(`/api/v1/governance/tracking-plans/${encodeURIComponent(planId)}`, {
      method: "PUT",
      body: JSON.stringify({
        name: params.name,
        description: params.description,
        enforcement_mode: params.enforcementMode ?? "observe",
        is_active: params.isActive ?? true,
      }),
    });
  }

  async deleteTrackingPlan(planId: string): Promise<void> {
    await this.request(`/api/v1/governance/tracking-plans/${encodeURIComponent(planId)}`, {
      method: "DELETE",
    });
  }

  async getEventSchemas(params?: { trackingPlanId?: string }): Promise<ListResponse<EventSchemaDefinition>> {
    return this.request("/api/v1/governance/event-schemas", {
      params: {
        ...(params?.trackingPlanId ? { tracking_plan_id: params.trackingPlanId } : {}),
      },
    });
  }

  async getEventSchema(schemaId: string): Promise<EventSchemaDefinition> {
    return this.request(`/api/v1/governance/event-schemas/${encodeURIComponent(schemaId)}`);
  }

  async createEventSchema(params: {
    trackingPlanId?: string;
    eventName: string;
    description?: string;
    status?: EventSchemaStatus;
    requiredProperties?: string[];
    propertySchema?: EventSchemaDefinition["property_schema"];
  }): Promise<EventSchemaDefinition> {
    return this.request("/api/v1/governance/event-schemas", {
      method: "POST",
      body: JSON.stringify({
        tracking_plan_id: params.trackingPlanId,
        event_name: params.eventName,
        description: params.description,
        status: params.status ?? "draft",
        required_properties: params.requiredProperties ?? [],
        property_schema: params.propertySchema ?? {},
      }),
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
    return this.request(`/api/v1/governance/event-schemas/${encodeURIComponent(schemaId)}`, {
      method: "PUT",
      body: JSON.stringify({
        tracking_plan_id: params.trackingPlanId,
        event_name: params.eventName,
        description: params.description,
        status: params.status ?? "draft",
        required_properties: params.requiredProperties ?? [],
        property_schema: params.propertySchema ?? {},
      }),
    });
  }

  async updateEventSchemaStatus(schemaId: string, status: EventSchemaStatus): Promise<EventSchemaDefinition> {
    return this.request(`/api/v1/governance/event-schemas/${encodeURIComponent(schemaId)}/status`, {
      method: "PUT",
      body: JSON.stringify({ status }),
    });
  }

  async deleteEventSchema(schemaId: string): Promise<void> {
    await this.request(`/api/v1/governance/event-schemas/${encodeURIComponent(schemaId)}`, {
      method: "DELETE",
    });
  }

  async getDataDictionaryEntries(params?: { entryType?: string }): Promise<ListResponse<DataDictionaryEntry>> {
    return this.request("/api/v1/governance/data-dictionary", {
      params: {
        ...(params?.entryType ? { entry_type: params.entryType } : {}),
      },
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
    return this.request("/api/v1/governance/data-dictionary", {
      method: "POST",
      body: JSON.stringify({
        entry_type: params.entryType,
        name: params.name,
        data_type: params.dataType,
        description: params.description,
        owner: params.owner,
        is_pii: params.isPii ?? false,
      }),
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
    return this.request(`/api/v1/governance/data-dictionary/${encodeURIComponent(entryId)}`, {
      method: "PUT",
      body: JSON.stringify({
        entry_type: params.entryType,
        name: params.name,
        data_type: params.dataType,
        description: params.description,
        owner: params.owner,
        is_pii: params.isPii ?? false,
      }),
    });
  }

  async deleteDataDictionaryEntry(entryId: string): Promise<void> {
    await this.request(`/api/v1/governance/data-dictionary/${encodeURIComponent(entryId)}`, {
      method: "DELETE",
    });
  }

  async getQualityViolations(params?: {
    eventName?: string;
    violationType?: string;
    limit?: number;
    offset?: number;
  }): Promise<ListResponse<EventQualityViolation>> {
    return this.request("/api/v1/governance/violations", {
      params: {
        ...(params?.eventName ? { event_name: params.eventName } : {}),
        ...(params?.violationType ? { violation_type: params.violationType } : {}),
        ...(params?.limit ? { limit: String(params.limit) } : {}),
        ...(params?.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }

  async getGovernanceHealth(): Promise<GovernanceHealth> {
    return this.request("/api/v1/governance/health");
  }

  async getFeatureFlags(): Promise<ListResponse<FeatureFlag>> {
    return this.request("/api/v1/feature-flags");
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
    return this.request("/api/v1/feature-flags", {
      method: "POST",
      body: JSON.stringify({
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
      }),
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
    return this.request(`/api/v1/feature-flags/${encodeURIComponent(flagId)}`, {
      method: "PUT",
      body: JSON.stringify({
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
      }),
    });
  }

  async deleteFeatureFlag(flagId: string): Promise<void> {
    await this.request(`/api/v1/feature-flags/${encodeURIComponent(flagId)}`, {
      method: "DELETE",
    });
  }

  async evaluateFeatureFlag(key: string, ctx: FeatureFlagEvaluationContext): Promise<FeatureFlagEvaluationResult> {
    return this.request(`/api/v1/feature-flags/${encodeURIComponent(key)}/evaluate`, {
      method: "POST",
      body: JSON.stringify(evaluationPayload(ctx)),
    });
  }

  async getFeatureFlagEvaluations(flagId: string, params?: { limit?: number; offset?: number }): Promise<ListResponse<FeatureFlagEvaluation>> {
    return this.request(`/api/v1/feature-flags/${encodeURIComponent(flagId)}/evaluations`, {
      params: {
        ...(params?.limit ? { limit: String(params.limit) } : {}),
        ...(params?.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }

  async getRemoteConfigs(): Promise<ListResponse<RemoteConfigEntry>> {
    return this.request("/api/v1/remote-config");
  }

  async createRemoteConfig(params: {
    key: string;
    description?: string;
    value?: unknown;
    targetingRules?: TargetingRules;
    isActive?: boolean;
  }): Promise<RemoteConfigEntry> {
    return this.request("/api/v1/remote-config", {
      method: "POST",
      body: JSON.stringify({
        key: params.key,
        description: params.description,
        value: params.value ?? {},
        targeting_rules: params.targetingRules ?? { match: "all", conditions: [] },
        is_active: params.isActive ?? true,
      }),
    });
  }

  async updateRemoteConfig(entryId: string, params: {
    key: string;
    description?: string;
    value?: unknown;
    targetingRules?: TargetingRules;
    isActive?: boolean;
  }): Promise<RemoteConfigEntry> {
    return this.request(`/api/v1/remote-config/${encodeURIComponent(entryId)}`, {
      method: "PUT",
      body: JSON.stringify({
        key: params.key,
        description: params.description,
        value: params.value ?? {},
        targeting_rules: params.targetingRules ?? { match: "all", conditions: [] },
        is_active: params.isActive ?? true,
      }),
    });
  }

  async deleteRemoteConfig(entryId: string): Promise<void> {
    await this.request(`/api/v1/remote-config/${encodeURIComponent(entryId)}`, {
      method: "DELETE",
    });
  }

  async evaluateRemoteConfig(key: string, ctx: FeatureFlagEvaluationContext): Promise<RemoteConfigEvaluationResult> {
    return this.request(`/api/v1/remote-config/${encodeURIComponent(key)}/evaluate`, {
      method: "POST",
      body: JSON.stringify(evaluationPayload(ctx)),
    });
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
  async getMarketingChannels(params: StatsQuery): Promise<ListResponse<MarketingChannelStat>> {
    return this.request("/api/v1/marketing/channels", {
      params: { start_at: toISO(params.startAt), end_at: toISO(params.endAt) },
    });
  }
  async getMarketingAttribution(params: StatsQuery & { model?: "first_touch" | "last_touch" | "linear" | string }): Promise<ListResponse<AttributionRow>> {
    return this.request("/api/v1/marketing/attribution", {
      params: {
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
        ...(params.model ? { model: params.model } : {}),
      },
    });
  }
  async getEcommerceReport(params: StatsQuery): Promise<EcommerceReport> {
    return this.request("/api/v1/marketing/ecommerce", {
      params: { start_at: toISO(params.startAt), end_at: toISO(params.endAt) },
    });
  }
  async getAiReferrers(params: StatsQuery): Promise<ListResponse<AiReferrerStat>> {
    return this.request("/api/v1/marketing/ai-referrers", {
      params: { start_at: toISO(params.startAt), end_at: toISO(params.endAt) },
    });
  }
  async getMarketingImports(params: { provider?: string; limit?: number; offset?: number } = {}): Promise<ListResponse<MarketingImport>> {
    return this.request("/api/v1/marketing/imports", {
      params: {
        ...(params.provider ? { provider: params.provider } : {}),
        ...(params.limit ? { limit: String(params.limit) } : {}),
        ...(params.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }
  async createMarketingImport(input: MarketingImportInput): Promise<MarketingImport> {
    return this.request("/api/v1/marketing/imports", {
      method: "POST",
      body: JSON.stringify(marketingImportPayload(input)),
    });
  }
  async getMarketingImportRows(importId: string, params: { limit?: number; offset?: number } = {}): Promise<ListResponse<MarketingImportRow>> {
    return this.request(`/api/v1/marketing/imports/${encodeURIComponent(importId)}/rows`, {
      params: {
        ...(params.limit ? { limit: String(params.limit) } : {}),
        ...(params.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }
  async deleteMarketingImport(importId: string): Promise<void> {
    await this.request(`/api/v1/marketing/imports/${encodeURIComponent(importId)}`, {
      method: "DELETE",
    });
  }
  async getMarketingImportSummary(params: StatsQuery & { provider?: string }): Promise<MarketingImportSummary> {
    return this.request("/api/v1/marketing/imports/summary", {
      params: {
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
        ...(params.provider ? { provider: params.provider } : {}),
      },
    });
  }
  async getWebVitals(params: StatsQuery): Promise<ListResponse<WebVitalSummary>> {
    return this.request("/api/v1/webvitals", { params: { start_at: toISO(params.startAt), end_at: toISO(params.endAt) } });
  }
  async getClickHeatmap(params: StatsQuery & { path: string }): Promise<ListResponse<ClickHeatmapPoint>> {
    return this.request("/api/v1/heatmaps", {
      params: {
        path: params.path,
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
      },
    });
  }
  async getClickStats(params: StatsQuery & { limit?: number }): Promise<ListResponse<PageClickStats>> {
    return this.request("/api/v1/heatmaps/stats", {
      params: {
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
        ...(params.limit ? { limit: String(params.limit) } : {}),
      },
    });
  }
  async getFrictionSignals(params: StatsQuery & { path?: string; limit?: number }): Promise<ListResponse<FrictionSignal>> {
    return this.request("/api/v1/heatmaps/friction", {
      params: {
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
        ...(params.path ? { path: params.path } : {}),
        ...(params.limit ? { limit: String(params.limit) } : {}),
      },
    });
  }
  async getVisualEventLabels(): Promise<ListResponse<VisualEventLabel>> {
    return this.request("/api/v1/heatmaps/labels");
  }
  async createVisualEventLabel(input: VisualEventLabelInput): Promise<VisualEventLabel> {
    return this.request("/api/v1/heatmaps/labels", {
      method: "POST",
      body: JSON.stringify(visualEventLabelPayload(input)),
    });
  }
  async updateVisualEventLabel(labelId: string, input: VisualEventLabelInput): Promise<VisualEventLabel> {
    return this.request(`/api/v1/heatmaps/labels/${encodeURIComponent(labelId)}`, {
      method: "PUT",
      body: JSON.stringify(visualEventLabelPayload(input)),
    });
  }
  async deleteVisualEventLabel(labelId: string): Promise<void> {
    await this.request(`/api/v1/heatmaps/labels/${encodeURIComponent(labelId)}`, {
      method: "DELETE",
    });
  }
  async getVisualEventLabelMatches(params: StatsQuery & { limit?: number }): Promise<ListResponse<VisualEventLabelStats>> {
    return this.request("/api/v1/heatmaps/labels/stats", {
      params: {
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
        ...(params.limit ? { limit: String(params.limit) } : {}),
      },
    });
  }
  async getVisualEventLabelStats(labelId: string, params: StatsQuery): Promise<VisualEventLabelStats> {
    return this.request(`/api/v1/heatmaps/labels/${encodeURIComponent(labelId)}/stats`, {
      params: {
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
      },
    });
  }
  async getErrors(params: PaginatedQuery): Promise<ListResponse<ErrorGroup>> {
    return this.request("/api/v1/errors", { params: { start_at: toISO(params.startAt), end_at: toISO(params.endAt), ...(params.limit ? { limit: String(params.limit) } : {}), ...(params.offset ? { offset: String(params.offset) } : {}) } });
  }
  async getErrorDetail(params: StatsQuery & { message?: string; fingerprint?: string; limit?: number }): Promise<ListResponse<ErrorInstance>> {
    return this.request("/api/v1/errors/detail", {
      params: {
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
        ...(params.message ? { message: params.message } : {}),
        ...(params.fingerprint ? { fingerprint: params.fingerprint } : {}),
        ...(params.limit ? { limit: String(params.limit) } : {}),
      },
    });
  }
  async getReleases(params?: { limit?: number; offset?: number }): Promise<ListResponse<AppRelease>> {
    return this.request("/api/v1/releases", {
      params: {
        ...(params?.limit ? { limit: String(params.limit) } : {}),
        ...(params?.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }
  async createRelease(params: {
    version: string;
    environment?: string;
    commitSha?: string;
    deployedAt?: string | Date;
    metadata?: Record<string, unknown>;
  }): Promise<AppRelease> {
    return this.request("/api/v1/releases", {
      method: "POST",
      body: JSON.stringify({
        version: params.version,
        environment: params.environment,
        commit_sha: params.commitSha,
        deployed_at: params.deployedAt ? toISO(params.deployedAt) : undefined,
        metadata: params.metadata || {},
      }),
    });
  }
  async deleteRelease(releaseId: string): Promise<void> {
    await this.request(`/api/v1/releases/${encodeURIComponent(releaseId)}`, {
      method: "DELETE",
    });
  }
  async getSourceMaps(params?: { releaseVersion?: string; limit?: number; offset?: number }): Promise<ListResponse<SourceMapArtifact>> {
    return this.request("/api/v1/source-maps", {
      params: {
        ...(params?.releaseVersion ? { release_version: params.releaseVersion } : {}),
        ...(params?.limit ? { limit: String(params.limit) } : {}),
        ...(params?.offset ? { offset: String(params.offset) } : {}),
      },
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
    return this.request("/api/v1/source-maps", {
      method: "POST",
      body: JSON.stringify({
        release_version: params.releaseVersion,
        environment: params.environment,
        minified_url: params.minifiedUrl,
        source_map_url: params.sourceMapUrl,
        artifacts: params.artifacts || {},
        uploaded_by: params.uploadedBy,
      }),
    });
  }
  async deleteSourceMap(sourceMapId: string): Promise<void> {
    await this.request(`/api/v1/source-maps/${encodeURIComponent(sourceMapId)}`, {
      method: "DELETE",
    });
  }
  async getLogs(params: PaginatedQuery & { level?: string; release?: string; environment?: string; search?: string }): Promise<ListResponse<LogEntry>> {
    return this.request("/api/v1/logs", {
      params: {
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
        ...(params.level ? { level: params.level } : {}),
        ...(params.release ? { release: params.release } : {}),
        ...(params.environment ? { environment: params.environment } : {}),
        ...(params.search ? { search: params.search } : {}),
        ...(params.limit ? { limit: String(params.limit) } : {}),
        ...(params.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }
  async getLogStats(params: StatsQuery): Promise<LogStats> {
    return this.request("/api/v1/logs/stats", { params: { start_at: toISO(params.startAt), end_at: toISO(params.endAt) } });
  }
  async getDashboards(): Promise<ListResponse<CustomDashboard>> {
    return this.request("/api/v1/dashboards");
  }
  async getDashboard(dashboardId: string): Promise<CustomDashboard> {
    return this.request(`/api/v1/dashboards/${encodeURIComponent(dashboardId)}`);
  }
  async createDashboard(params: {
    name: string;
    description?: string;
    layout?: Record<string, unknown>;
    widgets?: unknown[];
    isDefault?: boolean;
  }): Promise<CustomDashboard> {
    return this.request("/api/v1/dashboards", {
      method: "POST",
      body: JSON.stringify({
        name: params.name,
        description: params.description,
        layout: params.layout || {},
        widgets: params.widgets || [],
        is_default: params.isDefault ?? false,
      }),
    });
  }
  async updateDashboard(dashboardId: string, params: {
    name: string;
    description?: string;
    layout?: Record<string, unknown>;
    widgets?: unknown[];
    isDefault?: boolean;
  }): Promise<CustomDashboard> {
    return this.request(`/api/v1/dashboards/${encodeURIComponent(dashboardId)}`, {
      method: "PUT",
      body: JSON.stringify({
        name: params.name,
        description: params.description,
        layout: params.layout || {},
        widgets: params.widgets || [],
        is_default: params.isDefault ?? false,
      }),
    });
  }
  async deleteDashboard(dashboardId: string): Promise<void> {
    await this.request(`/api/v1/dashboards/${encodeURIComponent(dashboardId)}`, {
      method: "DELETE",
    });
  }
  async getSavedReports(): Promise<ListResponse<SavedReport>> {
    return this.request("/api/v1/reports");
  }
  async getSavedReport(reportId: string): Promise<SavedReport> {
    return this.request(`/api/v1/reports/${encodeURIComponent(reportId)}`);
  }
  async createSavedReport(params: {
    name: string;
    description?: string;
    reportType: string;
    params?: Record<string, unknown>;
    visualization?: string;
    isActive?: boolean;
  }): Promise<SavedReport> {
    return this.request("/api/v1/reports", {
      method: "POST",
      body: JSON.stringify({
        name: params.name,
        description: params.description,
        report_type: params.reportType,
        params: params.params || {},
        visualization: params.visualization || "table",
        is_active: params.isActive ?? true,
      }),
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
    return this.request(`/api/v1/reports/${encodeURIComponent(reportId)}`, {
      method: "PUT",
      body: JSON.stringify({
        name: params.name,
        description: params.description,
        report_type: params.reportType,
        params: params.params || {},
        visualization: params.visualization || "table",
        is_active: params.isActive ?? true,
      }),
    });
  }
  async deleteSavedReport(reportId: string): Promise<void> {
    await this.request(`/api/v1/reports/${encodeURIComponent(reportId)}`, {
      method: "DELETE",
    });
  }
  async runSavedReport(reportId: string, params?: { startAt?: string | Date; endAt?: string | Date }): Promise<QueryExplorerResponse> {
    return this.request(`/api/v1/reports/${encodeURIComponent(reportId)}/run`, {
      method: "POST",
      params: {
        ...(params?.startAt ? { start_at: toISO(params.startAt) } : {}),
        ...(params?.endAt ? { end_at: toISO(params.endAt) } : {}),
      },
    });
  }
  async runQueryExplorer(input: QueryExplorerRequest): Promise<QueryExplorerResponse> {
    return this.request("/api/v1/query-explorer", {
      method: "POST",
      body: JSON.stringify(input),
    });
  }
  async getQueryExplorerHistory(params?: { limit?: number; offset?: number }): Promise<ListResponse<QueryExplorerRun>> {
    return this.request("/api/v1/query-explorer/history", {
      params: {
        ...(params?.limit ? { limit: String(params.limit) } : {}),
        ...(params?.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }
  async getProductStickiness(params: StatsQuery): Promise<ProductStickinessReport> {
    return this.request("/api/v1/product/stickiness", {
      params: {
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
      },
    });
  }
  async getProductLifecycle(params: StatsQuery): Promise<ProductLifecycleReport> {
    return this.request("/api/v1/product/lifecycle", {
      params: {
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
      },
    });
  }
  async getProductActivation(input: ProductActivationRequest): Promise<ProductActivationReport> {
    return this.request("/api/v1/product/activation", {
      method: "POST",
      body: JSON.stringify({
        start_at: toISO(input.startAt),
        end_at: toISO(input.endAt),
        event_names: input.eventNames || [],
        paths: input.paths || [],
      }),
    });
  }
  async getProductImpact(input: ProductImpactRequest): Promise<ProductImpactReport> {
    return this.request("/api/v1/product/impact", {
      method: "POST",
      body: JSON.stringify({
        metric: input.metric,
        split_at: toISO(input.splitAt),
        window_days: input.windowDays,
        event_name: input.eventName,
      }),
    });
  }
  async getBiConnections(): Promise<ListResponse<BiDatabaseConnection>> {
    return this.request("/api/v1/bi/connections");
  }
  async getBiConnection(connectionId: string): Promise<BiDatabaseConnection> {
    return this.request(`/api/v1/bi/connections/${encodeURIComponent(connectionId)}`);
  }
  async createBiConnection(input: BiDatabaseConnectionInput): Promise<BiDatabaseConnection> {
    return this.request("/api/v1/bi/connections", {
      method: "POST",
      body: JSON.stringify(biDatabaseConnectionPayload(input)),
    });
  }
  async updateBiConnection(connectionId: string, input: BiDatabaseConnectionInput): Promise<BiDatabaseConnection> {
    return this.request(`/api/v1/bi/connections/${encodeURIComponent(connectionId)}`, {
      method: "PUT",
      body: JSON.stringify(biDatabaseConnectionPayload(input)),
    });
  }
  async deleteBiConnection(connectionId: string): Promise<void> {
    await this.request(`/api/v1/bi/connections/${encodeURIComponent(connectionId)}`, {
      method: "DELETE",
    });
  }
  async testBiConnection(connectionId: string): Promise<BiConnectionTestResponse> {
    return this.request(`/api/v1/bi/connections/${encodeURIComponent(connectionId)}/test`, {
      method: "POST",
    });
  }
  async runBiConnectionSql(connectionId: string, input: BiExternalSqlRunRequest): Promise<BiQueryResponse> {
    return this.request(`/api/v1/bi/connections/${encodeURIComponent(connectionId)}/query`, {
      method: "POST",
      body: JSON.stringify({
        sql_text: input.sqlText,
        limit: input.limit,
      }),
    });
  }
  async getBiEmbeds(): Promise<ListResponse<BiEmbed>> {
    return this.request("/api/v1/bi/embeds");
  }
  async getBiEmbed(embedId: string): Promise<BiEmbed> {
    return this.request(`/api/v1/bi/embeds/${encodeURIComponent(embedId)}`);
  }
  async createBiEmbed(input: BiEmbedInput): Promise<BiEmbedWithToken> {
    return this.request("/api/v1/bi/embeds", {
      method: "POST",
      body: JSON.stringify(biEmbedPayload(input)),
    });
  }
  async updateBiEmbed(embedId: string, input: BiEmbedInput): Promise<BiEmbed> {
    return this.request(`/api/v1/bi/embeds/${encodeURIComponent(embedId)}`, {
      method: "PUT",
      body: JSON.stringify(biEmbedPayload(input)),
    });
  }
  async deleteBiEmbed(embedId: string): Promise<void> {
    await this.request(`/api/v1/bi/embeds/${encodeURIComponent(embedId)}`, {
      method: "DELETE",
    });
  }
  async rotateBiEmbedToken(embedId: string): Promise<BiEmbedWithToken> {
    return this.request(`/api/v1/bi/embeds/${encodeURIComponent(embedId)}/rotate-token`, {
      method: "POST",
    });
  }
  async resolveBiEmbed(token: string): Promise<BiEmbedResolved> {
    return this.request(`/api/embed/bi/${encodeURIComponent(token)}`);
  }
  async getBiMetrics(): Promise<ListResponse<SemanticMetric>> {
    return this.request("/api/v1/bi/metrics");
  }
  async getBiMetric(metricId: string): Promise<SemanticMetric> {
    return this.request(`/api/v1/bi/metrics/${encodeURIComponent(metricId)}`);
  }
  async createBiMetric(input: SemanticMetricInput): Promise<SemanticMetric> {
    return this.request("/api/v1/bi/metrics", {
      method: "POST",
      body: JSON.stringify(semanticMetricPayload(input)),
    });
  }
  async updateBiMetric(metricId: string, input: SemanticMetricInput): Promise<SemanticMetric> {
    return this.request(`/api/v1/bi/metrics/${encodeURIComponent(metricId)}`, {
      method: "PUT",
      body: JSON.stringify(semanticMetricPayload(input)),
    });
  }
  async deleteBiMetric(metricId: string): Promise<void> {
    await this.request(`/api/v1/bi/metrics/${encodeURIComponent(metricId)}`, {
      method: "DELETE",
    });
  }
  async getBiRowPolicies(): Promise<ListResponse<BiRowPolicy>> {
    return this.request("/api/v1/bi/row-policies");
  }
  async createBiRowPolicy(input: BiRowPolicyInput): Promise<BiRowPolicy> {
    return this.request("/api/v1/bi/row-policies", {
      method: "POST",
      body: JSON.stringify(biRowPolicyPayload(input)),
    });
  }
  async updateBiRowPolicy(policyId: string, input: BiRowPolicyInput): Promise<BiRowPolicy> {
    return this.request(`/api/v1/bi/row-policies/${encodeURIComponent(policyId)}`, {
      method: "PUT",
      body: JSON.stringify(biRowPolicyPayload(input)),
    });
  }
  async deleteBiRowPolicy(policyId: string): Promise<void> {
    await this.request(`/api/v1/bi/row-policies/${encodeURIComponent(policyId)}`, {
      method: "DELETE",
    });
  }
  async runBiSql(input: BiSqlRunRequest): Promise<BiQueryResponse> {
    return this.request("/api/v1/bi/sql", {
      method: "POST",
      body: JSON.stringify({
        sql_text: input.sqlText,
        limit: input.limit,
      }),
    });
  }
  async getBiSavedQueries(): Promise<ListResponse<SavedSqlQuery>> {
    return this.request("/api/v1/bi/sql-queries");
  }
  async getBiSavedQuery(queryId: string): Promise<SavedSqlQuery> {
    return this.request(`/api/v1/bi/sql-queries/${encodeURIComponent(queryId)}`);
  }
  async createBiSavedQuery(input: SavedSqlInput): Promise<SavedSqlQuery> {
    return this.request("/api/v1/bi/sql-queries", {
      method: "POST",
      body: JSON.stringify(savedSqlPayload(input)),
    });
  }
  async updateBiSavedQuery(queryId: string, input: SavedSqlInput): Promise<SavedSqlQuery> {
    return this.request(`/api/v1/bi/sql-queries/${encodeURIComponent(queryId)}`, {
      method: "PUT",
      body: JSON.stringify(savedSqlPayload(input)),
    });
  }
  async deleteBiSavedQuery(queryId: string): Promise<void> {
    await this.request(`/api/v1/bi/sql-queries/${encodeURIComponent(queryId)}`, {
      method: "DELETE",
    });
  }
  async runBiSavedQuery(queryId: string, params?: { limit?: number }): Promise<BiQueryResponse> {
    return this.request(`/api/v1/bi/sql-queries/${encodeURIComponent(queryId)}/run`, {
      method: "POST",
      params: {
        ...(params?.limit ? { limit: String(params.limit) } : {}),
      },
    });
  }
  async runBiVisualQuery(input: BiVisualQueryRequest): Promise<BiQueryResponse> {
    return this.request("/api/v1/bi/visual-query", {
      method: "POST",
      body: JSON.stringify({
        dataset: input.dataset,
        dimensions: input.dimensions || [],
        metrics: input.metrics || [],
        start_at: input.startAt ? toISO(input.startAt) : undefined,
        end_at: input.endAt ? toISO(input.endAt) : undefined,
        limit: input.limit,
      }),
    });
  }
  async runBiDrillThrough(input: BiDrillThroughRequest): Promise<BiQueryResponse> {
    return this.request("/api/v1/bi/drill-through", {
      method: "POST",
      body: JSON.stringify({
        dataset: input.dataset,
        filters: input.filters || {},
        start_at: input.startAt ? toISO(input.startAt) : undefined,
        end_at: input.endAt ? toISO(input.endAt) : undefined,
        limit: input.limit,
      }),
    });
  }
  async getBiQueryRuns(params?: { limit?: number; offset?: number }): Promise<ListResponse<BiQueryRun>> {
    return this.request("/api/v1/bi/query-runs", {
      params: {
        ...(params?.limit ? { limit: String(params.limit) } : {}),
        ...(params?.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }
  async getCsvUploads(): Promise<ListResponse<CsvUpload>> {
    return this.request("/api/v1/bi/csv-uploads");
  }
  async createCsvUpload(input: CsvUploadInput): Promise<CsvUpload> {
    return this.request("/api/v1/bi/csv-uploads", {
      method: "POST",
      body: JSON.stringify(csvUploadPayload(input)),
    });
  }
  async getCsvUploadRows(uploadId: string, params?: { limit?: number; offset?: number }): Promise<ListResponse<Record<string, unknown>>> {
    return this.request(`/api/v1/bi/csv-uploads/${encodeURIComponent(uploadId)}/rows`, {
      params: {
        ...(params?.limit ? { limit: String(params.limit) } : {}),
        ...(params?.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }
  async deleteCsvUpload(uploadId: string): Promise<void> {
    await this.request(`/api/v1/bi/csv-uploads/${encodeURIComponent(uploadId)}`, {
      method: "DELETE",
    });
  }
  async getIntegrations(filter?: IntegrationFilter): Promise<ListResponse<Integration>> {
    return this.request("/api/v1/integrations", {
      params: {
        ...(filter?.category ? { category: filter.category } : {}),
        ...(filter?.capability ? { capability: filter.capability } : {}),
        ...(filter?.status ? { status: filter.status } : {}),
      },
    });
  }
  async getIntegration(key: string): Promise<Integration> {
    return this.request(`/api/v1/integrations/${encodeURIComponent(key)}`);
  }
  async getSources(): Promise<ListResponse<EventSource>> {
    return this.request("/api/v1/sources");
  }
  async getSource(sourceId: string): Promise<EventSource> {
    return this.request(`/api/v1/sources/${encodeURIComponent(sourceId)}`);
  }
  async createSource(input: SourceInput): Promise<SourceWithToken> {
    return this.request("/api/v1/sources", {
      method: "POST",
      body: JSON.stringify(sourcePayload(input)),
    });
  }
  async updateSource(sourceId: string, input: SourceInput): Promise<EventSource> {
    return this.request(`/api/v1/sources/${encodeURIComponent(sourceId)}`, {
      method: "PUT",
      body: JSON.stringify(sourcePayload(input)),
    });
  }
  async deleteSource(sourceId: string): Promise<void> {
    await this.request(`/api/v1/sources/${encodeURIComponent(sourceId)}`, {
      method: "DELETE",
    });
  }
  async getSourceIngestions(sourceId: string, params?: { limit?: number; offset?: number }): Promise<ListResponse<SourceIngestion>> {
    return this.request(`/api/v1/sources/${encodeURIComponent(sourceId)}/ingestions`, {
      params: {
        ...(params?.limit ? { limit: String(params.limit) } : {}),
        ...(params?.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }
  async ingestSourceWebhook(sourceId: string, token: string, payload: Record<string, unknown>): Promise<SourceIngestResponse> {
    return this.request(`/api/source/${encodeURIComponent(sourceId)}/collect`, {
      method: "POST",
      headers: {
        "X-Pulse-Source-Token": token,
      },
      body: JSON.stringify(payload),
    });
  }
  async getDestinations(): Promise<ListResponse<Destination>> {
    return this.request("/api/v1/destinations");
  }
  async getDestination(destinationId: string): Promise<Destination> {
    return this.request(`/api/v1/destinations/${encodeURIComponent(destinationId)}`);
  }
  async createDestination(input: DestinationInput): Promise<Destination> {
    return this.request("/api/v1/destinations", {
      method: "POST",
      body: JSON.stringify(destinationPayload(input)),
    });
  }
  async updateDestination(destinationId: string, input: DestinationInput): Promise<Destination> {
    return this.request(`/api/v1/destinations/${encodeURIComponent(destinationId)}`, {
      method: "PUT",
      body: JSON.stringify(destinationPayload(input)),
    });
  }
  async deleteDestination(destinationId: string): Promise<void> {
    await this.request(`/api/v1/destinations/${encodeURIComponent(destinationId)}`, {
      method: "DELETE",
    });
  }
  async getDestinationDeliveries(params?: { status?: string; limit?: number; offset?: number }): Promise<ListResponse<DestinationDelivery>> {
    return this.request("/api/v1/destination-deliveries", {
      params: {
        ...(params?.status ? { status: params.status } : {}),
        ...(params?.limit ? { limit: String(params.limit) } : {}),
        ...(params?.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }
  async retryDestinationDelivery(deliveryId: string): Promise<{ ok: boolean }> {
    return this.request(`/api/v1/destination-deliveries/${encodeURIComponent(deliveryId)}/retry`, {
      method: "POST",
    });
  }
  async getDestinationHealth(): Promise<ListResponse<DestinationHealth>> {
    return this.request("/api/v1/destination-health");
  }
  async askAiQuery(params: {
    question: string;
    startAt?: string | Date;
    endAt?: string | Date;
    limit?: number;
  }): Promise<AiQueryResponse> {
    return this.request("/api/v1/ai/query", {
      method: "POST",
      body: JSON.stringify({
        question: params.question,
        start_at: params.startAt ? toISO(params.startAt) : undefined,
        end_at: params.endAt ? toISO(params.endAt) : undefined,
        limit: params.limit,
      }),
    });
  }
  async getAiInsights(params?: { startAt?: string | Date; endAt?: string | Date }): Promise<ListResponse<AiInsight>> {
    return this.request("/api/v1/ai/insights", {
      params: {
        ...(params?.startAt ? { start_at: toISO(params.startAt) } : {}),
        ...(params?.endAt ? { end_at: toISO(params.endAt) } : {}),
      },
    });
  }
  async getAiQueryHistory(params?: { limit?: number; offset?: number }): Promise<ListResponse<AiQueryRun>> {
    return this.request("/api/v1/ai/history", {
      params: {
        ...(params?.limit ? { limit: String(params.limit) } : {}),
        ...(params?.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }
  async getLlmTraces(params?: { limit?: number; offset?: number }): Promise<ListResponse<LlmTrace>> {
    return this.request("/api/v1/ai/llm/traces", {
      params: {
        ...(params?.limit ? { limit: String(params.limit) } : {}),
        ...(params?.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }
  async recordLlmTrace(input: LlmTraceInput): Promise<LlmTrace> {
    return this.request("/api/v1/ai/llm/traces", {
      method: "POST",
      body: JSON.stringify(llmTracePayload(input)),
    });
  }
  async getLlmTrace(traceId: string): Promise<LlmTrace> {
    return this.request(`/api/v1/ai/llm/traces/${encodeURIComponent(traceId)}`);
  }
  async getLlmGenerations(params?: { limit?: number; offset?: number }): Promise<ListResponse<LlmGeneration>> {
    return this.request("/api/v1/ai/llm/generations", {
      params: {
        ...(params?.limit ? { limit: String(params.limit) } : {}),
        ...(params?.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }
  async recordLlmGeneration(input: LlmGenerationInput): Promise<LlmGeneration> {
    return this.request("/api/v1/ai/llm/generations", {
      method: "POST",
      body: JSON.stringify(llmGenerationPayload(input)),
    });
  }
  async getLlmEvaluations(params?: { limit?: number; offset?: number }): Promise<ListResponse<LlmEvaluation>> {
    return this.request("/api/v1/ai/llm/evaluations", {
      params: {
        ...(params?.limit ? { limit: String(params.limit) } : {}),
        ...(params?.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }
  async recordLlmEvaluation(input: LlmEvaluationInput): Promise<LlmEvaluation> {
    return this.request("/api/v1/ai/llm/evaluations", {
      method: "POST",
      body: JSON.stringify(llmEvaluationPayload(input)),
    });
  }
  async getLlmStats(params: StatsQuery): Promise<LlmStats> {
    return this.request("/api/v1/ai/llm/stats", {
      params: {
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
      },
    });
  }
  async getAlerts(): Promise<ListResponse<AlertRule>> { return this.request("/api/v1/alerts"); }
  async createAlert(input: AlertInput): Promise<AlertRule> {
    return this.request("/api/v1/alerts", {
      method: "POST",
      body: JSON.stringify(input),
    });
  }
  async updateAlert(alertId: string, input: AlertInput): Promise<AlertRule> {
    return this.request(`/api/v1/alerts/${encodeURIComponent(alertId)}`, {
      method: "PUT",
      body: JSON.stringify(input),
    });
  }
  async deleteAlert(alertId: string): Promise<void> {
    return this.request(`/api/v1/alerts/${encodeURIComponent(alertId)}`, {
      method: "DELETE",
    });
  }
  async toggleAlert(alertId: string): Promise<AlertRule> {
    return this.request(`/api/v1/alerts/${encodeURIComponent(alertId)}/toggle`, {
      method: "POST",
    });
  }
  async getExperiments(): Promise<ListResponse<Experiment>> { return this.request("/api/v1/experiments"); }
  async createExperiment(input: ExperimentInput): Promise<Experiment> {
    return this.request("/api/v1/experiments", {
      method: "POST",
      body: JSON.stringify(experimentPayload(input)),
    });
  }
  async getExperiment(experimentId: string): Promise<Experiment> {
    return this.request(`/api/v1/experiments/${encodeURIComponent(experimentId)}`);
  }
  async updateExperimentStatus(experimentId: string, status: string): Promise<Experiment> {
    return this.request(`/api/v1/experiments/${encodeURIComponent(experimentId)}/status`, {
      method: "PUT",
      body: JSON.stringify({ status }),
    });
  }
  async deleteExperiment(experimentId: string): Promise<void> {
    await this.request(`/api/v1/experiments/${encodeURIComponent(experimentId)}`, {
      method: "DELETE",
    });
  }
  async getExperimentResults(experimentId: string, params: StatsQuery): Promise<ExperimentResults> {
    return this.request(`/api/v1/experiments/${encodeURIComponent(experimentId)}/results`, {
      params: {
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
      },
    });
  }
  async assignExperiment(experimentId: string, visitorId: string): Promise<ExperimentAssignment> {
    return this.request(`/api/v1/experiments/${encodeURIComponent(experimentId)}/assign`, {
      method: "POST",
      body: JSON.stringify({ visitor_id: visitorId }),
    });
  }
  async getActiveSurveys(): Promise<ListResponse<Survey>> { return this.request("/api/v1/surveys/active"); }
  async getSurveyNps(surveyId: string, params?: { questionId?: string }): Promise<SurveyNpsReport> {
    return this.request(`/api/v1/surveys/${encodeURIComponent(surveyId)}/nps`, {
      params: {
        ...(params?.questionId ? { question_id: params.questionId } : {}),
      },
    });
  }
  async getSurveySentiment(surveyId: string, params?: { questionId?: string }): Promise<SurveySentimentReport> {
    return this.request(`/api/v1/surveys/${encodeURIComponent(surveyId)}/sentiment`, {
      params: {
        ...(params?.questionId ? { question_id: params.questionId } : {}),
      },
    });
  }
  async getGuides(): Promise<ListResponse<InAppGuide>> { return this.request("/api/v1/guides"); }
  async getActiveGuides(): Promise<ListResponse<InAppGuide>> { return this.request("/api/v1/guides/active"); }
  async getGuide(guideId: string): Promise<InAppGuide> {
    return this.request(`/api/v1/guides/${encodeURIComponent(guideId)}`);
  }
  async createGuide(input: GuideInput): Promise<InAppGuide> {
    return this.request("/api/v1/guides", {
      method: "POST",
      body: JSON.stringify(guidePayload(input)),
    });
  }
  async updateGuide(guideId: string, input: GuideInput): Promise<InAppGuide> {
    return this.request(`/api/v1/guides/${encodeURIComponent(guideId)}`, {
      method: "PUT",
      body: JSON.stringify(guidePayload(input)),
    });
  }
  async deleteGuide(guideId: string): Promise<void> {
    await this.request(`/api/v1/guides/${encodeURIComponent(guideId)}`, {
      method: "DELETE",
    });
  }
  async updateGuideStatus(guideId: string, status: string): Promise<InAppGuide> {
    return this.request(`/api/v1/guides/${encodeURIComponent(guideId)}/status`, {
      method: "PUT",
      body: JSON.stringify({ status }),
    });
  }
  async recordGuideEvent(guideId: string, input: GuideEventInput): Promise<GuideEvent> {
    return this.request(`/api/v1/guides/${encodeURIComponent(guideId)}/events`, {
      method: "POST",
      body: JSON.stringify(guideEventPayload(input)),
    });
  }
  async getGuideEvents(guideId: string, params?: { limit?: number; offset?: number }): Promise<ListResponse<GuideEvent>> {
    return this.request(`/api/v1/guides/${encodeURIComponent(guideId)}/events`, {
      params: {
        ...(params?.limit ? { limit: String(params.limit) } : {}),
        ...(params?.offset ? { offset: String(params.offset) } : {}),
      },
    });
  }
  async getGuideStats(guideId: string): Promise<GuideStats> {
    return this.request(`/api/v1/guides/${encodeURIComponent(guideId)}/stats`);
  }
}

export interface PulseConfig {
  apiKey: string;
  apiUrl?: string;
  autoTrack?: boolean;
  respectDnt?: boolean;
  consentMode?: string;
  consentGranted?: boolean;
  release?: string;
  environment?: string;
  debug?: boolean;
  batch?: boolean;
  batchSize?: number;
  batchFlushIntervalMs?: number;
  // Module feature flags
  trackUtm?: boolean;
  trackScrollDepth?: boolean;
  trackWebVitals?: boolean;
  trackOutlinks?: boolean;
  trackErrors?: boolean;
  trackClicks?: boolean;
  trackSearch?: boolean;
  trackSessionReplay?: boolean;
  sessionReplaySampleRate?: number;
  maskReplayText?: boolean;
  searchParam?: string;
}

export interface StatsQuery {
  startAt: string | Date;
  endAt: string | Date;
}

export interface PaginatedQuery extends StatsQuery {
  limit?: number;
  offset?: number;
}

export interface MetricWithPrev {
  value: number;
  prev: number;
}

export interface StatsResponse {
  pageviews: MetricWithPrev;
  visitors: MetricWithPrev;
  sessions: MetricWithPrev;
  bounce_rate: MetricWithPrev;
  avg_duration: MetricWithPrev;
  events: MetricWithPrev;
}

export interface PageData {
  path: string;
  views: number;
  unique_views: number;
  avg_duration: number;
}

export interface ReferrerData {
  referrer_domain: string;
  visits: number;
}

export interface EventData {
  event_name: string;
  count: number;
}

export interface DeviceData {
  browser: string;
  os: string;
  device: string;
  visitors: number;
}

export interface GeoData {
  country: string;
  visitors: number;
}

export interface RealtimeResponse {
  active_visitors: number;
}

export interface PulseBatchEvent {
  type: string;
  payload: Record<string, unknown>;
  visitorId: string;
  timestamp?: number;
  consentMode?: string;
  consentGranted?: boolean;
}

export interface PulseBatchResponse {
  ok: boolean;
  received: number;
  tracked: number;
  skipped: number;
  failed: number;
  errors: Array<{ index: number; error: string }>;
}

export interface ListResponse<T> {
  data: T[];
}

export interface UserProfile {
  id: string;
  project_id: string;
  visitor_id: string;
  user_id: string | null;
  traits: Record<string, unknown>;
  first_seen_at: string;
  last_seen_at: string;
  identified_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface UserAlias {
  id: string;
  project_id: string;
  user_id: string;
  visitor_id: string;
  created_at: string;
}

export interface IdentityGraphNode {
  id: string;
  node_type: "visitor" | "user" | "account" | string;
  key: string;
  label: string | null;
  metadata: Record<string, unknown>;
}

export interface IdentityGraphEdge {
  id: string;
  source: string;
  target: string;
  edge_type: "alias" | "identified_as" | "member_of" | string;
  metadata: Record<string, unknown>;
}

export interface IdentityGraph {
  nodes: IdentityGraphNode[];
  edges: IdentityGraphEdge[];
}

export interface IdentityGraphQuery {
  visitorId?: string;
  userId?: string;
  accountId?: string;
  limit?: number;
}

export interface AccountProfile {
  id: string;
  project_id: string;
  account_id: string;
  name: string | null;
  traits: Record<string, unknown>;
  first_seen_at: string;
  last_seen_at: string;
  created_at: string;
  updated_at: string;
}

export interface AccountMembership {
  id: string;
  project_id: string;
  account_id: string;
  user_id: string | null;
  visitor_id: string;
  role: string | null;
  traits: Record<string, unknown>;
  first_seen_at: string;
  last_seen_at: string;
  created_at: string;
  updated_at: string;
}

export interface AccountAnalytics {
  account_id: string;
  name: string | null;
  start_at: string;
  end_at: string;
  members: number;
  identified_users: number;
  sessions: number;
  pageviews: number;
  events: number;
  revenue: number;
  last_seen_at: string;
}

export interface ScimUser {
  id: string;
  project_id: string;
  user_name: string;
  external_id: string | null;
  active: boolean;
  display_name: string | null;
  given_name: string | null;
  family_name: string | null;
  emails: unknown[];
  traits: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface ScimUserInput {
  userName: string;
  externalId?: string;
  active?: boolean;
  displayName?: string;
  givenName?: string;
  familyName?: string;
  emails?: unknown[];
  traits?: Record<string, unknown>;
}

export interface ScimGroup {
  id: string;
  project_id: string;
  display_name: string;
  external_id: string | null;
  traits: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface ScimGroupInput {
  displayName: string;
  externalId?: string;
  traits?: Record<string, unknown>;
  members?: string[];
}

export interface ScimGroupWithMembers {
  group: ScimGroup;
  members: ScimUser[];
}

export interface IdentifyAccountOptions {
  accountId?: string;
  accountName?: string;
  accountTraits?: Record<string, unknown>;
  accountRole?: string;
}

export interface SessionReplayEvent {
  type: string;
  t: number;
  [key: string]: unknown;
}

export interface SessionRecordingSummary {
  id: string;
  session_id: string;
  visitor_id: string;
  events_count: number;
  started_at: string;
  duration_ms: number | null;
  entry_page: string | null;
  browser: string | null;
  os: string | null;
  device: string | null;
  country: string | null;
  screen: string | null;
  is_complete: boolean | null;
  created_at: string;
}

export interface SessionRecording extends SessionRecordingSummary {
  project_id: string;
  events_data: SessionReplayEvent[];
}

export interface EmailReportConfig {
  id: string;
  project_id: string;
  name: string;
  recipients: string[];
  schedule: "daily" | "weekly" | "monthly" | string;
  modules: string[];
  is_active: boolean | null;
  last_sent_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface EmailReportListResponse extends ListResponse<EmailReportConfig> {
  delivery_configured: boolean;
}

export interface DsarDeleteResult {
  visitor_id: string;
  deleted: Record<string, number>;
}

export interface AuditLog {
  id: string;
  project_id: string;
  actor: string;
  action: string;
  target_type: string;
  target_id: string | null;
  metadata: Record<string, unknown>;
  created_at: string;
}

export interface PrivacySettings {
  project_id: string;
  anonymize_ip: boolean;
  respect_dnt: boolean;
  bot_filtering: boolean;
  consent_required: boolean;
  allowed_consent_modes: string[];
  blocked_user_agents: string[];
  created_at: string;
  updated_at: string;
}

export interface SegmentCondition {
  source: "profile" | "identity" | "user" | "session" | "pageview" | "event" | "metric" | string;
  field?: string;
  op: "exists" | "not_exists" | "eq" | "neq" | "contains" | "starts_with" | "ends_with" | "gt" | "gte" | "lt" | "lte" | "in" | string;
  value?: unknown;
  event?: string;
}

export interface SegmentDefinition {
  match?: "all" | "any";
  conditions: SegmentCondition[];
}

export interface SavedSegment {
  id: string;
  project_id: string;
  name: string;
  description: string | null;
  definition: SegmentDefinition;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface SegmentEvaluation {
  segment_id: string;
  total_visitors: number;
  visitors: string[];
}

export interface SegmentCompareRow {
  segment_id: string;
  name: string;
  visitors: number;
  pageviews: number;
  sessions: number;
  events: number;
  conversions: number;
}

export interface SegmentBreakdownRow {
  value: string;
  visitors: number;
}

export type TrackingPlanEnforcementMode = "observe" | "reject" | string;
export type EventSchemaStatus = "draft" | "approved" | "deprecated" | string;
export type DataDictionaryEntryType = "event" | "property" | "metric" | "dimension" | string;

export interface TrackingPlan {
  id: string;
  project_id: string;
  name: string;
  description: string | null;
  enforcement_mode: TrackingPlanEnforcementMode;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export type EventPropertyRule = string | {
  type: string;
  required?: boolean;
  description?: string;
  pii?: boolean;
};

export interface EventSchemaDefinition {
  id: string;
  project_id: string;
  tracking_plan_id: string | null;
  event_name: string;
  description: string | null;
  status: EventSchemaStatus;
  required_properties: string[];
  property_schema: Record<string, EventPropertyRule>;
  created_at: string;
  updated_at: string;
}

export interface DataDictionaryEntry {
  id: string;
  project_id: string;
  entry_type: DataDictionaryEntryType;
  name: string;
  data_type: string | null;
  description: string | null;
  owner: string | null;
  is_pii: boolean;
  created_at: string;
  updated_at: string;
}

export interface EventQualityViolation {
  id: string;
  project_id: string;
  tracking_plan_id: string | null;
  event_schema_id: string | null;
  event_name: string;
  visitor_id: string | null;
  violation_type: string;
  message: string;
  details: Record<string, unknown>;
  created_at: string;
}

export interface EventQualitySummaryRow {
  event_name: string;
  violation_type: string;
  count: number;
}

export interface GovernanceHealth {
  status: "healthy" | "warning" | "critical" | "not_configured" | string;
  active_tracking_plan: TrackingPlan | null;
  observed_events_24h: number;
  covered_events_24h: number;
  coverage_ratio: number;
  approved_event_schemas: number;
  violations_24h: number;
  unknown_events_24h: number;
  unapproved_events_24h: number;
  missing_property_violations_24h: number;
  type_mismatch_violations_24h: number;
  top_violations: EventQualitySummaryRow[];
}

export type FeatureFlagType = "boolean" | "string" | "number" | "json" | string;

export interface FeatureFlagVariant {
  name: string;
  weight?: number;
  value?: unknown;
}

export interface TargetingCondition {
  field: string;
  op: "exists" | "not_exists" | "eq" | "neq" | "contains" | "starts_with" | "ends_with" | "gt" | "gte" | "lt" | "lte" | "in" | string;
  value?: unknown;
}

export interface TargetingRules {
  match?: "all" | "any";
  conditions: TargetingCondition[];
}

export interface FeatureFlag {
  id: string;
  project_id: string;
  key: string;
  name: string;
  description: string | null;
  enabled: boolean;
  flag_type: FeatureFlagType;
  default_value: unknown;
  variants: FeatureFlagVariant[];
  rollout_percentage: number;
  targeting_rules: TargetingRules;
  remote_config: Record<string, unknown>;
  experiment_id: string | null;
  guardrail_metrics: unknown[];
  created_at: string;
  updated_at: string;
}

export interface FeatureFlagEvaluationContext {
  visitorId: string;
  userId?: string;
  traits?: Record<string, unknown>;
  context?: Record<string, unknown>;
}

export interface FeatureFlagEvaluationResult {
  key: string;
  enabled: boolean;
  matched: boolean;
  variant: string | null;
  value: unknown;
  reason: string;
  experiment_id: string | null;
}

export interface FeatureFlagEvaluation {
  id: string;
  project_id: string;
  flag_id: string;
  visitor_id: string;
  user_id: string | null;
  matched: boolean;
  enabled: boolean;
  variant: string | null;
  value: unknown;
  reason: string;
  context: Record<string, unknown>;
  created_at: string;
}

export interface RemoteConfigEntry {
  id: string;
  project_id: string;
  key: string;
  description: string | null;
  value: unknown;
  targeting_rules: TargetingRules;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface RemoteConfigEvaluationResult {
  key: string;
  matched: boolean;
  value: unknown;
  reason: string;
}

// --- New module types ---

export interface FunnelStep {
  type: "url" | "event";
  value: string;
  label?: string;
}

export interface Funnel {
  id: string;
  project_id: string;
  name: string;
  steps: FunnelStep[];
  created_at: string;
  updated_at: string;
}

export interface FunnelResult {
  step: string;
  label: string;
  visitors: number;
  dropoff_rate: number;
}

export interface Goal {
  id: string;
  project_id: string;
  name: string;
  goal_type: string;
  config: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface GoalStats {
  conversions: number;
  unique_visitors: number;
  total_revenue: number;
  conversion_rate: number;
}

export interface RetentionCohort {
  date: string;
  total_visitors: number;
  returning: { period: number; visitors: number; percentage: number }[];
}

export interface CohortGroup {
  period_start: string;
  size: number;
  data: { period_offset: number; value: number }[];
}

export interface PageFlow {
  from_path: string;
  to_path: string;
  transitions: number;
  percentage: number;
}

export interface CampaignStat {
  utm_source: string;
  utm_medium: string;
  utm_campaign: string;
  visitors: number;
  sessions: number;
  pageviews: number;
  bounce_rate: number;
}

export interface MarketingChannelStat {
  channel: string;
  visitors: number;
  sessions: number;
  pageviews: number;
  percentage: number;
}

export interface AttributionRow {
  model: string;
  channel: string;
  source: string | null;
  campaign: string | null;
  conversions: number;
  revenue: number;
}

export interface RevenueByCurrency {
  currency: string;
  orders: number;
  revenue: number;
}

export interface ProductRevenue {
  product_id: string;
  product_name: string;
  orders: number;
  revenue: number;
}

export interface EcommerceReport {
  start_at: string;
  end_at: string;
  orders: number;
  revenue: number;
  average_order_value: number;
  currency_breakdown: RevenueByCurrency[];
  top_products: ProductRevenue[];
}

export interface AiReferrerStat {
  referrer_domain: string;
  provider: string;
  visitors: number;
  sessions: number;
  pageviews: number;
}

export interface MarketingImport {
  id: string;
  project_id: string;
  provider: "google_analytics" | "google_ads" | "search_console" | string;
  name: string;
  row_count: number;
  imported_by: string | null;
  metadata: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface MarketingImportRow {
  id: number;
  import_id: string;
  project_id: string;
  row_number: number;
  row_date: string | null;
  dimensions: Record<string, unknown>;
  metrics: Record<string, number>;
  raw_row: Record<string, unknown>;
  created_at: string;
}

export interface MarketingImportRowInput {
  date?: string;
  dimensions?: Record<string, unknown>;
  metrics?: Record<string, number>;
  rawRow?: Record<string, unknown>;
}

export interface MarketingImportInput {
  provider: "google_analytics" | "google_ads" | "search_console" | "ga4" | "google_search_console" | string;
  name: string;
  rows: MarketingImportRowInput[];
  importedBy?: string;
  metadata?: Record<string, unknown>;
}

export interface MarketingImportSummary {
  provider: string | null;
  start_date: string;
  end_date: string;
  rows: number;
  impressions: number;
  clicks: number;
  cost: number;
  conversions: number;
  revenue: number;
  sessions: number;
  users: number;
}

export interface WebVitalSummary {
  metric_name: string;
  p50: number;
  p75: number;
  p99: number;
  good: number;
  needs_improvement: number;
  poor: number;
  total: number;
}

export interface ClickHeatmapPoint {
  x: number;
  y: number;
  count: number;
  element_selector: string | null;
}

export interface PageClickStats {
  path: string;
  total_clicks: number;
  unique_visitors: number;
}

export interface FrictionSignal {
  signal_type: "rage_click" | string;
  severity: "medium" | "high" | string;
  path: string;
  element_selector: string | null;
  visitor_id: string;
  session_id: string | null;
  occurrences: number;
  first_seen_at: string;
  last_seen_at: string;
}

export interface VisualEventLabel {
  id: string;
  project_id: string;
  name: string;
  event_name: string;
  path_pattern: string;
  element_selector: string;
  properties: Record<string, unknown>;
  status: "active" | "paused" | "archived" | string;
  created_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface VisualEventLabelInput {
  name: string;
  eventName: string;
  pathPattern?: string;
  elementSelector: string;
  properties?: Record<string, unknown>;
  status?: "active" | "paused" | "archived" | string;
  createdBy?: string;
}

export interface VisualEventLabelStats {
  label_id: string;
  name: string;
  event_name: string;
  path_pattern: string;
  element_selector: string;
  total_clicks: number;
  unique_visitors: number;
  first_seen_at: string | null;
  last_seen_at: string | null;
}

export interface ErrorGroup {
  fingerprint: string;
  message: string;
  count: number;
  affected_visitors: number;
  first_seen: string;
  last_seen: string;
  last_path?: string | null;
  last_browser?: string | null;
  release?: string | null;
  environment?: string | null;
  source_map_configured?: boolean;
}

export interface MatchedSourceMap {
  id: string;
  release_version: string;
  environment: string;
  minified_url: string;
  source_map_url: string | null;
}

export interface ErrorInstance {
  id: number;
  visitor_id: string;
  session_id: string;
  message: string;
  stack: string | null;
  filename: string | null;
  lineno: number | null;
  colno: number | null;
  path: string | null;
  browser: string | null;
  os: string | null;
  release: string | null;
  environment: string | null;
  fingerprint: string | null;
  matched_source_map: MatchedSourceMap | null;
  created_at: string;
}

export interface AppRelease {
  id: string;
  project_id: string;
  version: string;
  environment: string;
  commit_sha: string | null;
  deployed_at: string | null;
  metadata: Record<string, unknown>;
  created_at: string;
}

export interface SourceMapArtifact {
  id: string;
  project_id: string;
  release_id: string | null;
  release_version: string;
  environment: string;
  minified_url: string;
  source_map_url: string | null;
  artifacts: Record<string, unknown>;
  uploaded_by: string | null;
  created_at: string;
}

export interface LogEntry {
  id: number;
  project_id: string;
  visitor_id: string | null;
  session_id: string | null;
  level: string;
  message: string;
  body: Record<string, unknown>;
  path: string | null;
  release: string | null;
  environment: string | null;
  browser: string | null;
  os: string | null;
  created_at: string;
}

export interface LogStats {
  total: number;
  levels: { level: string; count: number }[];
  releases: { release: string; count: number }[];
}

export interface AiInsight {
  title: string;
  summary: string;
  severity: "info" | "warning" | "critical" | string;
  metric: string;
  evidence: Record<string, unknown>;
}

export interface AiQueryRun {
  id: string;
  project_id: string;
  question: string;
  intent: string;
  answer: string;
  result: Record<string, unknown>;
  insights: AiInsight[];
  start_at: string;
  end_at: string;
  created_at: string;
}

export interface AiQueryResponse {
  id: string;
  question: string;
  intent: string;
  answer: string;
  result: Record<string, unknown>;
  insights: AiInsight[];
  start_at: string;
  end_at: string;
}

export interface LlmTrace {
  id: string;
  project_id: string;
  trace_key: string;
  name: string | null;
  user_id: string | null;
  visitor_id: string | null;
  session_id: string | null;
  metadata: Record<string, unknown>;
  status: "started" | "success" | "error" | "cancelled" | string;
  started_at: string;
  ended_at: string | null;
  duration_ms: number | null;
  created_at: string;
  updated_at: string;
}

export interface LlmTraceInput {
  traceKey: string;
  name?: string;
  userId?: string;
  visitorId?: string;
  sessionId?: string;
  metadata?: Record<string, unknown>;
  status?: "started" | "success" | "error" | "cancelled" | string;
  startedAt?: string | Date;
  endedAt?: string | Date;
  durationMs?: number;
}

export interface LlmGeneration {
  id: string;
  project_id: string;
  trace_id: string | null;
  trace_key: string | null;
  provider: string;
  model: string;
  operation: string;
  prompt: unknown;
  completion: unknown;
  input_tokens: number;
  output_tokens: number;
  total_tokens: number;
  latency_ms: number | null;
  cost_usd: number;
  status: "success" | "error" | "cancelled" | string;
  error_message: string | null;
  metadata: Record<string, unknown>;
  created_at: string;
}

export interface LlmGenerationInput {
  traceId?: string;
  traceKey?: string;
  provider: string;
  model: string;
  operation?: string;
  prompt?: unknown;
  completion?: unknown;
  inputTokens?: number;
  outputTokens?: number;
  totalTokens?: number;
  latencyMs?: number;
  costUsd?: number;
  status?: "success" | "error" | "cancelled" | string;
  errorMessage?: string;
  metadata?: Record<string, unknown>;
}

export interface LlmEvaluation {
  id: string;
  project_id: string;
  generation_id: string | null;
  trace_id: string | null;
  trace_key: string | null;
  evaluator: string;
  metric: string;
  score: number | null;
  label: string | null;
  passed: boolean | null;
  metadata: Record<string, unknown>;
  created_at: string;
}

export interface LlmEvaluationInput {
  generationId?: string;
  traceId?: string;
  traceKey?: string;
  evaluator: string;
  metric: string;
  score?: number;
  label?: string;
  passed?: boolean;
  metadata?: Record<string, unknown>;
}

export interface LlmStats {
  total_generations: number;
  error_generations: number;
  total_tokens: number;
  avg_latency_ms: number;
  total_cost_usd: number;
  evaluation_count: number;
  evaluation_pass_rate: number;
}

export interface CustomDashboard {
  id: string;
  project_id: string;
  name: string;
  description: string | null;
  layout: Record<string, unknown>;
  widgets: unknown[];
  is_default: boolean;
  created_at: string;
  updated_at: string;
}

export interface SavedReport {
  id: string;
  project_id: string;
  name: string;
  description: string | null;
  report_type: string;
  params: Record<string, unknown>;
  visualization: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface QueryExplorerRun {
  id: string;
  project_id: string;
  report_type: string;
  query: Record<string, unknown>;
  result: Record<string, unknown>;
  row_count: number;
  created_at: string;
}

export interface QueryExplorerRequest {
  report_type: "stats" | "timeseries" | "pages" | "referrers" | "events" | "devices" | "geo" | "campaigns" | string;
  start_at?: string;
  end_at?: string;
  limit?: number;
  offset?: number;
  params?: Record<string, unknown>;
}

export interface QueryExplorerResponse {
  run: QueryExplorerRun;
  summary: string;
}

export interface ProductStickinessPeriod {
  date: string;
  active_visitors: number;
}

export interface ProductStickinessReport {
  start_at: string;
  end_at: string;
  dau: number;
  wau: number;
  mau: number;
  dau_wau: number;
  wau_mau: number;
  dau_mau: number;
  periods: ProductStickinessPeriod[];
}

export interface ProductLifecycleReport {
  start_at: string;
  end_at: string;
  previous_start_at: string;
  previous_end_at: string;
  active_visitors: number;
  new_visitors: number;
  returning_visitors: number;
  resurrected_visitors: number;
  dormant_visitors: number;
}

export interface ProductActivationRequest {
  startAt: string | Date;
  endAt: string | Date;
  eventNames?: string[];
  paths?: string[];
}

export interface ProductActivationReport {
  start_at: string;
  end_at: string;
  cohort_visitors: number;
  activated_visitors: number;
  activation_rate: number;
  required_events: string[];
  required_paths: string[];
}

export interface ProductImpactRequest {
  metric: "pageviews" | "visitors" | "sessions" | "events" | "errors" | string;
  splitAt: string | Date;
  windowDays?: number;
  eventName?: string;
}

export interface ProductImpactReport {
  metric: string;
  event_name: string | null;
  split_at: string;
  before_start_at: string;
  before_end_at: string;
  after_start_at: string;
  after_end_at: string;
  before_value: number;
  after_value: number;
  absolute_change: number;
  percent_change: number;
  direction: "up" | "down" | "flat" | string;
  summary: string;
}

export interface Integration {
  key: string;
  name: string;
  category: string;
  status: "available" | "planned" | string;
  description: string;
  capabilities: string[];
  setup: Record<string, unknown>;
}

export interface IntegrationFilter {
  category?: string;
  capability?: string;
  status?: string;
}

export interface EventSource {
  id: string;
  project_id: string;
  name: string;
  source_type: string;
  description: string | null;
  token_prefix: string;
  schema: Record<string, unknown>;
  config: Record<string, unknown>;
  is_active: boolean;
  last_received_at: string | null;
  failure_count: number;
  created_at: string;
  updated_at: string;
}

export interface SourceInput {
  name: string;
  sourceType?: "webhook" | string;
  description?: string;
  schema?: Record<string, unknown>;
  config?: Record<string, unknown>;
  isActive?: boolean;
}

export interface SourceWithToken {
  source: EventSource;
  token: string;
}

export interface SourceIngestion {
  id: string;
  project_id: string;
  source_id: string;
  event_type: string;
  payload: Record<string, unknown>;
  headers: Record<string, unknown>;
  status: "accepted" | "rejected" | string;
  error_message: string | null;
  destination_deliveries: number;
  received_at: string;
}

export interface SourceIngestResponse {
  ok: boolean;
  ingestion_id: string;
  event_type: string;
  destination_deliveries: number;
}

export interface DestinationTransform {
  include?: string[];
  include_fields?: string[];
  exclude?: string[];
  drop_fields?: string[];
  rename?: Record<string, string>;
  rename_fields?: Record<string, string>;
  set?: Record<string, unknown>;
  static_fields?: Record<string, unknown>;
  wrap?: string;
  wrap_key?: string;
}

export interface Destination {
  id: string;
  project_id: string;
  name: string;
  destination_type: string;
  endpoint_url: string;
  secret: string | null;
  headers: Record<string, unknown>;
  event_types: string[];
  transform: DestinationTransform | Record<string, unknown>;
  is_active: boolean;
  last_success_at: string | null;
  last_failure_at: string | null;
  failure_count: number;
  created_at: string;
  updated_at: string;
}

export interface DestinationInput {
  name: string;
  destinationType?: "webhook" | string;
  endpointUrl: string;
  secret?: string;
  headers?: Record<string, unknown>;
  eventTypes?: string[];
  transform?: DestinationTransform | Record<string, unknown>;
  isActive?: boolean;
}

export interface DestinationDelivery {
  id: string;
  project_id: string;
  destination_id: string;
  event_type: string;
  status: "pending" | "retry" | "delivered" | "dead_letter" | string;
  payload: Record<string, unknown>;
  attempts: number;
  response_status: number | null;
  response_body: string | null;
  error_message: string | null;
  next_retry_at: string;
  delivered_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface DestinationHealth {
  destination_id: string;
  name: string;
  destination_type: string;
  is_active: boolean;
  status: "healthy" | "failing" | "degraded" | "disabled" | string;
  last_success_at: string | null;
  last_failure_at: string | null;
  failure_count: number;
  total_deliveries: number;
  pending_deliveries: number;
  retry_deliveries: number;
  delivered_deliveries: number;
  dead_letter_deliveries: number;
}

export interface SemanticMetric {
  id: string;
  project_id: string;
  key: string;
  name: string;
  description: string | null;
  dataset: string;
  expression: string;
  filters: Record<string, unknown>;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface SemanticMetricInput {
  key: string;
  name: string;
  description?: string;
  dataset: "pageviews" | "events" | "sessions" | "daily_stats" | "csv_uploads" | string;
  expression: string;
  filters?: Record<string, unknown>;
  isActive?: boolean;
}

export interface BiDatabaseConnection {
  id: string;
  project_id: string;
  name: string;
  database_type: "postgres" | "clickhouse" | "http_json" | string;
  connection_string_masked: string;
  allowed_schemas: string[];
  is_active: boolean;
  last_tested_at: string | null;
  last_error: string | null;
  created_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface BiDatabaseConnectionInput {
  name: string;
  databaseType?: "postgres" | "clickhouse" | "http_json" | string;
  connectionString: string;
  allowedSchemas?: string[];
  isActive?: boolean;
  createdBy?: string;
}

export interface BiConnectionTestResponse {
  connection: BiDatabaseConnection;
  ok: boolean;
  error: string | null;
}

export interface BiEmbed {
  id: string;
  project_id: string;
  name: string;
  resource_type: "dashboard" | "report" | "sql_query" | "visual_query" | "metric" | string;
  resource_id: string | null;
  resource_config: Record<string, unknown>;
  allowed_origins: string[];
  theme: Record<string, unknown>;
  token_prefix: string;
  is_active: boolean;
  expires_at: string | null;
  last_accessed_at: string | null;
  access_count: number;
  created_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface BiEmbedInput {
  name: string;
  resourceType: "dashboard" | "report" | "sql_query" | "visual_query" | "metric" | string;
  resourceId?: string;
  resourceConfig?: Record<string, unknown>;
  allowedOrigins?: string[];
  theme?: Record<string, unknown>;
  isActive?: boolean;
  expiresAt?: string | Date;
  createdBy?: string;
}

export interface BiEmbedWithToken {
  embed: BiEmbed;
  token: string;
  embed_url: string;
}

export interface BiEmbedResolved {
  embed: BiEmbed;
  resource: Record<string, unknown>;
  result: Record<string, unknown> | null;
}

export interface BiRowPolicy {
  id: string;
  project_id: string;
  name: string;
  dataset: "pageviews" | "events" | "sessions" | "daily_stats" | "csv_uploads" | string;
  field: string;
  operator: "eq" | "neq" | "in" | "not_in" | string;
  values: unknown[];
  is_active: boolean;
  created_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface BiRowPolicyInput {
  name: string;
  dataset: "pageviews" | "events" | "sessions" | "daily_stats" | "csv_uploads" | string;
  field: string;
  operator?: "eq" | "neq" | "in" | "not_in" | string;
  values: unknown[];
  isActive?: boolean;
  createdBy?: string;
}

export interface SavedSqlQuery {
  id: string;
  project_id: string;
  name: string;
  description: string | null;
  sql_text: string;
  parameters: Record<string, unknown>;
  created_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface SavedSqlInput {
  name: string;
  description?: string;
  sqlText: string;
  parameters?: Record<string, unknown>;
  createdBy?: string;
}

export interface BiQueryRun {
  id: string;
  project_id: string;
  query_id: string | null;
  query_type: "sql" | "saved_sql" | "visual" | "drill_through" | "external_sql" | string;
  sql_text: string;
  result: Record<string, unknown>[];
  row_count: number;
  duration_ms: number;
  status: "success" | "error" | string;
  error_message: string | null;
  created_at: string;
}

export interface BiQueryResponse {
  run: BiQueryRun;
  rows: Record<string, unknown>[];
}

export interface BiSqlRunRequest {
  sqlText: string;
  limit?: number;
}

export interface BiExternalSqlRunRequest {
  sqlText: string;
  limit?: number;
}

export interface BiVisualQueryRequest {
  dataset: "pageviews" | "events" | "sessions" | "daily_stats" | string;
  dimensions?: string[];
  metrics?: string[];
  startAt?: string | Date;
  endAt?: string | Date;
  limit?: number;
}

export interface BiDrillThroughRequest {
  dataset: "pageviews" | "events" | "sessions" | "daily_stats" | "csv_uploads" | string;
  filters?: Record<string, unknown>;
  startAt?: string | Date;
  endAt?: string | Date;
  limit?: number;
}

export interface CsvUpload {
  id: string;
  project_id: string;
  name: string;
  description: string | null;
  columns: string[];
  row_count: number;
  uploaded_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface CsvUploadInput {
  name: string;
  description?: string;
  columns: string[];
  rows: Record<string, unknown>[];
  uploadedBy?: string;
}

export interface AlertNotifyChannel {
  type: "webhook";
  url: string;
  secret?: string;
}

export interface AlertInput {
  name: string;
  module: string;
  metric: "pageviews" | "visitors" | "bounce_rate" | "error_count" | "avg_duration";
  operator: "gt" | "lt" | "gte" | "lte" | "eq";
  threshold: number;
  window_minutes?: number;
  cooldown_minutes?: number;
  notify_channels?: AlertNotifyChannel[];
}

export interface AlertRule {
  id: string;
  project_id: string;
  name: string;
  module: string;
  metric: string;
  operator: string;
  threshold: number;
  window_minutes: number;
  cooldown_minutes: number;
  notify_channels: AlertNotifyChannel[];
  is_active: boolean;
  last_triggered_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface Experiment {
  id: string;
  name: string;
  description?: string;
  variants: { name: string; weight: number }[];
  goal_id?: string;
  status: string;
  started_at?: string;
  ended_at?: string;
}

export interface ExperimentInput {
  name: string;
  description?: string;
  variants: { name: string; weight: number }[];
  goalId?: string;
}

export interface ExperimentAssignment {
  variant: string;
}

export interface ExperimentVariantResult {
  name: string;
  assignments: number;
  conversions: number;
  conversion_rate: number;
  lift_percent: number | null;
  p_value: number | null;
  confidence: number | null;
  significant: boolean;
  is_baseline: boolean;
}

export interface ExperimentResults {
  experiment_id: string;
  baseline_variant: string | null;
  winner: string | null;
  variants: ExperimentVariantResult[];
}

export interface Survey {
  id: string;
  name: string;
  questions: { type: string; text: string; options?: string[] }[];
  trigger_config: Record<string, unknown>;
  appearance: Record<string, unknown>;
  status: string;
}

export interface SurveyNpsReport {
  survey_id: string;
  question_id: string | null;
  total_responses: number;
  scored_responses: number;
  promoters: number;
  passives: number;
  detractors: number;
  nps_score: number;
}

export interface SurveySentimentExample {
  response_id: string;
  sentiment: "positive" | "neutral" | "negative" | string;
  score: number;
  text: string;
}

export interface SurveySentimentReport {
  survey_id: string;
  question_id: string | null;
  total_text_responses: number;
  positive: number;
  neutral: number;
  negative: number;
  sentiment_score: number;
  examples: SurveySentimentExample[];
}

export interface InAppGuide {
  id: string;
  project_id: string;
  name: string;
  guide_type: "tour" | "tooltip" | "onboarding" | "announcement" | "checklist" | string;
  steps: Record<string, unknown>[];
  targeting: Record<string, unknown>;
  appearance: Record<string, unknown>;
  status: "draft" | "active" | "paused" | "archived" | string;
  priority: number;
  started_at: string | null;
  ended_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface GuideInput {
  name: string;
  guideType?: "tour" | "tooltip" | "onboarding" | "announcement" | "checklist" | string;
  steps?: Record<string, unknown>[];
  targeting?: Record<string, unknown>;
  appearance?: Record<string, unknown>;
  priority?: number;
}

export interface GuideEvent {
  id: string;
  project_id: string;
  guide_id: string;
  visitor_id: string;
  event_type: "shown" | "started" | "step_viewed" | "completed" | "dismissed" | "converted" | string;
  step_id: string | null;
  metadata: Record<string, unknown>;
  path: string | null;
  created_at: string;
}

export interface GuideEventInput {
  visitorId: string;
  eventType: "shown" | "started" | "step_viewed" | "completed" | "dismissed" | "converted" | string;
  stepId?: string;
  metadata?: Record<string, unknown>;
  path?: string;
}

export interface GuideStats {
  guide_id: string;
  shown: number;
  started: number;
  completed: number;
  dismissed: number;
  converted: number;
  completion_rate: number;
  dismissal_rate: number;
}

export interface SharedDashboard {
  id: string;
  name: string;
  token: string;
  modules: string[];
  expires_at?: string;
}

export interface PulseConfig {
  apiKey: string;
  apiUrl?: string;
  autoTrack?: boolean;
  respectDnt?: boolean;
  debug?: boolean;
  // Module feature flags
  trackUtm?: boolean;
  trackScrollDepth?: boolean;
  trackWebVitals?: boolean;
  trackOutlinks?: boolean;
  trackErrors?: boolean;
  trackClicks?: boolean;
  trackSearch?: boolean;
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

export interface ListResponse<T> {
  data: T[];
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

export interface ErrorGroup {
  message: string;
  count: number;
  affected_visitors: number;
  first_seen: string;
  last_seen: string;
}

export interface AlertRule {
  id: string;
  name: string;
  module: string;
  metric: string;
  operator: string;
  threshold: number;
  window_minutes: number;
  cooldown_minutes: number;
  is_active: boolean;
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

export interface Survey {
  id: string;
  name: string;
  questions: { type: string; text: string; options?: string[] }[];
  trigger_config: Record<string, unknown>;
  appearance: Record<string, unknown>;
  status: string;
}

export interface SharedDashboard {
  id: string;
  name: string;
  token: string;
  modules: string[];
  expires_at?: string;
}

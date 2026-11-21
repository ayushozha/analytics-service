// Response shapes of the endpoints the web app uses (see crates/pulse-server).

export type Interval = 'hour' | 'day' | 'week' | 'month';
export type Math = 'total' | 'dau' | 'unique_sessions';

export interface Me { project_id: string; project_name: string }

export interface TrendSeries {
  /** `null` counts all events. */
  event: string | null;
  math: Math;
  custom_name?: string;
}

export interface TrendsQuery {
  series: TrendSeries[];
  date_from: string;
  date_to: string;
  interval: Interval;
  compare?: boolean;
  formula?: string;
  breakdown?: { property: string; type?: 'event' | 'session' | 'person'; limit?: number };
}

export interface TrendResult {
  label: string;
  series_index: number | null;
  event: string | null;
  breakdown_value: string | null;
  compare_label: 'current' | 'previous' | null;
  days: string[];
  data: number[];
  count: number;
}

export interface TrendsResponse { results: TrendResult[]; interval: Interval }

export interface FunnelStep {
  name: string;
  order: number;
  count: number;
  conversion_rate: number;
  conversion_rate_from_previous: number;
  dropoff_count: number;
  median_time_to_convert_secs: number | null;
}

export interface FunnelsResponse { results: { breakdown_value: string | null; steps: FunnelStep[] }[] }

export interface RetentionRow {
  date: string;
  label: string;
  breakdown_value: string | null;
  values: { interval: number; count: number; percentage: number }[];
}

export interface RetentionResponse { results: RetentionRow[]; period: Interval }

export interface Person {
  visitor_id: string;
  user_id: string | null;
  traits: Record<string, unknown> | null;
  first_seen_at: string | null;
  last_seen_at: string | null;
  identified_at?: string | null;
}

export interface PersonDetail extends Person {
  aliases: string[];
  stats: { events: number; pageviews: number; sessions: number };
  cohorts: { id: string; name: string }[] | null;
}

export interface Page<T> { data: T[]; limit: number; offset: number; total?: number }

export interface PersonEvent {
  created_at: string;
  event: string;
  properties: Record<string, unknown> | null;
  session_id: string | null;
}

export interface PersonSession {
  session_id: string;
  first_at: string;
  last_at: string;
  duration_ms: number;
  entry_page: string | null;
  exit_page: string | null;
  browser: string | null;
  os: string | null;
  device: string | null;
  country: string | null;
  pageviews: number;
  events: number;
}

export type DefinitionKind = 'event' | 'event_property' | 'person_property' | 'session_property';

export interface DefinitionMeta {
  description: string | null;
  owner: string | null;
  tags: string[];
  verified: boolean;
  hidden: boolean;
  is_pii: boolean;
}

export interface EventDefinition extends DefinitionMeta {
  name: string;
  volume: number;
  unique_users: number;
  last_seen_at: string | null;
}

export interface PropertyDefinition extends DefinitionMeta {
  name: string;
  kind: DefinitionKind;
  property_type: string;
  seen: number | null;
}

export interface PropertyFilter {
  key: string;
  type?: 'event' | 'session' | 'person';
  operator?: 'exact' | 'is_not' | 'icontains' | 'not_icontains' | 'is_set' | 'is_not_set';
  value?: unknown;
}

export interface ExplorerEvent {
  event: string;
  visitor_id: string;
  session_id: string | null;
  created_at: string;
  properties: Record<string, unknown>;
  person?: { user_id: string | null; traits: Record<string, unknown> | null };
}

export interface EventsPage { results: ExplorerEvent[]; next_before: string | null }

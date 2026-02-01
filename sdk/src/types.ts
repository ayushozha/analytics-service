export interface PulseConfig {
  apiKey: string;
  apiUrl?: string;
  autoTrack?: boolean;
  respectDnt?: boolean;
  debug?: boolean;
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

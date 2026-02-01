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

  async trackEvent(params: {
    visitorId: string;
    eventName: string;
    data?: Record<string, unknown>;
    path?: string;
    ip?: string;
    userAgent?: string;
  }) {
    await this.request("/api/collect", {
      method: "POST",
      headers: {
        ...(params.ip ? { "X-Forwarded-For": params.ip } : {}),
        ...(params.userAgent ? { "User-Agent": params.userAgent } : {}),
      },
      body: JSON.stringify({
        type: "event",
        payload: {
          name: params.eventName,
          data: params.data || {},
          path: params.path,
        },
        visitor_id: params.visitorId,
      }),
    });
  }

  async trackPageview(params: {
    visitorId: string;
    path: string;
    title?: string;
    referrer?: string;
    ip?: string;
    userAgent?: string;
  }) {
    await this.request("/api/collect", {
      method: "POST",
      headers: {
        ...(params.ip ? { "X-Forwarded-For": params.ip } : {}),
        ...(params.userAgent ? { "User-Agent": params.userAgent } : {}),
      },
      body: JSON.stringify({
        type: "pageview",
        payload: {
          path: params.path,
          title: params.title,
          referrer: params.referrer,
        },
        visitor_id: params.visitorId,
      }),
    });
  }

  async getStats(params: StatsQuery): Promise<StatsResponse> {
    return this.request("/api/v1/stats", {
      params: { start_at: toISO(params.startAt), end_at: toISO(params.endAt) },
    });
  }

  async getPages(params: PaginatedQuery): Promise<ListResponse<PageData>> {
    return this.request("/api/v1/pages", {
      params: {
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
        ...(params.limit ? { limit: String(params.limit) } : {}),
      },
    });
  }

  async getReferrers(params: PaginatedQuery): Promise<ListResponse<ReferrerData>> {
    return this.request("/api/v1/referrers", {
      params: {
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
        ...(params.limit ? { limit: String(params.limit) } : {}),
      },
    });
  }

  async getEvents(params: PaginatedQuery): Promise<ListResponse<EventData>> {
    return this.request("/api/v1/events", {
      params: {
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
        ...(params.limit ? { limit: String(params.limit) } : {}),
      },
    });
  }

  async getDevices(params: PaginatedQuery): Promise<ListResponse<DeviceData>> {
    return this.request("/api/v1/devices", {
      params: {
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
        ...(params.limit ? { limit: String(params.limit) } : {}),
      },
    });
  }

  async getGeo(params: PaginatedQuery): Promise<ListResponse<GeoData>> {
    return this.request("/api/v1/geo", {
      params: {
        start_at: toISO(params.startAt),
        end_at: toISO(params.endAt),
        ...(params.limit ? { limit: String(params.limit) } : {}),
      },
    });
  }

  async getRealtime(): Promise<RealtimeResponse> {
    return this.request("/api/v1/realtime");
  }
}

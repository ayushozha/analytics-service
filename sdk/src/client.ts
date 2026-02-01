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

  constructor(config: PulseConfig) {
    this.config = {
      apiKey: config.apiKey,
      apiUrl: config.apiUrl || DEFAULT_API_URL,
      autoTrack: config.autoTrack ?? (typeof window !== "undefined"),
      respectDnt: config.respectDnt ?? true,
      debug: config.debug ?? false,
    };
    this.visitorId = generateVisitorId();

    if (this.config.autoTrack && typeof window !== "undefined") {
      this.setupAutoTracking();
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

  pageview(path?: string) {
    this.send("pageview", {
      path: path || (typeof location !== "undefined" ? location.pathname + location.search : "/"),
      title: typeof document !== "undefined" ? document.title : undefined,
      referrer: typeof document !== "undefined" ? document.referrer : undefined,
      screen: typeof screen !== "undefined" ? `${screen.width}x${screen.height}` : undefined,
      language: typeof navigator !== "undefined" ? navigator.language : undefined,
    });
  }

  event(name: string, data?: Record<string, unknown>) {
    this.send("event", {
      name,
      data: data || {},
      path: typeof location !== "undefined" ? location.pathname : undefined,
    });
  }

  identify(traits: Record<string, unknown>) {
    this.send("identify", { traits });
  }

  async getStats(params: StatsQuery): Promise<StatsResponse> {
    return this.query("/api/v1/stats", {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
    });
  }

  async getPages(params: PaginatedQuery): Promise<ListResponse<PageData>> {
    return this.query("/api/v1/pages", {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
      ...(params.limit ? { limit: String(params.limit) } : {}),
      ...(params.offset ? { offset: String(params.offset) } : {}),
    });
  }

  async getReferrers(params: PaginatedQuery): Promise<ListResponse<ReferrerData>> {
    return this.query("/api/v1/referrers", {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
      ...(params.limit ? { limit: String(params.limit) } : {}),
    });
  }

  async getEvents(params: PaginatedQuery): Promise<ListResponse<EventData>> {
    return this.query("/api/v1/events", {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
      ...(params.limit ? { limit: String(params.limit) } : {}),
    });
  }

  async getDevices(params: PaginatedQuery): Promise<ListResponse<DeviceData>> {
    return this.query("/api/v1/devices", {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
      ...(params.limit ? { limit: String(params.limit) } : {}),
    });
  }

  async getGeo(params: PaginatedQuery): Promise<ListResponse<GeoData>> {
    return this.query("/api/v1/geo", {
      start_at: toISO(params.startAt),
      end_at: toISO(params.endAt),
      ...(params.limit ? { limit: String(params.limit) } : {}),
    });
  }

  async getRealtime(): Promise<RealtimeResponse> {
    return this.query("/api/v1/realtime");
  }
}

export function createPulse(config: PulseConfig): PulseClient {
  return new PulseClient(config);
}

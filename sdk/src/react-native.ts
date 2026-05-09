import type { IdentifyAccountOptions } from "./types";

const DEFAULT_API_URL = "https://pulse.ayushojha.com";

export interface PulseNativeConfig {
  apiKey: string;
  apiUrl?: string;
  visitorId: string;
  consentMode?: string;
  consentGranted?: boolean;
  release?: string;
  environment?: string;
  debug?: boolean;
}

export interface NativePageviewInput {
  path: string;
  title?: string;
  referrer?: string;
  screen?: string;
  language?: string;
}

export class PulseNativeClient {
  private apiKey: string;
  private apiUrl: string;
  private visitorId: string;
  private consentMode: string;
  private consentGranted: boolean;
  private release: string;
  private environment: string;
  private debug: boolean;

  constructor(config: PulseNativeConfig) {
    this.apiKey = config.apiKey;
    this.apiUrl = config.apiUrl || DEFAULT_API_URL;
    this.visitorId = config.visitorId;
    this.consentMode = config.consentMode || "analytics";
    this.consentGranted = config.consentGranted ?? true;
    this.release = config.release || "";
    this.environment = config.environment || "production";
    this.debug = config.debug ?? false;
  }

  setVisitorId(visitorId: string) {
    this.visitorId = visitorId;
  }

  setConsent(granted: boolean, mode = "analytics") {
    this.consentGranted = granted;
    this.consentMode = mode;
  }

  async pageview(input: NativePageviewInput) {
    return this.collect("pageview", { ...input });
  }

  async screen(name: string, data?: Record<string, unknown>) {
    return this.event("screen_view", { name, ...(data || {}) });
  }

  async event(
    name: string,
    data?: Record<string, unknown>,
    revenueAmount?: number,
    revenueCurrency?: string,
  ) {
    return this.collect("event", {
      name,
      data: data || {},
      revenue_amount: revenueAmount,
      revenue_currency: revenueAmount == null ? undefined : revenueCurrency || "USD",
    });
  }

  async identify(
    userIdOrTraits: string | Record<string, unknown>,
    traits?: Record<string, unknown>,
    account?: IdentifyAccountOptions,
  ) {
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
    return this.collect("identify", payload);
  }

  async log(
    level: "trace" | "debug" | "info" | "warn" | "error" | "fatal" | string,
    message: string,
    body?: Record<string, unknown>,
  ) {
    return this.collect("log", {
      level,
      message,
      body: body || {},
      release: this.release || undefined,
      environment: this.environment,
    });
  }

  async surveyResponse(surveyId: string, answers: unknown[], completed = true) {
    return this.collect("survey_response", {
      survey_id: surveyId,
      answers,
      completed,
    });
  }

  private async collect(type: string, payload: Record<string, unknown>) {
    const body = JSON.stringify({
      type,
      payload,
      visitor_id: this.visitorId,
      consent_mode: this.consentMode,
      consent_granted: this.consentGranted,
    });

    if (this.debug) {
      console.log("[pulse-native]", type, payload);
    }

    const response = await fetch(`${this.apiUrl}/api/collect`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Pulse-Key": this.apiKey,
      },
      body,
    });
    if (!response.ok) {
      throw new Error(`Pulse API error: ${response.status}`);
    }
    const text = await response.text();
    return text ? JSON.parse(text) : undefined;
  }
}

export function createPulseNative(config: PulseNativeConfig): PulseNativeClient {
  return new PulseNativeClient(config);
}

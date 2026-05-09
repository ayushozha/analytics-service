import { PulseClient, createPulse } from "./client";
import type { PulseConfig } from "./types";

const DEFAULT_API_URL = "https://pulse.ayushojha.com";

export interface NextPulseScriptConfig extends PulseConfig {
  id?: string;
  scriptSrc?: string;
  strategy?: string;
}

export type NextPulseScriptProps = Record<string, string | undefined>;

export function getPulseScriptProps(config: NextPulseScriptConfig): NextPulseScriptProps {
  const apiUrl = config.apiUrl || DEFAULT_API_URL;
  return compactProps({
    id: config.id || "pulse-analytics",
    src: config.scriptSrc || `${apiUrl}/api/script.js`,
    strategy: config.strategy,
    "data-key": config.apiKey,
    "data-api": apiUrl,
    "data-dnt": String(config.respectDnt !== false),
    "data-consent-mode": config.consentMode || "analytics",
    "data-consent-granted": String(config.consentGranted !== false),
    "data-release": config.release,
    "data-environment": config.environment || "production",
    "data-utm": String(config.trackUtm !== false),
    "data-scroll": String(config.trackScrollDepth === true),
    "data-vitals": String(config.trackWebVitals === true),
    "data-outlinks": String(config.trackOutlinks === true),
    "data-errors": String(config.trackErrors === true),
    "data-clicks": String(config.trackClicks === true),
    "data-search": String(config.trackSearch === true),
    "data-replay": String(config.trackSessionReplay === true),
    "data-replay-sample":
      config.sessionReplaySampleRate == null ? undefined : String(config.sessionReplaySampleRate),
    "data-replay-mask": String(config.maskReplayText !== false),
    "data-search-param": config.searchParam,
  });
}

export function createNextPulseClient(config: PulseConfig): PulseClient {
  return createPulse({ ...config, autoTrack: config.autoTrack ?? false });
}

export function trackNextPageview(client: PulseClient, url: string) {
  client.pageview(url);
}

function compactProps(props: NextPulseScriptProps): NextPulseScriptProps {
  return Object.fromEntries(
    Object.entries(props).filter(([, value]) => value !== undefined && value !== ""),
  ) as NextPulseScriptProps;
}

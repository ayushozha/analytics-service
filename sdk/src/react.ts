import {
  createContext,
  createElement,
  useContext,
  useEffect,
  useMemo,
  type ReactNode,
} from "react";

import { PulseClient, createPulse } from "./client";
import type { IdentifyAccountOptions, PulseConfig } from "./types";

export interface PulseProviderProps {
  config?: PulseConfig;
  client?: PulseClient;
  children: ReactNode;
}

export const PulseContext = createContext<PulseClient | null>(null);

export function PulseProvider({ config, client, children }: PulseProviderProps) {
  const value = useMemo(() => {
    if (client) return client;
    if (!config) {
      throw new Error("PulseProvider requires either a client or config");
    }
    return createPulse(config);
  }, [client, config]);

  return createElement(PulseContext.Provider, { value }, children);
}

export function usePulse(): PulseClient {
  const client = useContext(PulseContext);
  if (!client) {
    throw new Error("usePulse must be used within PulseProvider");
  }
  return client;
}

export function usePulsePageview(path?: string, deps: readonly unknown[] = []) {
  const pulse = usePulse();
  useEffect(() => {
    pulse.pageview(path);
  }, [pulse, path, ...deps]);
}

export function usePulseEvent(
  name: string,
  data?: Record<string, unknown>,
  deps: readonly unknown[] = [],
) {
  const pulse = usePulse();
  useEffect(() => {
    pulse.event(name, data);
  }, [pulse, name, data, ...deps]);
}

export function usePulseIdentify(
  userIdOrTraits: string | Record<string, unknown>,
  traits?: Record<string, unknown>,
  account?: IdentifyAccountOptions,
  deps: readonly unknown[] = [],
) {
  const pulse = usePulse();
  useEffect(() => {
    pulse.identify(userIdOrTraits, traits, account);
  }, [pulse, userIdOrTraits, traits, account, ...deps]);
}

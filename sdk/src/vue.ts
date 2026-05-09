import {
  inject,
  onMounted,
  provide,
  unref,
  watch,
  type App,
  type InjectionKey,
  type Ref,
} from "vue";

import { PulseClient, createPulse } from "./client";
import type { IdentifyAccountOptions, PulseConfig } from "./types";

export interface PulseVuePluginOptions {
  config?: PulseConfig;
  client?: PulseClient;
}

export const PulseKey: InjectionKey<PulseClient> = Symbol("PulseAnalytics");

export function createPulseVue(options: PulseVuePluginOptions) {
  const client = resolveClient(options);
  return {
    install(app: App) {
      app.provide(PulseKey, client);
      app.config.globalProperties.$pulse = client;
    },
  };
}

export function providePulse(options: PulseVuePluginOptions): PulseClient {
  const client = resolveClient(options);
  provide(PulseKey, client);
  return client;
}

export function usePulse(): PulseClient {
  const client = inject(PulseKey);
  if (!client) {
    throw new Error("usePulse must be used after createPulseVue or providePulse");
  }
  return client;
}

export function usePulsePageview(path?: string | Ref<string | undefined>, watchPath = false) {
  const pulse = usePulse();
  onMounted(() => {
    pulse.pageview(unref(path));
  });
  if (watchPath) {
    watch(
      () => unref(path),
      (value, previous) => {
        if (value && value !== previous) pulse.pageview(value);
      },
    );
  }
}

export function usePulseEvent(
  name: string,
  data?: Record<string, unknown> | Ref<Record<string, unknown> | undefined>,
) {
  const pulse = usePulse();
  return {
    track(extra?: Record<string, unknown>) {
      pulse.event(name, { ...(unref(data) || {}), ...(extra || {}) });
    },
  };
}

export function usePulseIdentify(
  userIdOrTraits: string | Record<string, unknown>,
  traits?: Record<string, unknown>,
  account?: IdentifyAccountOptions,
) {
  const pulse = usePulse();
  return {
    identify() {
      pulse.identify(userIdOrTraits, traits, account);
    },
  };
}

function resolveClient(options: PulseVuePluginOptions): PulseClient {
  if (options.client) return options.client;
  if (!options.config) {
    throw new Error("createPulseVue requires either a client or config");
  }
  return createPulse(options.config);
}

declare module "vue" {
  interface ComponentCustomProperties {
    $pulse: PulseClient;
  }
}

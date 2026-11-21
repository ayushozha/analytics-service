import { useEffect, useState } from 'react';
import { api } from './api';
import type { DefinitionKind, DefinitionMeta, EventDefinition, PropertyDefinition } from './types';

// Definitions change rarely; share one request per kind across all pickers on a page.
const cache = new Map<string, Promise<unknown>>();

function cached<T>(url: string): Promise<T> {
  if (!cache.has(url)) {
    const p = api<{ data: T }>(url).then((r) => r.data);
    p.catch(() => cache.delete(url));
    cache.set(url, p);
  }
  return cache.get(url) as Promise<T>;
}

/** Forget cached definitions (after an edit). */
export const invalidateDefinitions = () => cache.clear();

export const eventDefinitionsUrl = (q = '') => `/api/v2/event-definitions${q ? `?${q}` : ''}`;
export const propertyDefinitionsUrl = (kind: DefinitionKind, event?: string | null) => {
  const p = new URLSearchParams({ kind });
  if (event) p.set('event', event);
  return `/api/v2/property-definitions?${p}`;
};

export function useCached<T>(url: string | null): T | undefined {
  const [data, setData] = useState<T>();
  useEffect(() => {
    if (!url) return;
    let live = true;
    cached<T>(url).then((d) => live && setData(d), () => undefined);
    return () => {
      live = false;
    };
  }, [url]);
  return data;
}

export const useEventDefinitions = () => useCached<EventDefinition[]>(eventDefinitionsUrl());
export const usePropertyDefinitions = (kind: DefinitionKind, event?: string | null) =>
  useCached<PropertyDefinition[]>(propertyDefinitionsUrl(kind, event));

export async function saveDefinition(kind: DefinitionKind, name: string, patch: Partial<DefinitionMeta>) {
  const meta = await api<DefinitionMeta>('/api/v2/definitions', { kind, name, ...patch }, { method: 'PATCH' });
  invalidateDefinitions();
  return meta;
}

/** Comma-separated tag input → clean, unique tags. */
export const parseTags = (s: string) => [...new Set(s.split(',').map((t) => t.trim()).filter(Boolean))];

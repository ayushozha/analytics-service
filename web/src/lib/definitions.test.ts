import { afterEach, describe, expect, it, vi } from 'vitest';
import { parseTags, propertyDefinitionsUrl, saveDefinition } from './definitions';
import { describeFilter } from '../components/PropertyFilters';

afterEach(() => vi.unstubAllGlobals());

describe('definitions', () => {
  it('parses comma-separated tags into unique trimmed values', () => {
    expect(parseTags(' growth, activation ,growth,, ')).toEqual(['growth', 'activation']);
    expect(parseTags('')).toEqual([]);
  });

  it('builds property definition URLs per kind and event', () => {
    expect(propertyDefinitionsUrl('person_property')).toBe('/api/v2/property-definitions?kind=person_property');
    expect(propertyDefinitionsUrl('event_property', '$pageview')).toBe('/api/v2/property-definitions?kind=event_property&event=%24pageview');
  });

  it('saves metadata with a PATCH to /api/v2/definitions', async () => {
    const fetch = vi.fn().mockResolvedValue(new Response('{"tags":["a"]}', { status: 200 }));
    vi.stubGlobal('fetch', fetch);
    await saveDefinition('event', 'signup', { tags: ['a'], verified: true });
    const [url, init] = fetch.mock.calls[0];
    expect(url).toBe('/api/v2/definitions');
    expect(init.method).toBe('PATCH');
    expect(JSON.parse(init.body)).toEqual({ kind: 'event', name: 'signup', tags: ['a'], verified: true });
  });

  it('describes filters for chips', () => {
    expect(describeFilter({ key: 'plan', type: 'event', operator: 'exact', value: 'pro' })).toBe('plan = pro');
    expect(describeFilter({ key: 'email', type: 'person', operator: 'is_set' })).toBe('person.email is set');
  });
});

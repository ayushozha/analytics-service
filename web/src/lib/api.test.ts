import { afterEach, describe, expect, it, vi } from 'vitest';
import { ApiError, WEB_APP_HEADER, api, redirectToLogin } from './api';

afterEach(() => vi.unstubAllGlobals());

describe('api', () => {
  it('sends the web-app header and JSON body', async () => {
    const fetch = vi.fn().mockResolvedValue(new Response('{"ok":true}', { status: 200 }));
    vi.stubGlobal('fetch', fetch);
    await expect(api('/api/x', { a: 1 })).resolves.toEqual({ ok: true });
    const [, init] = fetch.mock.calls[0];
    expect(init.method).toBe('POST');
    expect(init.credentials).toBe('same-origin');
    expect(init.headers[WEB_APP_HEADER]).toBe('1');
    expect(init.body).toBe('{"a":1}');
  });

  it('surfaces the server error message', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('{"error":"Bad series"}', { status: 400 })));
    await expect(api('/api/x')).rejects.toEqual(new ApiError(400, 'Bad series'));
  });

  it('builds a login URL that returns to the current page', () => {
    const assign = vi.fn();
    redirectToLogin({ pathname: '/app/persons/a b', search: '?q=1', assign } as unknown as Location);
    expect(assign).toHaveBeenCalledWith('/dashboard/login?next=%2Fapp%2Fpersons%2Fa%20b%3Fq%3D1');
  });
});

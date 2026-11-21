// Thin client for the Pulse REST API. The web app authenticates with the
// dashboard session cookie; the server only honours that cookie when this
// header is present (CSRF guard), see `middleware::auth::WEB_APP_HEADER`.
export const WEB_APP_HEADER = 'x-pulse-web';
export const LOGIN_PATH = '/dashboard/login';

export class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message);
  }
}

/** Send the browser to the dashboard login, returning to the current page afterwards. */
export function redirectToLogin(loc: Location = window.location) {
  const next = loc.pathname + loc.search;
  loc.assign(`${LOGIN_PATH}?next=${encodeURIComponent(next)}`);
}

export async function api<T>(path: string, body?: unknown, init: RequestInit = {}): Promise<T> {
  const res = await fetch(path, {
    method: body === undefined ? 'GET' : 'POST',
    credentials: 'same-origin',
    ...init,
    headers: {
      [WEB_APP_HEADER]: '1',
      ...(body === undefined ? {} : { 'content-type': 'application/json' }),
      ...init.headers,
    },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (res.status === 401) {
    redirectToLogin();
    throw new ApiError(401, 'Signed out');
  }
  if (!res.ok) {
    const text = await res.text();
    let message = text;
    try {
      message = JSON.parse(text).error ?? text;
    } catch {
      /* plain-text error */
    }
    throw new ApiError(res.status, message || res.statusText);
  }
  return res.json() as Promise<T>;
}

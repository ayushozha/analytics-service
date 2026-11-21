const nf = new Intl.NumberFormat('en-US');
const compact = new Intl.NumberFormat('en-US', { notation: 'compact', maximumFractionDigits: 1 });

export const formatNumber = (n: number) => (Math.abs(n) >= 10_000 ? compact.format(n) : nf.format(Math.round(n * 10) / 10));
export const formatPercent = (n: number, digits = 1) => `${n.toFixed(digits)}%`;

/** Percentage change from `prev` to `cur`, or null when there is no baseline. */
export function change(cur: number, prev: number): number | null {
  if (!prev) return null;
  return ((cur - prev) / prev) * 100;
}

export function formatDuration(secs: number | null | undefined): string {
  if (secs == null) return '–';
  if (secs < 60) return `${Math.round(secs)}s`;
  if (secs < 3600) return `${Math.round(secs / 60)}m`;
  if (secs < 86400) return `${(secs / 3600).toFixed(1)}h`;
  return `${(secs / 86400).toFixed(1)}d`;
}

/** Axis/legend label for a bucket start returned by the API (naive UTC timestamps). */
export function formatBucket(iso: string, interval: string): string {
  const d = new Date(iso.endsWith('Z') ? iso : `${iso}Z`);
  const opts: Intl.DateTimeFormatOptions =
    interval === 'hour' ? { hour: 'numeric', timeZone: 'UTC' }
    : interval === 'month' ? { month: 'short', year: '2-digit', timeZone: 'UTC' }
    : { month: 'short', day: 'numeric', timeZone: 'UTC' };
  return d.toLocaleDateString('en-US', opts).replace(',', '');
}

export function timeAgo(iso: string | null | undefined, now = Date.now()): string {
  if (!iso) return '–';
  const s = Math.max(0, (now - new Date(iso).getTime()) / 1000);
  if (s < 60) return `${Math.round(s)}s ago`;
  if (s < 3600) return `${Math.round(s / 60)}m ago`;
  if (s < 86400) return `${Math.round(s / 3600)}h ago`;
  return `${Math.round(s / 86400)}d ago`;
}

export const DATE_RANGES = [
  { key: '7d', label: 'Last 7 days', days: 7, interval: 'day' },
  { key: '30d', label: 'Last 30 days', days: 30, interval: 'day' },
  { key: '90d', label: 'Last 90 days', days: 90, interval: 'week' },
] as const;
export type DateRangeKey = (typeof DATE_RANGES)[number]['key'];

/** ISO bounds for a preset range ending now. */
export function rangeBounds(key: DateRangeKey, now = new Date()) {
  const r = DATE_RANGES.find((x) => x.key === key) ?? DATE_RANGES[1];
  const from = new Date(now.getTime() - r.days * 86_400_000);
  return { date_from: from.toISOString(), date_to: now.toISOString(), interval: r.interval };
}

export function initials(name: string): string {
  const parts = name.replace(/@.*/, '').split(/[\s._-]+/).filter(Boolean);
  return ((parts[0]?.[0] ?? '?') + (parts[1]?.[0] ?? '')).toUpperCase();
}

export function personName(p: { traits: Record<string, unknown> | null; user_id: string | null; visitor_id: string }) {
  const t = p.traits ?? {};
  return String(t.name ?? t.email ?? p.user_id ?? p.visitor_id);
}

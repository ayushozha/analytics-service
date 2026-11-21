import type { ChartSeries } from '../components/LineChart';
import { formatBucket } from './format';
import type { TrendResult, TrendsResponse } from './types';

export const SERIES_COLORS = ['var(--series-1)', 'var(--series-2)', 'var(--series-3)', 'var(--series-4)', 'var(--series-5)'];
export const seriesColor = (i: number) => SERIES_COLORS[i % SERIES_COLORS.length];

export const MATH_LABELS = { total: 'Total count', dau: 'Unique users', unique_sessions: 'Unique sessions' } as const;

/** Properties offered for breakdowns: event columns plus session columns. */
export const BREAKDOWNS = [
  { property: 'path', type: 'event', label: 'Page path' },
  { property: 'browser', type: 'session', label: 'Browser' },
  { property: 'os', type: 'session', label: 'OS' },
  { property: 'device', type: 'session', label: 'Device type' },
  { property: 'country', type: 'session', label: 'Country' },
  { property: 'entry_page', type: 'session', label: 'Entry page' },
] as const;

/** Server-side name of the bucket that collects breakdown values past the limit. */
export const BREAKDOWN_OTHER = '$$_other';
export const breakdownLabel = (v: string | null) => (v === BREAKDOWN_OTHER ? 'Other' : v ?? '(none)');

export function resultLabel(r: TrendResult): string {
  const base = r.breakdown_value != null ? `${r.label} · ${breakdownLabel(r.breakdown_value)}` : r.label;
  return r.compare_label === 'previous' ? `${base} (previous)` : base;
}

/** Map an API trends response onto chart labels and coloured series. */
export function toChart(res: TrendsResponse): { labels: string[]; series: ChartSeries[] } {
  const current = res.results.filter((r) => r.compare_label !== 'previous');
  const labels = (current[0]?.days ?? []).map((d) => formatBucket(d, res.interval));
  let colorIdx = 0;
  const series = res.results.map((r) => ({
    label: resultLabel(r),
    data: r.data,
    dashed: r.compare_label === 'previous',
    color: r.compare_label === 'previous' ? 'var(--faint)' : seriesColor(colorIdx++),
  }));
  return { labels, series };
}

/** Pair each current result with its previous-period counterpart, if any. */
export function currentAndPrevious(res: TrendsResponse, seriesIndex: number) {
  const of = (label: 'current' | 'previous') =>
    res.results.find((r) => r.series_index === seriesIndex && (r.compare_label ?? 'current') === label && r.breakdown_value == null);
  return { current: of('current'), previous: of('previous') };
}

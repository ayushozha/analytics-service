import { describe, expect, it } from 'vitest';
import { breakdownLabel, currentAndPrevious, toChart } from './insights';
import type { TrendsResponse } from './types';

const res: TrendsResponse = {
  interval: 'day',
  results: [
    { label: '$pageview', series_index: 0, event: '$pageview', breakdown_value: null, compare_label: 'current', days: ['2026-11-01T00:00:00', '2026-11-02T00:00:00'], data: [3, 5], count: 8 },
    { label: '$pageview', series_index: 0, event: '$pageview', breakdown_value: null, compare_label: 'previous', days: ['2026-10-30T00:00:00', '2026-10-31T00:00:00'], data: [1, 2], count: 3 },
  ],
};

describe('insights', () => {
  it('maps trends onto chart series, comparison dashed', () => {
    const { labels, series } = toChart(res);
    expect(labels).toEqual(['Nov 1', 'Nov 2']);
    expect(series.map((s) => [s.label, s.dashed])).toEqual([['$pageview', false], ['$pageview (previous)', true]]);
  });

  it('pairs current and previous results', () => {
    const { current, previous } = currentAndPrevious(res, 0);
    expect(current?.count).toBe(8);
    expect(previous?.count).toBe(3);
  });

  it('renames the overflow breakdown bucket', () => {
    expect(breakdownLabel('$$_other')).toBe('Other');
    expect(breakdownLabel('Chrome')).toBe('Chrome');
  });
});

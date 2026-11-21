import { describe, expect, it } from 'vitest';
import { render } from '@testing-library/react';
import { LineChart, niceTicks, showLabel } from './LineChart';
import { RetentionGrid, isFuture, meanCurve } from './RetentionGrid';
import type { RetentionRow } from '../lib/types';

describe('LineChart', () => {
  it('uses evenly spaced nice ticks', () => {
    expect(niceTicks(24)).toEqual([0, 10, 20, 30]);
    expect(niceTicks(1010)).toEqual([0, 500, 1000, 1500]);
    expect(niceTicks(0)).toEqual([0, 1]);
  });

  it('always shows the last x label and drops ones crowding it', () => {
    const shown = Array.from({ length: 31 }, (_, i) => i).filter((i) => showLabel(i, 31, 4));
    expect(shown).toEqual([0, 4, 8, 12, 16, 20, 24, 30]);
  });

  it('renders one path per series plus the area fill', () => {
    const { container, getByRole } = render(
      <LineChart ariaLabel="Visitors" labels={['a', 'b', 'c']}
        series={[{ label: 'Now', color: 'red', data: [1, 2, 3] }, { label: 'Before', color: 'grey', data: [1, 1, 1], dashed: true }]} />,
    );
    expect(getByRole('img', { name: 'Visitors' })).toBeTruthy();
    expect(container.querySelectorAll('path')).toHaveLength(3);
    expect(container.querySelector('path[stroke-dasharray]')).toBeTruthy();
  });
});

describe('retention', () => {
  const now = Date.parse('2026-11-20T00:00:00Z');
  const row = (date: string, counts: number[]): RetentionRow => ({
    date, label: '', breakdown_value: null,
    values: counts.map((count, interval) => ({ interval, count, percentage: (count / counts[0]) * 100 })),
  });

  it('treats intervals after now as future', () => {
    expect(isFuture('2026-11-16T00:00:00', 'week', 0, now)).toBe(false);
    expect(isFuture('2026-11-16T00:00:00', 'week', 1, now)).toBe(true);
  });

  it('weights the mean curve by cohort size and skips future cells', () => {
    const rows = [row('2026-11-02T00:00:00', [100, 50, 20]), row('2026-11-09T00:00:00', [300, 60, 0])];
    // Week 1: (50+60)/(100+300); week 2 only the first cohort has reached it.
    const curve = meanCurve(rows, 'week', now);
    expect(curve).toHaveLength(3);
    [100, 27.5, 20].forEach((v, i) => expect(curve[i]).toBeCloseTo(v));
  });

  it('renders percentages and leaves future cells blank', () => {
    const { getAllByText, container } = render(<RetentionGrid period="week" rows={[row('2026-11-16T00:00:00', [10, 4])]} />);
    expect(getAllByText('100%')).toHaveLength(1);
    expect(container.textContent).not.toContain('40%');
  });
});

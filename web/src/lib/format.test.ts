import { describe, expect, it } from 'vitest';
import { change, formatBucket, formatDuration, formatNumber, initials, personName, rangeBounds, timeAgo } from './format';

describe('format', () => {
  it('formats numbers, compacting large ones', () => {
    expect(formatNumber(1010)).toBe('1,010');
    expect(formatNumber(12.34)).toBe('12.3');
    expect(formatNumber(48_200)).toBe('48.2K');
  });

  it('computes change only when there is a baseline', () => {
    expect(change(150, 100)).toBe(50);
    expect(change(50, 100)).toBe(-50);
    expect(change(10, 0)).toBeNull();
  });

  it('formats durations', () => {
    expect(formatDuration(null)).toBe('–');
    expect(formatDuration(42)).toBe('42s');
    expect(formatDuration(260)).toBe('4m');
    expect(formatDuration(5400)).toBe('1.5h');
  });

  it('labels API buckets in UTC', () => {
    expect(formatBucket('2026-11-16T00:00:00', 'week')).toBe('Nov 16');
    expect(formatBucket('2026-11-01T00:00:00', 'month')).toBe('Nov 26');
  });

  it('describes relative time', () => {
    const now = Date.parse('2026-11-20T12:00:00Z');
    expect(timeAgo('2026-11-20T11:59:30Z', now)).toBe('30s ago');
    expect(timeAgo('2026-11-18T12:00:00Z', now)).toBe('2d ago');
    expect(timeAgo(null, now)).toBe('–');
  });

  it('builds preset date ranges ending now', () => {
    const now = new Date('2026-11-20T00:00:00Z');
    expect(rangeBounds('7d', now)).toEqual({ date_from: '2026-11-13T00:00:00.000Z', date_to: now.toISOString(), interval: 'day' });
    expect(rangeBounds('90d', now).interval).toBe('week');
  });

  it('names people from traits, then IDs', () => {
    expect(personName({ traits: { name: 'Maya Rivera' }, user_id: 'm@x.io', visitor_id: 'v' })).toBe('Maya Rivera');
    expect(personName({ traits: null, user_id: null, visitor_id: 'v1' })).toBe('v1');
    expect(initials('Maya Rivera')).toBe('MR');
    expect(initials('luna.rivera@northwind.io')).toBe('LR');
  });
});

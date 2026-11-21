import type { RetentionRow } from '../lib/types';
import { formatBucket, formatNumber } from '../lib/format';

/** Accent-tinted cohort grid; strong cells switch to white text for contrast. */
export function cellStyle(pct: number) {
  return {
    background: `rgba(207,47,118,${(0.06 + pct / 110).toFixed(2)})`,
    color: pct > 55 ? '#fff' : 'var(--ink)',
  };
}

const PERIOD_MS: Record<string, number> = { hour: 3_600_000, day: 86_400_000, week: 604_800_000, month: 2_592_000_000 };

/** True when interval `i` of a cohort starting at `date` hasn't happened yet. */
export function isFuture(date: string, period: string, i: number, now = Date.now()) {
  const start = new Date(date.endsWith('Z') ? date : `${date}Z`).getTime();
  return start + i * (PERIOD_MS[period] ?? PERIOD_MS.day) > now;
}

export function RetentionGrid({ rows, period, showCounts = false }: { rows: RetentionRow[]; period: string; showCounts?: boolean }) {
  const cols = Math.max(0, ...rows.map((r) => r.values.length));
  const unit = period[0].toUpperCase() + period.slice(1);
  return (
    <div className="scroll-x">
      <table className="ret-table">
        <thead>
          <tr>
            <th scope="col">Cohort</th>
            <th scope="col" style={{ textAlign: 'right' }}>Size</th>
            {Array.from({ length: cols }, (_, i) => <th scope="col" key={i} style={{ textAlign: 'center' }}>{unit} {i}</th>)}
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => {
            const size = r.values[0]?.count ?? 0;
            return (
              <tr key={r.date + (r.breakdown_value ?? '')}>
                <th scope="row" style={{ fontWeight: 500, color: 'var(--ink)' }}>{formatBucket(r.date, period)}</th>
                <td style={{ textAlign: 'right', color: 'var(--ink-2)' }}>{formatNumber(size)}</td>
                {Array.from({ length: cols }, (_, i) => {
                  const v = r.values[i];
                  if (!v || size === 0 || isFuture(r.date, period, i)) return <td key={i} />;
                  return <td key={i} style={cellStyle(v.percentage)}>{showCounts ? formatNumber(v.count) : `${Math.round(v.percentage)}%`}</td>;
                })}
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

/** Mean retention per interval, weighted by cohort size, over cohorts that have reached it. */
export function meanCurve(rows: RetentionRow[], period: string, now = Date.now()): number[] {
  const cols = Math.max(0, ...rows.map((r) => r.values.length));
  const out: number[] = [];
  for (let i = 0; i < cols; i++) {
    let kept = 0;
    let size = 0;
    for (const r of rows) {
      const s = r.values[0]?.count ?? 0;
      if (!s || !r.values[i] || isFuture(r.date, period, i, now)) continue;
      kept += r.values[i].count;
      size += s;
    }
    if (!size) break;
    out.push((kept / size) * 100);
  }
  return out;
}

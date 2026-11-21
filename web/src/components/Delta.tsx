import { formatPercent } from '../lib/format';

export function Delta({ value, suffix = 'vs previous' }: { value: number | null; suffix?: string }) {
  if (value == null) return <span className="delta flat">No previous data</span>;
  const dir = value > 0.05 ? 'up' : value < -0.05 ? 'down' : 'flat';
  const arrow = dir === 'up' ? '↑' : dir === 'down' ? '↓' : '→';
  return <span className={`delta ${dir}`}>{arrow} {formatPercent(Math.abs(value))} <span style={{ color: 'var(--muted)', fontWeight: 400 }}>{suffix}</span></span>;
}

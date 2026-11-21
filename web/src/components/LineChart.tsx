import { useLayoutEffect, useRef, useState } from 'react';
import { formatNumber } from '../lib/format';

export interface ChartSeries {
  label: string;
  color: string;
  data: number[];
  /** Previous-period comparison lines are drawn dashed and grey. */
  dashed?: boolean;
}

const PAD = { top: 12, right: 12, bottom: 28, left: 44 };

/** Evenly spaced axis ticks on a "nice" step (1, 2, 5 × 10^n), at most `count` intervals. */
export function niceTicks(max: number, count = 4): number[] {
  if (max <= 0) return [0, 1];
  const raw = max / count;
  const exp = 10 ** Math.floor(Math.log10(raw));
  const f = raw / exp;
  const step = (f <= 1 ? 1 : f <= 2 ? 2 : f <= 5 ? 5 : 10) * exp;
  const ticks = [];
  for (let v = 0; v < max + step; v += step) ticks.push(Number(v.toPrecision(12)));
  return ticks;
}

/** Every `every`-th label plus the last one, dropping any label crowding the last. */
export function showLabel(i: number, n: number, every: number): boolean {
  if (i === n - 1) return true;
  return i % every === 0 && n - 1 - i >= every * 0.75;
}

function useWidth<T extends HTMLElement>(fallback: number) {
  const ref = useRef<T>(null);
  const [width, setWidth] = useState(fallback);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el || typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(([entry]) => setWidth(Math.max(240, entry.contentRect.width)));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  return [ref, width] as const;
}

export function LineChart({ labels, series, height = 260, ariaLabel, area = true }: {
  labels: string[];
  series: ChartSeries[];
  height?: number;
  ariaLabel: string;
  area?: boolean;
}) {
  const [ref, width] = useWidth<HTMLDivElement>(800);
  const [hover, setHover] = useState<number | null>(null);
  const n = labels.length;
  const ticks = niceTicks(Math.max(0, ...series.flatMap((s) => s.data)));
  const max = ticks[ticks.length - 1];
  const w = width - PAD.left - PAD.right;
  const h = height - PAD.top - PAD.bottom;
  const x = (i: number) => PAD.left + (n <= 1 ? w / 2 : (i / (n - 1)) * w);
  const y = (v: number) => PAD.top + h - (v / max) * h;
  const labelEvery = Math.max(1, Math.ceil(n / Math.max(2, Math.floor(w / 70))));

  const onMove = (e: React.MouseEvent<SVGSVGElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const px = ((e.clientX - rect.left) / rect.width) * width;
    const i = n <= 1 ? 0 : Math.round(((px - PAD.left) / w) * (n - 1));
    setHover(Math.min(n - 1, Math.max(0, i)));
  };

  return (
    <div ref={ref} style={{ position: 'relative' }}>
      <svg className="chart" role="img" aria-label={ariaLabel} width={width} height={height}
        viewBox={`0 0 ${width} ${height}`} onMouseMove={onMove} onMouseLeave={() => setHover(null)}>
        <g stroke="var(--line-soft)">
          {ticks.map((t) => <line key={t} x1={PAD.left} x2={width - PAD.right} y1={y(t)} y2={y(t)} />)}
        </g>
        <g fill="var(--muted)" fontSize="11">
          {ticks.map((t) => <text key={t} x={PAD.left - 8} y={y(t) + 4} textAnchor="end">{formatNumber(t)}</text>)}
          {labels.map((l, i) => showLabel(i, n, labelEvery) && (
            <text key={i} x={x(i)} y={height - 8} textAnchor={i === 0 ? 'start' : i === n - 1 ? 'end' : 'middle'}>{l}</text>
          ))}
        </g>
        {series.map((s, si) => {
          const pts = s.data.map((v, i) => `${x(i)},${y(v)}`);
          if (!pts.length) return null;
          const line = `M${pts.join('L')}`;
          return (
            <g key={si}>
              {area && !s.dashed && si === 0 && (
                <path d={`${line}L${x(s.data.length - 1)},${y(0)}L${x(0)},${y(0)}Z`} fill={s.color} fillOpacity="0.08" />
              )}
              <path d={line} fill="none" stroke={s.dashed ? 'var(--faint)' : s.color} strokeWidth={s.dashed ? 1.75 : 2.25}
                strokeDasharray={s.dashed ? '5 4' : undefined} strokeLinejoin="round" />
            </g>
          );
        })}
        {hover != null && (
          <g>
            <line x1={x(hover)} x2={x(hover)} y1={PAD.top} y2={PAD.top + h} stroke="var(--line-dashed)" />
            {series.map((s, si) => s.data[hover] != null && (
              <circle key={si} cx={x(hover)} cy={y(s.data[hover])} r="3.5" fill="#fff"
                stroke={s.dashed ? 'var(--faint)' : s.color} strokeWidth="2" />
            ))}
          </g>
        )}
      </svg>
      {hover != null && (
        <div className="tooltip" role="tooltip"
          style={{
            left: `${(x(hover) / width) * 100}%`,
            top: y(Math.max(...series.map((s) => s.data[hover] ?? 0))),
            // Keep the tooltip inside the card near either edge.
            transform: `translate(${x(hover) > width * 0.65 ? 'calc(-100% - 12px)' : x(hover) < width * 0.35 ? '12px' : '-50%'}, -110%)`,
          }}>
          <div style={{ fontWeight: 600, marginBottom: 4 }}>{labels[hover]}</div>
          {series.map((s, si) => (
            <div className="tooltip-row" key={si}>
              <span className="swatch" style={{ background: s.dashed ? 'var(--faint)' : s.color }} />
              <span style={{ flex: 1 }}>{s.label}</span>
              <strong style={{ marginLeft: 12 }}>{formatNumber(s.data[hover] ?? 0)}</strong>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export function Legend({ series }: { series: ChartSeries[] }) {
  return (
    <div className="legend">
      {series.map((s, i) => (
        <span className="legend-item" key={i}>
          <span className={`swatch${s.dashed ? ' dashed' : ''}`} style={s.dashed ? undefined : { background: s.color }} />
          {s.label}
        </span>
      ))}
    </div>
  );
}

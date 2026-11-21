import { useState } from 'react';
import { NavLink, useParams } from 'react-router';
import { api } from '../lib/api';
import { formatNumber, formatPercent, rangeBounds, type DateRangeKey } from '../lib/format';
import { BREAKDOWNS, MATH_LABELS, resultLabel, seriesColor, toChart } from '../lib/insights';
import type { FunnelsResponse, Interval, Math, RetentionResponse, TrendSeries, TrendsResponse } from '../lib/types';
import { useApi, useDebounced } from '../lib/useApi';
import { Async } from '../components/States';
import { DateRangeSelect } from '../components/DateRangeSelect';
import { EventInput } from '../components/EventInput';
import { FunnelChart } from '../components/FunnelChart';
import { Legend, LineChart } from '../components/LineChart';
import { RetentionGrid, meanCurve } from '../components/RetentionGrid';

const KINDS = [
  { key: 'trends', label: 'Trends' },
  { key: 'funnels', label: 'Funnels' },
  { key: 'retention', label: 'Retention' },
] as const;
const COMING = ['Paths', 'Stickiness', 'Lifecycle', 'SQL'];
const LETTERS = 'ABCDE';
const MAX_SERIES = LETTERS.length;

export function Insights() {
  const { kind = 'trends' } = useParams();
  const [range, setRange] = useState<DateRangeKey>('30d');
  const title = { trends: 'How is usage changing?', funnels: 'Where do people drop off?', retention: 'Do people come back?' }[kind] ?? 'Insight';
  return (
    <>
      <div className="page-head">
        <div>
          <span className="crumb">Insights / New insight</span>
          <h1>{title}</h1>
        </div>
        <div className="actions"><DateRangeSelect value={range} onChange={setRange} /></div>
      </div>
      <div className="tabs" role="tablist" aria-label="Insight type">
        {KINDS.map((k) => (
          <NavLink key={k.key} role="tab" to={`/insights/${k.key}`} aria-selected={kind === k.key}
            className={() => `tab${kind === k.key ? ' active' : ''}`}>{k.label}</NavLink>
        ))}
        {COMING.map((c) => <span key={c} className="tab soon" title="Coming soon">{c}</span>)}
      </div>
      {kind === 'funnels' ? <Funnels range={range} /> : kind === 'retention' ? <Retention range={range} /> : <Trends range={range} />}
    </>
  );
}

function Trends({ range }: { range: DateRangeKey }) {
  const [series, setSeries] = useState<TrendSeries[]>([{ event: '$pageview', math: 'dau' }]);
  const [breakdown, setBreakdown] = useState('');
  const [compare, setCompare] = useState(false);
  const [interval, setInterval] = useState<Interval | ''>('');
  const b = rangeBounds(range);
  const bd = BREAKDOWNS.find((x) => x.property === breakdown);
  const debounced = useDebounced(series);
  const query = {
    series: debounced, date_from: b.date_from, date_to: b.date_to, interval: interval || b.interval, compare,
    breakdown: bd ? { property: bd.property, type: bd.type, limit: 10 } : undefined,
  };
  const res = useApi(`trends:${JSON.stringify({ ...query, date_from: range, date_to: '' })}`,
    () => api<TrendsResponse>('/api/v1/insights/trends', query));
  const update = (i: number, patch: Partial<TrendSeries>) => setSeries(series.map((s, j) => (j === i ? { ...s, ...patch } : s)));

  return (
    <div className="grid" style={{ alignItems: 'flex-start' }}>
      <section className="card" style={{ flex: '0 1 380px' }} aria-label="Query">
        <div className="query">
          <h2>Series</h2>
          {series.map((s, i) => (
            <div className="series-row" key={i}>
              <span className="series-letter" style={{ background: seriesColor(i) }}>{LETTERS[i]}</span>
              <EventInput label={`Series ${LETTERS[i]} event`} value={s.event} onChange={(event) => update(i, { event })} />
              <select className="select" aria-label={`Series ${LETTERS[i]} math`} value={s.math}
                onChange={(e) => update(i, { math: e.target.value as Math })}>
                {Object.entries(MATH_LABELS).map(([k, l]) => <option key={k} value={k}>{l}</option>)}
              </select>
              {series.length > 1 && (
                <button className="btn btn-sm" aria-label={`Remove series ${LETTERS[i]}`} onClick={() => setSeries(series.filter((_, j) => j !== i))}>✕</button>
              )}
            </div>
          ))}
          {series.length < MAX_SERIES && (
            <button className="btn btn-sm btn-dashed" style={{ alignSelf: 'flex-start' }}
              onClick={() => setSeries([...series, { event: null, math: 'total' }])}>+ Add series</button>
          )}
          <h2 style={{ marginTop: 8 }}>Breakdown</h2>
          <select className="select" aria-label="Breakdown" value={breakdown} onChange={(e) => setBreakdown(e.target.value)}>
            <option value="">None</option>
            {BREAKDOWNS.map((x) => <option key={x.property} value={x.property}>{x.label}</option>)}
          </select>
          <h2 style={{ marginTop: 8 }}>Options</h2>
          <label className="toolbar"><input type="checkbox" checked={compare} onChange={(e) => setCompare(e.target.checked)} style={{ accentColor: 'var(--accent)' }} /> Compare to previous period</label>
          <label className="toolbar">Group by
            <select className="select" aria-label="Interval" value={interval} onChange={(e) => setInterval(e.target.value as Interval | '')}>
              <option value="">Auto</option><option value="hour">Hour</option><option value="day">Day</option><option value="week">Week</option><option value="month">Month</option>
            </select>
          </label>
        </div>
      </section>
      <section className="card wide">
        <Async state={res} height={300} isEmpty={(r) => r.results.length === 0}>
          {(r) => {
            const { labels, series: chart } = toChart(r);
            return (
              <>
                <LineChart labels={labels} series={chart} height={300} ariaLabel="Trends chart" area={chart.length === 1} />
                <Legend series={chart} />
                <div className="scroll-x">
                  <table className="table">
                    <thead><tr><th>Series</th><th className="num">Total</th></tr></thead>
                    <tbody>
                      {r.results.map((x, i) => (
                        <tr key={i}><td>{resultLabel(x)}</td><td className="num">{formatNumber(x.count)}</td></tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </>
            );
          }}
        </Async>
      </section>
    </div>
  );
}

function Funnels({ range }: { range: DateRangeKey }) {
  const [steps, setSteps] = useState<string[]>(['$pageview', '']);
  const [order, setOrder] = useState('ordered');
  const [windowDays, setWindowDays] = useState(14);
  const b = rangeBounds(range);
  const debounced = useDebounced(steps);
  const ready = debounced.filter((s) => s.trim()).length >= 2;
  const body = {
    steps: debounced.filter((s) => s.trim()).map((event) => ({ event })),
    date_from: b.date_from, date_to: b.date_to, order, funnel_window: windowDays, funnel_window_unit: 'day',
  };
  const res = useApi(`funnel:${range}:${JSON.stringify([debounced, order, windowDays])}`,
    () => (ready ? api<FunnelsResponse>('/api/v1/insights/funnels', body) : Promise.resolve({ results: [] })));

  return (
    <div className="grid" style={{ alignItems: 'flex-start' }}>
      <section className="card" style={{ flex: '0 1 380px' }} aria-label="Query">
        <div className="query">
          <h2>Steps</h2>
          {steps.map((s, i) => (
            <div className="series-row" key={i}>
              <span className="series-letter" style={{ background: 'var(--ink-2)' }}>{i + 1}</span>
              <EventInput allowAll={false} label={`Step ${i + 1} event`} value={s} onChange={(v) => setSteps(steps.map((x, j) => (j === i ? v ?? '' : x)))} />
              {steps.length > 2 && <button className="btn btn-sm" aria-label={`Remove step ${i + 1}`} onClick={() => setSteps(steps.filter((_, j) => j !== i))}>✕</button>}
            </div>
          ))}
          <button className="btn btn-sm btn-dashed" style={{ alignSelf: 'flex-start' }} onClick={() => setSteps([...steps, ''])}>+ Add step</button>
          <h2 style={{ marginTop: 8 }}>Options</h2>
          <label className="toolbar">Step order
            <select className="select" aria-label="Step order" value={order} onChange={(e) => setOrder(e.target.value)}>
              <option value="ordered">Sequential</option><option value="strict">Strict order</option><option value="unordered">Any order</option>
            </select>
          </label>
          <label className="toolbar">Conversion window
            <input className="input" type="number" min={1} max={90} style={{ width: 70 }} aria-label="Conversion window in days"
              value={windowDays} onChange={(e) => setWindowDays(Math.max(1, Number(e.target.value) || 1))} /> days
          </label>
        </div>
      </section>
      <section className="card wide">
        {!ready ? <div className="empty">Add at least two steps to see the funnel.</div> : (
          <Async state={res} height={280} isEmpty={(r) => !r.results[0]?.steps.length}>
            {(r) => {
              const s = r.results[0].steps;
              const last = s[s.length - 1];
              return (
                <>
                  <div className="card-head">
                    <h2>{formatPercent(last.conversion_rate)} conversion</h2>
                    <span className="card-sub">{formatNumber(s[0].count)} entered · {formatNumber(last.count)} completed</span>
                  </div>
                  <FunnelChart steps={s} />
                </>
              );
            }}
          </Async>
        )}
      </section>
    </div>
  );
}

function Retention({ range }: { range: DateRangeKey }) {
  const [target, setTarget] = useState<string | null>('$pageview');
  const [returning, setReturning] = useState<string | null>(null);
  const [period, setPeriod] = useState<Interval>('week');
  const [counts, setCounts] = useState(false);
  const intervals = period === 'day' ? 14 : period === 'week' ? 8 : 6;
  const b = rangeBounds(range);
  // Debounce a string so the value is stable between renders.
  const [t, r] = JSON.parse(useDebounced(JSON.stringify([target, returning]))) as (string | null)[];
  const body = {
    target: { event: t }, returning: { event: r }, retention_type: 'first_time',
    period, total_intervals: intervals, date_from: b.date_from, date_to: b.date_to,
  };
  const res = useApi(`ret:${range}:${JSON.stringify([t, r, period])}`,
    () => api<RetentionResponse>('/api/v1/insights/retention', body));

  return (
    <>
      <section className="card" aria-label="Query">
        <div className="sentence">
          <span>Show people who did</span>
          <EventInput label="Cohort event" value={target} onChange={setTarget} />
          <span>for the first time, and came back to do</span>
          <EventInput label="Return event" value={returning} onChange={setReturning} />
          <span>each</span>
          <select className="select" aria-label="Period" value={period} onChange={(e) => setPeriod(e.target.value as Interval)}>
            <option value="day">day</option><option value="week">week</option><option value="month">month</option>
          </select>
        </div>
      </section>
      <section className="card">
        <div className="card-head">
          <h2>Retention curve</h2>
          <span className="card-sub">Mean of all cohorts, weighted by size (%)</span>
        </div>
        <Async state={res} height={220} isEmpty={(x) => meanCurve(x.results, x.period).length === 0}>
          {(x) => {
            const curve = meanCurve(x.results, x.period);
            const unit = x.period[0].toUpperCase() + x.period.slice(1);
            return <LineChart height={220} ariaLabel="Retention curve" labels={curve.map((_, i) => `${unit} ${i}`)}
              series={[{ label: 'Retained %', color: 'var(--accent)', data: curve }]} />;
          }}
        </Async>
      </section>
      <section className="card">
        <div className="card-head">
          <h2>Cohorts</h2>
          <label className="toolbar card-sub"><input type="checkbox" checked={counts} onChange={(e) => setCounts(e.target.checked)} style={{ accentColor: 'var(--accent)' }} /> Show counts</label>
        </div>
        <Async state={res} height={280} isEmpty={(r) => r.results.length === 0}>
          {(r) => <RetentionGrid rows={r.results} period={r.period} showCounts={counts} />}
        </Async>
      </section>
    </>
  );
}


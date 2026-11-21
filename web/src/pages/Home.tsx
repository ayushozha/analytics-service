import { useState } from 'react';
import { Link } from 'react-router';
import { api } from '../lib/api';
import { change, formatNumber, rangeBounds, type DateRangeKey } from '../lib/format';
import { breakdownLabel, currentAndPrevious, toChart } from '../lib/insights';
import type { Interval, RetentionResponse, TrendsQuery, TrendsResponse } from '../lib/types';
import { useApi } from '../lib/useApi';
import { Async } from '../components/States';
import { Delta } from '../components/Delta';
import { DateRangeSelect } from '../components/DateRangeSelect';
import { Legend, LineChart } from '../components/LineChart';
import { RetentionGrid } from '../components/RetentionGrid';

const trends = (q: TrendsQuery) => api<TrendsResponse>('/api/v1/insights/trends', q);

// Project-agnostic KPIs: they only rely on built-in pageviews and sessions.
const KPIS = [
  { label: 'Unique visitors', series: { event: '$pageview', math: 'dau' } },
  { label: 'Pageviews', series: { event: '$pageview', math: 'total' } },
  { label: 'Sessions', series: { event: null, math: 'unique_sessions' } },
  { label: 'Events', series: { event: null, math: 'total' } },
] as const;

export function Home({ projectName }: { projectName?: string }) {
  const [range, setRange] = useState<DateRangeKey>('30d');
  const b = rangeBounds(range);
  const span = { date_from: b.date_from, date_to: b.date_to };

  const kpis = useApi(`kpi:${range}`, () =>
    trends({ ...span, interval: b.interval, compare: true, series: KPIS.map((k) => ({ ...k.series })) }));
  const pages = useApi(`pages:${range}`, () =>
    trends({ ...span, interval: b.interval, series: [{ event: '$pageview', math: 'total' }], breakdown: { property: 'path', limit: 8 } }));
  const retention = useApi(`ret:${range}`, () =>
    api<RetentionResponse>('/api/v1/insights/retention', {
      target: { event: '$pageview' }, returning: { event: null }, retention_type: 'first_time',
      period: 'week', total_intervals: 6,
      date_from: new Date(Date.now() - 6 * 7 * 86_400_000).toISOString(), date_to: b.date_to,
    }));

  return (
    <>
      <div className="page-head">
        <div>
          <span className="crumb">Home</span>
          <h1>{projectName ? `${projectName} overview` : 'Overview'}</h1>
        </div>
        <div className="actions">
          <DateRangeSelect value={range} onChange={setRange} />
          <Link className="btn btn-primary" to="/insights">+ New insight</Link>
        </div>
      </div>

      <div className="kpis">
        {KPIS.map((k, i) => (
          <div className="kpi" key={k.label}>
            <span className="kpi-label">{k.label}</span>
            <Async state={kpis} height={52}>
              {(res) => {
                const { current, previous } = currentAndPrevious(res, i);
                const cur = current?.count ?? 0;
                return (
                  <>
                    <span className="kpi-value">{formatNumber(cur)}</span>
                    <Delta value={previous ? change(cur, previous.count) : null} />
                  </>
                );
              }}
            </Async>
          </div>
        ))}
      </div>

      <section className="card">
        <div className="card-head">
          <h2>Unique visitors</h2>
          <span className="card-sub">Per {b.interval as Interval}, dashed line is the previous period</span>
        </div>
        <Async state={kpis} height={260}>
          {(res) => {
            const only = { ...res, results: res.results.filter((r) => r.series_index === 0) };
            const { labels, series } = toChart(only);
            series.forEach((s) => (s.label = s.dashed ? 'Previous period' : 'Unique visitors'));
            return (<><LineChart labels={labels} series={series} ariaLabel="Unique visitors over time" /><Legend series={series} /></>);
          }}
        </Async>
      </section>

      <div className="grid">
        <section className="card">
          <div className="card-head"><h2>Top pages</h2><span className="card-sub">Pageviews</span></div>
          <Async state={pages} isEmpty={(r) => r.results.length === 0} empty="No pageviews in this range yet.">
            {(res) => {
              const rows = [...res.results].sort((a, c) => c.count - a.count);
              const top = rows[0]?.count || 1;
              return (
                <table className="table">
                  <thead><tr><th>Path</th><th className="num">Views</th></tr></thead>
                  <tbody>
                    {rows.map((r) => (
                      <tr key={r.breakdown_value ?? ''}>
                        <td className="bar-cell">
                          <span className="bar" style={{ width: `${(r.count / top) * 100}%` }} />
                          <span className="label mono">{breakdownLabel(r.breakdown_value)}</span>
                        </td>
                        <td className="num">{formatNumber(r.count)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              );
            }}
          </Async>
        </section>
        <section className="card wide">
          <div className="card-head">
            <h2>Do new visitors come back?</h2>
            <Link to="/insights/retention" className="card-sub">Open in Insights →</Link>
          </div>
          <Async state={retention} isEmpty={(r) => r.results.length === 0}>
            {(res) => <RetentionGrid rows={res.results} period={res.period} />}
          </Async>
        </section>
      </div>
    </>
  );
}

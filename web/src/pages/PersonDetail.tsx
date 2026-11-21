import { Link, useParams } from 'react-router';
import { api } from '../lib/api';
import { formatDuration, formatNumber, initials, personName, timeAgo } from '../lib/format';
import type { Page, PersonDetail as Detail, PersonEvent, PersonSession } from '../lib/types';
import { useApi } from '../lib/useApi';
import { Async } from '../components/States';

const time = (iso: string) => new Date(iso).toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: false });
const day = (iso: string) => new Date(iso).toLocaleDateString('en-US', { weekday: 'short', month: 'short', day: 'numeric' });

function describe(e: PersonEvent): string {
  return Object.entries(e.properties ?? {})
    .filter(([k]) => k !== 'title')
    .map(([k, v]) => (k === 'path' ? String(v) : `${k} = ${typeof v === 'string' ? `"${v}"` : JSON.stringify(v)}`))
    .join(' · ');
}

export function PersonDetail() {
  const { id = '' } = useParams();
  const base = `/api/v2/persons/${encodeURIComponent(id)}`;
  const person = useApi(`p:${id}`, () => api<Detail>(base));
  const events = useApi(`pe:${id}`, () => api<Page<PersonEvent>>(`${base}/events?limit=200`));
  const sessions = useApi(`ps:${id}`, () => api<Page<PersonSession>>(`${base}/sessions?limit=20`));

  return (
    <>
      <span className="crumb"><Link to="/persons">Persons</Link> / {person.data ? personName(person.data) : '…'}</span>
      <Async state={person} height={120}>
        {(p) => {
          const name = personName(p);
          const traits = Object.entries(p.traits ?? {});
          return (
            <>
              <div className="page-head" style={{ alignItems: 'center' }}>
                <div style={{ flexDirection: 'row', alignItems: 'center', gap: 16 }}>
                  <div className="avatar">{initials(name)}</div>
                  <div style={{ display: 'flex', flexDirection: 'column', gap: 4, minWidth: 0 }}>
                    <h1>{name}</h1>
                    <div className="meta-row">
                      {p.user_id && <><span>{p.user_id}</span><span>·</span></>}
                      <span>First seen {timeAgo(p.first_seen_at)}</span><span>·</span>
                      <span>Last seen {timeAgo(p.last_seen_at)}</span>
                      {p.aliases.length > 1 && <><span>·</span><span>{p.aliases.length} IDs merged</span></>}
                    </div>
                  </div>
                </div>
              </div>
              <div className="kpis">
                <div className="kpi"><span className="kpi-label">Sessions</span><span className="kpi-value">{formatNumber(p.stats.sessions)}</span></div>
                <div className="kpi"><span className="kpi-label">Pageviews</span><span className="kpi-value">{formatNumber(p.stats.pageviews)}</span></div>
                <div className="kpi"><span className="kpi-label">Events</span><span className="kpi-value">{formatNumber(p.stats.events)}</span></div>
              </div>
              <div className="grid" style={{ alignItems: 'flex-start' }}>
                <aside style={{ flex: '1 1 320px', display: 'flex', flexDirection: 'column', gap: 16, minWidth: 0 }}>
                  <section className="card" style={{ gap: 6 }}>
                    <h2>Properties</h2>
                    {traits.length === 0 && <span className="card-sub">No properties set yet.</span>}
                    {traits.map(([k, v]) => (
                      <div className="prop-row" key={k}>
                        <span className="prop-key">{k}</span>
                        <span className="prop-val" title={String(v)}>{typeof v === 'object' ? JSON.stringify(v) : String(v)}</span>
                      </div>
                    ))}
                  </section>
                  <section className="card" style={{ gap: 8 }}>
                    <h2>Cohorts</h2>
                    <div className="toolbar">
                      {(p.cohorts ?? []).length === 0 ? <span className="card-sub">Not in any cohort.</span>
                        : p.cohorts!.map((c) => <span className="chip" key={c.id}>{c.name}</span>)}
                    </div>
                    <h2 style={{ marginTop: 6 }}>IDs</h2>
                    <div className="toolbar">{p.aliases.map((a) => <code className="chip" key={a}>{a}</code>)}</div>
                  </section>
                </aside>
                <section className="card wide">
                  <h2>Activity</h2>
                  <Async state={sessions} height={240} isEmpty={(s) => s.data.length === 0} empty="No sessions recorded.">
                    {(s) => s.data.map((sess) => {
                      const evs = (events.data?.data ?? []).filter((e) => e.session_id === sess.session_id);
                      return (
                        <div className="session" key={sess.session_id}>
                          <div className="session-head">
                            <strong>{day(sess.first_at)}, {time(sess.first_at)}</strong>
                            <span style={{ color: 'var(--muted)' }}>
                              {formatDuration(sess.duration_ms / 1000)} · {sess.pageviews} pageviews · {sess.events} events
                              {sess.entry_page && <> · {sess.entry_page}{sess.exit_page && sess.exit_page !== sess.entry_page ? ` → ${sess.exit_page}` : ''}</>}
                            </span>
                            <span className="soon-badge">{[sess.browser, sess.os].filter(Boolean).join(' · ') || 'Unknown device'}</span>
                          </div>
                          {evs.map((e, i) => (
                            <div className="session-event" key={i}>
                              <span className="t">{time(e.created_at)}</span>
                              <span className="n">{e.event}</span>
                              <span className="d">{describe(e)}</span>
                            </div>
                          ))}
                        </div>
                      );
                    })}
                  </Async>
                </section>
              </div>
            </>
          );
        }}
      </Async>
    </>
  );
}

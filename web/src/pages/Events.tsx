import { Fragment, useEffect, useState } from 'react';
import { Link, NavLink, useParams } from 'react-router';
import { api } from '../lib/api';
import { eventDefinitionsUrl, propertyDefinitionsUrl, usePropertyDefinitions } from '../lib/definitions';
import { formatNumber, personName, timeAgo } from '../lib/format';
import type { DefinitionKind, EventDefinition, EventsPage, ExplorerEvent, PropertyDefinition, PropertyFilter } from '../lib/types';
import { useApi, useDebounced } from '../lib/useApi';
import { Async } from '../components/States';
import { EventInput } from '../components/EventInput';
import { PropertyFilters } from '../components/PropertyFilters';
import { DefinitionBadges, DefinitionEditor } from '../components/DefinitionEditor';

const TABS = [
  { key: 'activity', label: 'Activity' },
  { key: 'definitions', label: 'Event definitions' },
  { key: 'properties', label: 'Property definitions' },
] as const;
/** How often the activity feed refreshes while "Live" is on. */
const LIVE_REFRESH_MS = 5_000;

export function Events() {
  const { tab = 'activity' } = useParams();
  return (
    <>
      <div className="page-head">
        <div>
          <span className="crumb">Data</span>
          <h1>Events</h1>
        </div>
      </div>
      <div className="tabs" role="tablist" aria-label="Events views">
        {TABS.map((t) => (
          <NavLink key={t.key} role="tab" to={`/events/${t.key}`} aria-selected={tab === t.key}
            className={() => `tab${tab === t.key ? ' active' : ''}`}>{t.label}</NavLink>
        ))}
        <span className="tab soon" title="Coming soon">Actions</span>
      </div>
      {tab === 'definitions' ? <EventDefinitions /> : tab === 'properties' ? <PropertyDefinitions /> : <Activity />}
    </>
  );
}

// ── Activity ──

function pathOf(e: ExplorerEvent): string {
  const p = e.properties;
  return String(p.path ?? p.url ?? '');
}

function Activity() {
  const [event, setEvent] = useState<string | null>(null);
  const [filters, setFilters] = useState<PropertyFilter[]>([]);
  const [live, setLive] = useState(false);
  const [tick, setTick] = useState(0);
  const [older, setOlder] = useState<ExplorerEvent[]>([]);
  const [cursor, setCursor] = useState<string | null>(null);
  const [open, setOpen] = useState<number | null>(null);
  const ev = useDebounced(event);
  const query = { event: ev ?? undefined, properties: filters, limit: 50 };
  const key = JSON.stringify(query);

  useEffect(() => {
    if (!live) return;
    const t = setInterval(() => setTick((n) => n + 1), LIVE_REFRESH_MS);
    return () => clearInterval(t);
  }, [live]);
  // A new query (or a live refresh) starts from the newest page again.
  useEffect(() => { setOlder([]); setCursor(null); }, [key, tick]);
  useEffect(() => setOpen(null), [key]);

  const res = useApi(`events:${key}:${tick}`, () => api<EventsPage>('/api/v2/events/query', query));
  const next = cursor === null ? res.data?.next_before ?? null : cursor;
  const loadMore = async () => {
    if (!next) return;
    const page = await api<EventsPage>('/api/v2/events/query', { ...query, before: next });
    setOlder((o) => [...o, ...page.results]);
    setCursor(page.next_before ?? '');
  };

  return (
    <section className="card">
      <div className="card-head" style={{ alignItems: 'flex-start' }}>
        <div className="toolbar" style={{ flex: 1 }}>
          <div style={{ width: 240, display: 'flex' }}><EventInput label="Event" value={event} onChange={setEvent} /></div>
          <PropertyFilters value={filters} onChange={setFilters} event={ev} />
        </div>
        <label className="toolbar card-sub">
          <input type="checkbox" checked={live} onChange={(e) => setLive(e.target.checked)} style={{ accentColor: 'var(--accent)' }} />
          {live && <span className="live-dot" aria-hidden="true" />} Live
        </label>
      </div>
      <Async state={res} height={360} isEmpty={(r) => r.results.length === 0} empty="No events match in the last 30 days.">
        {(r) => {
          const rows = [...r.results, ...older];
          return (
            <>
              <div className="scroll-x">
                <table className="table">
                  <thead><tr><th>Event</th><th>Person</th><th>URL / path</th><th className="num">Time</th></tr></thead>
                  <tbody>
                    {rows.map((e, i) => (
                      <Fragment key={`${e.created_at}-${i}`}>
                        <tr className={`clickable${open === i ? ' selected' : ''}`} onClick={() => setOpen(open === i ? null : i)}
                          aria-expanded={open === i}>
                          <td className="mono" style={{ fontWeight: 600 }}>{e.event}</td>
                          <td>
                            <Link to={`/persons/${encodeURIComponent(e.visitor_id)}`} onClick={(x) => x.stopPropagation()}>
                              {e.person?.traits || e.person?.user_id
                                ? personName({ traits: e.person.traits, user_id: e.person.user_id, visitor_id: e.visitor_id })
                                : <span className="mono">{e.visitor_id.slice(0, 8)}…</span>}
                            </Link>
                          </td>
                          <td className="mono" style={{ color: 'var(--ink-2)' }}>{pathOf(e)}</td>
                          <td className="num" title={new Date(e.created_at).toLocaleString()}>{timeAgo(e.created_at)}</td>
                        </tr>
                        {open === i && (
                          <tr><td colSpan={4}>
                            <div className="props-grid">
                              {Object.entries(e.properties).map(([k, v]) => (
                                <Fragment key={k}><span className="k">{k}</span><span className="v">{typeof v === 'string' ? v : JSON.stringify(v)}</span></Fragment>
                              ))}
                              <span className="k">session_id</span><span className="v">{e.session_id ?? '–'}</span>
                              <span className="k">timestamp</span><span className="v">{e.created_at}</span>
                            </div>
                          </td></tr>
                        )}
                      </Fragment>
                    ))}
                  </tbody>
                </table>
              </div>
              {next && <button className="btn btn-sm" style={{ alignSelf: 'center' }} onClick={loadMore}>Load older events</button>}
            </>
          );
        }}
      </Async>
    </section>
  );
}

// ── Definitions ──

function useDefinitionList<T>(url: string, refresh: number) {
  return useApi(`${url}#${refresh}`, () => api<{ data: T[] }>(url).then((r) => r.data));
}

function SearchBar({ search, setSearch, hidden, setHidden, placeholder }: {
  search: string; setSearch: (v: string) => void; hidden: boolean; setHidden: (v: boolean) => void; placeholder: string;
}) {
  return (
    <div className="card-head">
      <input className="input" type="search" aria-label="Search definitions" placeholder={placeholder}
        style={{ flex: 1, maxWidth: 420 }} value={search} onChange={(e) => setSearch(e.target.value)} />
      <label className="toolbar card-sub">
        <input type="checkbox" checked={hidden} onChange={(e) => setHidden(e.target.checked)} style={{ accentColor: 'var(--accent)' }} /> Show hidden
      </label>
    </div>
  );
}

function Tags({ tags }: { tags: string[] }) {
  return <div className="toolbar" style={{ gap: 4 }}>{tags.map((t) => <span className="chip" key={t}>{t}</span>)}</div>;
}

function EventDefinitions() {
  const [search, setSearch] = useState('');
  const [hidden, setHidden] = useState(false);
  const [refresh, setRefresh] = useState(0);
  const [selected, setSelected] = useState<string | null>(null);
  const debounced = useDebounced(search);
  const q = new URLSearchParams({ ...(debounced ? { search: debounced } : {}), ...(hidden ? { include_hidden: 'true' } : {}) });
  const res = useDefinitionList<EventDefinition>(eventDefinitionsUrl(q.toString()), refresh);
  const current = res.data?.find((d) => d.name === selected);

  return (
    <div className="split">
      <section className="card">
        <SearchBar search={search} setSearch={setSearch} hidden={hidden} setHidden={setHidden} placeholder="Search events, descriptions or tags" />
        <Async state={res} height={360} isEmpty={(r) => r.length === 0} empty="No events in the last 30 days.">
          {(rows) => (
            <div className="scroll-x">
              <table className="table">
                <thead><tr><th>Event</th><th>Description</th><th>Tags</th><th className="num">Volume (30d)</th><th className="num">Users</th><th className="num">Last seen</th></tr></thead>
                <tbody>
                  {rows.map((d) => (
                    <tr key={d.name} className={`clickable${selected === d.name ? ' selected' : ''}`} onClick={() => setSelected(d.name)}>
                      <td><div className="toolbar" style={{ gap: 6 }}><span className="mono" style={{ fontWeight: 600 }}>{d.name}</span><DefinitionBadges meta={d} /></div></td>
                      <td style={{ color: d.description ? 'var(--ink-2)' : 'var(--faint)', maxWidth: 320 }}>{d.description ?? 'Add a description'}</td>
                      <td><Tags tags={d.tags} /></td>
                      <td className="num">{formatNumber(d.volume)}</td>
                      <td className="num">{formatNumber(d.unique_users)}</td>
                      <td className="num">{d.last_seen_at ? timeAgo(d.last_seen_at) : 'Never'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </Async>
      </section>
      {current && (
        <div className="editor" style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
          <DefinitionEditor key={current.name} kind="event" name={current.name} meta={current}
            onSaved={() => setRefresh((n) => n + 1)} onClose={() => setSelected(null)} />
          <EventProperties event={current.name} />
        </div>
      )}
    </div>
  );
}

function EventProperties({ event }: { event: string }) {
  const props = usePropertyDefinitions('event_property', event);
  return (
    <section className="card" style={{ gap: 6 }}>
      <div className="card-head">
        <h2>Properties</h2>
        <Link className="card-sub" to={`/insights/trends?event=${encodeURIComponent(event)}`}>Analyze in Trends →</Link>
      </div>
      {!props ? <div className="skeleton" style={{ height: 80 }} /> : props.length === 0 ? <span className="card-sub">No properties seen.</span>
        : props.map((p) => (
          <div className="prop-row" key={p.name}>
            <span className="prop-key">{p.name}</span>
            <span className="prop-val"><span className="type-badge">{p.property_type}</span></span>
            <span className="card-sub">{formatNumber(p.seen ?? 0)}</span>
          </div>
        ))}
    </section>
  );
}

const KINDS: { key: DefinitionKind; label: string }[] = [
  { key: 'event_property', label: 'Event' },
  { key: 'person_property', label: 'Person' },
  { key: 'session_property', label: 'Session' },
];

function PropertyDefinitions() {
  const [kind, setKind] = useState<DefinitionKind>('event_property');
  const [search, setSearch] = useState('');
  const [hidden, setHidden] = useState(false);
  const [refresh, setRefresh] = useState(0);
  const [selected, setSelected] = useState<string | null>(null);
  const debounced = useDebounced(search);
  const url = `${propertyDefinitionsUrl(kind)}${debounced ? `&search=${encodeURIComponent(debounced)}` : ''}${hidden ? '&include_hidden=true' : ''}`;
  const res = useDefinitionList<PropertyDefinition>(url, refresh);
  const current = res.data?.find((d) => d.name === selected);

  return (
    <div className="split">
      <section className="card">
        <div className="toolbar">
          <div className="segmented" role="group" aria-label="Property kind">
            {KINDS.map((k) => (
              <button key={k.key} aria-pressed={kind === k.key} onClick={() => { setKind(k.key); setSelected(null); }}>{k.label}</button>
            ))}
          </div>
        </div>
        <SearchBar search={search} setSearch={setSearch} hidden={hidden} setHidden={setHidden} placeholder="Search properties" />
        <Async state={res} height={360} isEmpty={(r) => r.length === 0} empty="No properties found.">
          {(rows) => (
            <div className="scroll-x">
              <table className="table">
                <thead><tr><th>Property</th><th>Type</th><th>Description</th><th>Tags</th><th className="num">Seen</th></tr></thead>
                <tbody>
                  {rows.map((d) => (
                    <tr key={d.name} className={`clickable${selected === d.name ? ' selected' : ''}`} onClick={() => setSelected(d.name)}>
                      <td><div className="toolbar" style={{ gap: 6 }}><span className="mono" style={{ fontWeight: 600 }}>{d.name}</span><DefinitionBadges meta={d} /></div></td>
                      <td><span className="type-badge">{d.property_type}</span></td>
                      <td style={{ color: d.description ? 'var(--ink-2)' : 'var(--faint)', maxWidth: 320 }}>{d.description ?? 'Add a description'}</td>
                      <td><Tags tags={d.tags} /></td>
                      <td className="num">{d.seen == null ? '–' : formatNumber(d.seen)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </Async>
      </section>
      {current && (
        <DefinitionEditor key={`${kind}:${current.name}`} kind={kind} name={current.name} meta={current}
          onSaved={() => setRefresh((n) => n + 1)} onClose={() => setSelected(null)} />
      )}
    </div>
  );
}

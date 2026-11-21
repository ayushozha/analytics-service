import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router';
import { api } from '../lib/api';
import { formatNumber, initials, personName, timeAgo } from '../lib/format';
import type { Page, Person } from '../lib/types';
import { useApi } from '../lib/useApi';
import { Async } from '../components/States';

const PAGE_SIZE = 50;

export function Persons() {
  const navigate = useNavigate();
  const [input, setInput] = useState('');
  const [search, setSearch] = useState('');
  const [offset, setOffset] = useState(0);

  // Debounce typing so each keystroke doesn't hit the API.
  useEffect(() => {
    const t = setTimeout(() => { setSearch(input.trim()); setOffset(0); }, 250);
    return () => clearTimeout(t);
  }, [input]);

  const res = useApi(`persons:${search}:${offset}`, () =>
    api<Page<Person>>('/api/v2/persons/query', { search: search || undefined, limit: PAGE_SIZE, offset }));

  return (
    <>
      <div className="page-head">
        <div>
          <span className="crumb">Data</span>
          <h1>Persons</h1>
        </div>
      </div>
      <section className="card">
        <div className="card-head">
          <input className="input" type="search" aria-label="Search persons" placeholder="Search by email, name or ID"
            style={{ flex: 1, maxWidth: 420 }} value={input} onChange={(e) => setInput(e.target.value)} />
          {res.data?.total != null && <span className="card-sub">{formatNumber(res.data.total)} people</span>}
        </div>
        <Async state={res} height={320} isEmpty={(r) => r.data.length === 0} empty="No people match.">
          {(r) => (
            <>
              <div className="scroll-x">
                <table className="table">
                  <thead><tr><th>Person</th><th>ID</th><th>First seen</th><th>Last seen</th></tr></thead>
                  <tbody>
                    {r.data.map((p) => {
                      const name = personName(p);
                      const open = () => navigate(`/persons/${encodeURIComponent(p.visitor_id)}`);
                      return (
                        <tr key={p.visitor_id} className="clickable" onClick={open}>
                          <td>
                            <a href={`/app/persons/${encodeURIComponent(p.visitor_id)}`} onClick={(e) => { e.preventDefault(); open(); }}
                              style={{ display: 'flex', alignItems: 'center', gap: 10, color: 'var(--ink)', textDecoration: 'none', fontWeight: 500 }}>
                              <span className="avatar sm">{initials(name)}</span>{name}
                            </a>
                          </td>
                          <td className="mono" style={{ color: 'var(--ink-2)' }}>{p.user_id ?? p.visitor_id.slice(0, 8)}</td>
                          <td>{timeAgo(p.first_seen_at)}</td>
                          <td>{timeAgo(p.last_seen_at)}</td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
              <div className="toolbar" style={{ justifyContent: 'flex-end' }}>
                <button className="btn btn-sm" disabled={offset === 0} onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}>← Previous</button>
                <button className="btn btn-sm" disabled={r.data.length < PAGE_SIZE} onClick={() => setOffset(offset + PAGE_SIZE)}>Next →</button>
              </div>
            </>
          )}
        </Async>
      </section>
    </>
  );
}

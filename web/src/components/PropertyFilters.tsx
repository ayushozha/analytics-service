import { useId, useState } from 'react';
import { usePropertyDefinitions } from '../lib/definitions';
import type { PropertyFilter } from '../lib/types';

const OPERATORS = [
  { key: 'exact', label: '=', needsValue: true },
  { key: 'is_not', label: '≠', needsValue: true },
  { key: 'icontains', label: 'contains', needsValue: true },
  { key: 'not_icontains', label: 'does not contain', needsValue: true },
  { key: 'is_set', label: 'is set', needsValue: false },
  { key: 'is_not_set', label: 'is not set', needsValue: false },
] as const;
const TYPES = [
  { key: 'event', label: 'Event', kind: 'event_property' },
  { key: 'person', label: 'Person', kind: 'person_property' },
  { key: 'session', label: 'Session', kind: 'session_property' },
] as const;

export function describeFilter(f: PropertyFilter): string {
  const op = OPERATORS.find((o) => o.key === (f.operator ?? 'exact'));
  const scope = f.type && f.type !== 'event' ? `${f.type}.` : '';
  return `${scope}${f.key} ${op?.label ?? f.operator}${op?.needsValue ? ` ${String(f.value)}` : ''}`;
}

/** Chips for active filters plus an inline form to add one, keys suggested from definitions. */
export function PropertyFilters({ value, onChange, event }: {
  value: PropertyFilter[];
  onChange: (v: PropertyFilter[]) => void;
  event?: string | null;
}) {
  const id = useId();
  const [open, setOpen] = useState(false);
  const [type, setType] = useState<(typeof TYPES)[number]['key']>('event');
  const [key, setKey] = useState('');
  const [operator, setOperator] = useState<(typeof OPERATORS)[number]['key']>('exact');
  const [val, setVal] = useState('');
  const kind = TYPES.find((t) => t.key === type)!.kind;
  const defs = usePropertyDefinitions(kind, type === 'event' ? event : null);
  const needsValue = OPERATORS.find((o) => o.key === operator)!.needsValue;

  const add = () => {
    if (!key.trim() || (needsValue && !val.trim())) return;
    onChange([...value, { key: key.trim(), type, operator, value: needsValue ? val.trim() : undefined }]);
    setKey('');
    setVal('');
    setOpen(false);
  };

  return (
    <div className="toolbar">
      {value.map((f, i) => (
        <span className="filter-chip" key={i}>
          <span className="mono">{describeFilter(f)}</span>
          <button aria-label={`Remove filter ${describeFilter(f)}`} onClick={() => onChange(value.filter((_, j) => j !== i))}>✕</button>
        </span>
      ))}
      {open ? (
        <form className="toolbar" onSubmit={(e) => { e.preventDefault(); add(); }}>
          <select className="select" aria-label="Property type" value={type} onChange={(e) => setType(e.target.value as typeof type)}>
            {TYPES.map((t) => <option key={t.key} value={t.key}>{t.label}</option>)}
          </select>
          <input className="input input-mono" aria-label="Property" list={id} placeholder="property" value={key}
            onChange={(e) => setKey(e.target.value)} autoFocus style={{ width: 160 }} />
          <datalist id={id}>{(defs ?? []).map((d) => <option key={d.name} value={d.name}>{d.property_type}</option>)}</datalist>
          <select className="select" aria-label="Operator" value={operator} onChange={(e) => setOperator(e.target.value as typeof operator)}>
            {OPERATORS.map((o) => <option key={o.key} value={o.key}>{o.label}</option>)}
          </select>
          {needsValue && <input className="input" aria-label="Value" placeholder="value" value={val} onChange={(e) => setVal(e.target.value)} style={{ width: 140 }} />}
          <button className="btn btn-sm btn-primary" type="submit">Add</button>
          <button className="btn btn-sm" type="button" onClick={() => setOpen(false)}>Cancel</button>
        </form>
      ) : (
        <button className="btn btn-sm btn-dashed" onClick={() => setOpen(true)}>+ Filter</button>
      )}
    </div>
  );
}

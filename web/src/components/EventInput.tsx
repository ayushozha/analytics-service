import { useId } from 'react';
import { useEventDefinitions } from '../lib/definitions';

// Event picker: suggests events seen in the last 30 days (busiest first) plus
// documented ones; hidden events are left out. Empty means "all events".
export function EventInput({ value, onChange, label, allowAll = true }: {
  value: string | null;
  onChange: (v: string | null) => void;
  label: string;
  allowAll?: boolean;
}) {
  const id = useId();
  const defs = useEventDefinitions();
  return (
    <>
      <input className="input input-mono" aria-label={label} list={id} style={{ flex: '1 1 120px', minWidth: 0 }}
        placeholder={allowAll ? 'All events' : 'event name'} value={value ?? ''}
        onChange={(e) => onChange(e.target.value.trim() === '' && allowAll ? null : e.target.value)} />
      <datalist id={id}>
        {(defs ?? [{ name: '$pageview', description: null, volume: 0 }]).map((d) => (
          <option key={d.name} value={d.name}>{d.description ?? (d.volume ? `${d.volume.toLocaleString()} in 30 days` : '')}</option>
        ))}
      </datalist>
    </>
  );
}

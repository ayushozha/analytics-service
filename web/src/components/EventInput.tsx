// Event picker. Until event definitions land, this is free text with the
// built-in events suggested; an empty value means "all events".
export const BUILTIN_EVENTS = ['$pageview'];

export function EventInput({ value, onChange, label, allowAll = true }: {
  value: string | null;
  onChange: (v: string | null) => void;
  label: string;
  allowAll?: boolean;
}) {
  return (
    <>
      <input className="input input-mono" aria-label={label} list="pulse-events" style={{ flex: '1 1 120px', minWidth: 0 }}
        placeholder={allowAll ? 'All events' : 'event name'} value={value ?? ''}
        onChange={(e) => onChange(e.target.value.trim() === '' && allowAll ? null : e.target.value)} />
      <datalist id="pulse-events">{BUILTIN_EVENTS.map((e) => <option key={e} value={e} />)}</datalist>
    </>
  );
}

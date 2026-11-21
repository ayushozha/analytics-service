import { DATE_RANGES, type DateRangeKey } from '../lib/format';

export function DateRangeSelect({ value, onChange }: { value: DateRangeKey; onChange: (v: DateRangeKey) => void }) {
  return (
    <select className="select" aria-label="Date range" value={value} onChange={(e) => onChange(e.target.value as DateRangeKey)}>
      {DATE_RANGES.map((r) => <option key={r.key} value={r.key}>{r.label}</option>)}
    </select>
  );
}

import type { FunnelStep } from '../lib/types';
import { formatDuration, formatNumber, formatPercent } from '../lib/format';

/** Vertical step bars; the hatched track behind each bar is the drop-off. */
export function FunnelChart({ steps }: { steps: FunnelStep[] }) {
  const first = steps[0]?.count || 1;
  return (
    <div className="funnel" role="list" aria-label="Funnel steps">
      {steps.map((s, i) => (
        <div className="funnel-step" role="listitem" key={i}>
          <div className="funnel-track" title={`${formatNumber(s.dropoff_count)} dropped off`}>
            <div className="funnel-fill" style={{ height: `${Math.max(2, (s.count / first) * 100)}%` }} />
          </div>
          <span className="funnel-name">{i + 1}. {s.name}</span>
          <strong>{formatPercent(s.conversion_rate)}</strong>
          <span className="funnel-meta">
            {formatNumber(s.count)} people
            {i > 0 && <> · {formatPercent(s.conversion_rate_from_previous, 0)} from prev · {formatDuration(s.median_time_to_convert_secs)}</>}
          </span>
        </div>
      ))}
    </div>
  );
}

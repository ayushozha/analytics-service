import type { ReactNode } from 'react';
import type { ApiState } from '../lib/useApi';

/** Render loading / error / empty states around an API result. */
export function Async<T>({ state, height = 160, empty, isEmpty, children }: {
  state: ApiState<T>;
  height?: number;
  empty?: string;
  isEmpty?: (data: T) => boolean;
  children: (data: T) => ReactNode;
}) {
  if (state.error) return <div className="error" role="alert">{state.error.message}</div>;
  if (state.data === undefined) return <div className="skeleton" style={{ height }} aria-busy="true" />;
  if (isEmpty?.(state.data)) return <div className="empty">{empty ?? 'No data in this range yet.'}</div>;
  return <>{children(state.data)}</>;
}

import { useEffect, useState } from 'react';

export interface ApiState<T> { data: T | undefined; error: Error | undefined; loading: boolean }

/** Run `load` whenever `key` changes; stale responses are ignored. */
export function useApi<T>(key: string, load: () => Promise<T>): ApiState<T> {
  const [state, setState] = useState<ApiState<T>>({ data: undefined, error: undefined, loading: true });
  useEffect(() => {
    let live = true;
    setState((s) => ({ ...s, loading: true, error: undefined }));
    load().then(
      (data) => live && setState({ data, error: undefined, loading: false }),
      (error: Error) => live && setState({ data: undefined, error, loading: false }),
    );
    return () => {
      live = false;
    };
    // `key` captures everything `load` depends on.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  return state;
}

/** `value`, updated only after it has stopped changing for `ms`. */
export function useDebounced<T>(value: T, ms = 300): T {
  const [v, setV] = useState(value);
  useEffect(() => {
    const t = setTimeout(() => setV(value), ms);
    return () => clearTimeout(t);
  }, [value, ms]);
  return v;
}

import { useCallback, useEffect, useRef, useState } from "react";
import { createAsyncResource, type ResourceState } from "../utils/asyncResource";

export function useAsyncResource<T>(key: string, read: () => Promise<T>, interval?: number, enabled = true) {
  const [state, setState] = useState<ResourceState<T> & { key: string }>({ key, loading: true });
  const resource = useRef<ReturnType<typeof createAsyncResource<T>> | undefined>(undefined);

  useEffect(() => {
    if (!enabled) return;
    const next = createAsyncResource(read, (value) => setState({ ...value, key }));
    resource.current = next;
    void next.refresh();
    const timer = interval ? window.setInterval(() => void next.refresh(), interval) : undefined;
    return () => {
      next.dispose();
      window.clearInterval(timer);
      resource.current = undefined;
    };
  }, [key, read, interval, enabled]);

  const refresh = useCallback(() => resource.current?.refresh(true) ?? Promise.resolve(), []);
  const visible: ResourceState<T> = state.key === key ? state : { loading: true };
  return { ...visible, refresh };
}

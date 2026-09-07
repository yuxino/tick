export interface ResourceState<T> {
  value?: T;
  error?: unknown;
  loading: boolean;
}

// Polls share the current request; an explicit refresh can supersede an old read.
export function createAsyncResource<T>(
  read: () => Promise<T>,
  publish: (state: ResourceState<T>) => void,
) {
  let disposed = false;
  let sequence = 0;
  let pending: Promise<void> | undefined;
  let value: T | undefined;

  function refresh(replace = false): Promise<void> {
    if (disposed) return Promise.resolve();
    if (pending && !replace) return pending;
    const request = ++sequence;
    publish({ value, loading: true });
    pending = Promise.resolve().then(read).then(
      (next) => {
        if (disposed || request !== sequence) return;
        value = next;
        publish({ value, loading: false });
      },
      (error: unknown) => {
        if (disposed || request !== sequence) return;
        value = undefined;
        publish({ error, loading: false });
      },
    ).finally(() => {
      if (request === sequence) pending = undefined;
    });
    return pending;
  }

  return {
    refresh,
    dispose() {
      disposed = true;
      sequence += 1;
    },
  };
}

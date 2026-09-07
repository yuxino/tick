import { describe, expect, it, vi } from "vitest";
import { createAsyncResource } from "./asyncResource";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

describe("task detail requests", () => {
  it("coalesces slow automatic polls without adding requests", async () => {
    const request = deferred<string>();
    const read = vi.fn(() => request.promise);
    const publish = vi.fn();
    const resource = createAsyncResource(read, publish);
    const first = resource.refresh();
    expect(resource.refresh()).toBe(first);
    await Promise.resolve();
    expect(read).toHaveBeenCalledTimes(1);
    request.resolve("current log");
    await first;
    expect(publish).toHaveBeenLastCalledWith({ value: "current log", loading: false });
  });

  it("does not restore a pre-clear log after an explicit refresh completes", async () => {
    const old = deferred<string>();
    const fresh = deferred<string>();
    const read = vi.fn().mockReturnValueOnce(old.promise).mockReturnValueOnce(fresh.promise);
    const publish = vi.fn();
    const resource = createAsyncResource(read, publish);
    const first = resource.refresh();
    const second = resource.refresh(true);
    fresh.resolve("");
    await second;
    old.resolve("deleted log");
    await first;
    expect(publish).toHaveBeenLastCalledWith({ value: "", loading: false });
  });

  it.each(["resolve", "reject"] as const)("ignores late %s after switching task or unmounting", async (result) => {
    const request = deferred<string>();
    const publish = vi.fn();
    const resource = createAsyncResource(() => request.promise, publish);
    const pending = resource.refresh();
    resource.dispose();
    publish.mockClear();
    request[result]("old task");
    await pending;
    await resource.refresh();
    expect(publish).not.toHaveBeenCalled();
  });

  it("clears stale content on error and allows the next poll to recover", async () => {
    const read = vi.fn().mockResolvedValueOnce("old").mockRejectedValueOnce("offline").mockResolvedValueOnce("new");
    const publish = vi.fn();
    const resource = createAsyncResource(read, publish);
    await resource.refresh();
    await resource.refresh();
    expect(publish).toHaveBeenLastCalledWith({ error: "offline", loading: false });
    await resource.refresh();
    expect(publish).toHaveBeenLastCalledWith({ value: "new", loading: false });
  });
});

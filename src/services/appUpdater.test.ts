import type { DownloadEvent } from "@tauri-apps/plugin-updater";
import { describe, expect, it } from "vitest";
import {
  applyDownloadEvent,
  describeUpdateError,
  downloadPercent,
  initialUpdateState,
  isWindowsRuntime,
  updateViewReducer,
} from "./appUpdater";

describe("updater progress", () => {
  it("uses the real content length when the server provides it", () => {
    let progress = applyDownloadEvent(initialUpdateState.progress, {
      event: "Started",
      data: { contentLength: 1_000 },
    } as DownloadEvent);
    progress = applyDownloadEvent(progress, {
      event: "Progress",
      data: { chunkLength: 420 },
    } as DownloadEvent);

    expect(progress).toEqual({ downloaded: 420, total: 1_000, finished: false });
    expect(downloadPercent(progress)).toBe(42);
  });

  it("keeps progress indeterminate when total size is unavailable", () => {
    let progress = applyDownloadEvent(initialUpdateState.progress, {
      event: "Started",
      data: { contentLength: undefined },
    } as DownloadEvent);
    progress = applyDownloadEvent(progress, {
      event: "Progress",
      data: { chunkLength: 256 },
    } as DownloadEvent);

    expect(progress.downloaded).toBe(256);
    expect(progress.total).toBeUndefined();
    expect(downloadPercent(progress)).toBeUndefined();
  });

  it("marks the real Finished event without inventing bytes", () => {
    const progress = applyDownloadEvent(
      { downloaded: 256, finished: false },
      { event: "Finished" } as DownloadEvent,
    );
    expect(progress).toEqual({ downloaded: 256, finished: true });
  });
});

describe("updater state and errors", () => {
  it("protects repeated actions with explicit busy phases", () => {
    const checking = updateViewReducer(initialUpdateState, { type: "checking" });
    const downloading = updateViewReducer(checking, { type: "download-started" });
    const installing = updateViewReducer(downloading, { type: "installing" });

    expect([checking.phase, downloading.phase, installing.phase]).toEqual([
      "checking",
      "downloading",
      "installing",
    ]);
  });

  it("turns signature, network and cancellation failures into safe user guidance", () => {
    expect(describeUpdateError(new Error("signature verification failed"))).toContain("停止安装");
    expect(describeUpdateError(new Error("network timeout"))).toContain("当前版本没有变化");
    expect(describeUpdateError(new Error("download cancelled"))).toContain("已取消");
  });

  it("distinguishes Windows installer behavior", () => {
    expect(isWindowsRuntime("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")).toBe(true);
    expect(isWindowsRuntime("Mozilla/5.0 (Macintosh; Intel Mac OS X)")).toBe(false);
  });
});

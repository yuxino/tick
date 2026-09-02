import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";

export type AppUpdate = Update;

export const TICK_RELEASES_URL = "https://github.com/yuxino/tick/releases";

export interface UpdateMetadata {
  version: string;
  currentVersion: string;
  body?: string;
  date?: string;
}

export interface DownloadProgress {
  downloaded: number;
  total?: number;
  finished: boolean;
}

export type UpdatePhase =
  | "idle"
  | "checking"
  | "current"
  | "available"
  | "downloading"
  | "installing"
  | "ready"
  | "windows-installer"
  | "error";

export interface UpdateViewState {
  phase: UpdatePhase;
  currentVersion?: string;
  update?: UpdateMetadata;
  progress: DownloadProgress;
  error?: string;
}

export type UpdateViewAction =
  | { type: "version"; version: string }
  | { type: "checking" }
  | { type: "current" }
  | { type: "available"; update: UpdateMetadata }
  | { type: "download-started" }
  | { type: "download-event"; event: DownloadEvent }
  | { type: "installing" }
  | { type: "ready" }
  | { type: "windows-installer" }
  | { type: "error"; message: string };

export const initialUpdateState: UpdateViewState = {
  phase: "idle",
  progress: { downloaded: 0, finished: false },
};

export function updateViewReducer(state: UpdateViewState, action: UpdateViewAction): UpdateViewState {
  switch (action.type) {
    case "version":
      return { ...state, currentVersion: action.version };
    case "checking":
      return { ...state, phase: "checking", update: undefined, error: undefined, progress: initialUpdateState.progress };
    case "current":
      return { ...state, phase: "current", update: undefined, error: undefined };
    case "available":
      return { ...state, phase: "available", update: action.update, error: undefined };
    case "download-started":
      return { ...state, phase: "downloading", error: undefined, progress: initialUpdateState.progress };
    case "download-event":
      return { ...state, progress: applyDownloadEvent(state.progress, action.event) };
    case "installing":
      return { ...state, phase: "installing" };
    case "ready":
      return { ...state, phase: "ready" };
    case "windows-installer":
      return { ...state, phase: "windows-installer" };
    case "error":
      return { ...state, phase: "error", error: action.message };
  }
}

export function applyDownloadEvent(progress: DownloadProgress, event: DownloadEvent): DownloadProgress {
  if (event.event === "Started") {
    const total = event.data.contentLength;
    return {
      downloaded: 0,
      total: typeof total === "number" && total > 0 ? total : undefined,
      finished: false,
    };
  }

  if (event.event === "Progress") {
    return {
      ...progress,
      downloaded: progress.downloaded + Math.max(0, event.data.chunkLength),
    };
  }

  return { ...progress, finished: true };
}

export function downloadPercent(progress: DownloadProgress): number | undefined {
  if (!progress.total) return undefined;
  return Math.min(100, Math.max(0, Math.round((progress.downloaded / progress.total) * 100)));
}

export function describeUpdateError(error: unknown): string {
  const raw = error instanceof Error ? error.message : String(error);
  const normalized = raw.toLowerCase();

  if (normalized.includes("signature") || normalized.includes("public key") || normalized.includes("pubkey")) {
    return "更新签名验证失败，Tick 已停止安装以保护当前应用。请重新检查；若仍失败，可打开 Releases 手动恢复。";
  }
  if (normalized.includes("cancel") || normalized.includes("abort")) {
    return "更新已取消，当前版本没有变化。可以重新检查后再试。";
  }
  if (
    normalized.includes("network") ||
    normalized.includes("timed out") ||
    normalized.includes("timeout") ||
    normalized.includes("http") ||
    normalized.includes("fetch")
  ) {
    return "无法连接更新服务或下载中断。请检查网络后重新检查；当前版本没有变化。";
  }

  const detail = raw.replace(/^Error:\s*/, "").trim();
  return detail ? `更新失败：${detail}` : "更新失败，请重新检查。";
}

export async function currentAppVersion() {
  return getVersion();
}

export async function checkForAppUpdate(): Promise<Update | null> {
  return check({ timeout: 30_000 });
}

export async function openTickReleases() {
  await openUrl(TICK_RELEASES_URL);
}

export async function relaunchTick() {
  await relaunch();
}

export function isWindowsRuntime(userAgent = navigator.userAgent) {
  return userAgent.toLowerCase().includes("windows");
}

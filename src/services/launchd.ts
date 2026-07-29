import { invoke } from "@tauri-apps/api/core";
import type { JobLog, LaunchdJob, LaunchdJobInput, LogKind } from "../types/launchd";

export interface GenerateNodeScriptInput {
  prompt: string;
  currentScript?: string;
}

export interface GenerateNodeScriptResponse {
  script: string;
}

export interface AutomationDraft {
  job: LaunchdJobInput;
  summary: string;
  risks: string[];
}

export interface RunNodeScriptDebugInput {
  script: string;
  workingDirectory?: string;
}

export interface RunNodeScriptDebugResponse {
  stdout: string;
  stderr: string;
  exitCode?: number;
  durationMs: number;
  timedOut: boolean;
}

export function listLaunchdJobs() {
  return invoke<LaunchdJob[]>("list_launchd_jobs");
}

export function saveLaunchdJob(input: LaunchdJobInput) {
  return invoke<LaunchdJob>("save_launchd_job", { input });
}

export function enableLaunchdJob(id: string) {
  return invoke<LaunchdJob>("enable_launchd_job", { id });
}

export function disableLaunchdJob(id: string) {
  return invoke<LaunchdJob>("disable_launchd_job", { id });
}

export function runLaunchdJobNow(id: string) {
  return invoke<void>("run_launchd_job_now", { id });
}

export function deleteLaunchdJob(id: string) {
  return invoke<void>("delete_launchd_job", { id });
}

export function readLaunchdLog(id: string, kind: LogKind) {
  return invoke<JobLog>("read_launchd_log", { id, kind });
}

export function clearLaunchdLog(id: string, kind: LogKind) {
  return invoke<void>("clear_launchd_log", { id, kind });
}

export function readLaunchdPlist(id: string) {
  return invoke<string>("read_launchd_plist", { id });
}

export function printLaunchdJob(id: string) {
  return invoke<string>("print_launchd_job", { id });
}

export function generateNodeScript(input: GenerateNodeScriptInput) {
  return invoke<GenerateNodeScriptResponse>("generate_node_script", { input });
}

export function generateAutomation(prompt: string, currentJob?: LaunchdJobInput) {
  return invoke<AutomationDraft>("generate_automation", {
    input: { prompt, currentJob },
  });
}

export function runNodeScriptDebug(input: RunNodeScriptDebugInput) {
  return invoke<RunNodeScriptDebugResponse>("run_node_script_debug", { input });
}

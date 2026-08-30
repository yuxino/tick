import { invoke } from "@tauri-apps/api/core";
import type {
  JobLog,
  LogKind,
  NodeRuntimeStatus,
  ScheduledJob,
  ScheduledJobInput,
  SchedulerCapabilities,
} from "../types/scheduler";

export interface AutomationDraft {
  job: ScheduledJobInput;
  summary: string;
  risks: string[];
}

interface RunNodeScriptDebugInput {
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

export function getSchedulerCapabilities() {
  return invoke<SchedulerCapabilities & { maximumIntervalSeconds?: number | null }>("get_scheduler_capabilities")
    .then((capabilities) => ({
      ...capabilities,
      maximumIntervalSeconds: capabilities.maximumIntervalSeconds ?? undefined,
    }));
}

export function getNodeRuntimeStatus() {
  return invoke<NodeRuntimeStatus>("get_node_runtime_status");
}

export function listScheduledJobs() {
  return invoke<ScheduledJob[]>("list_scheduled_jobs");
}

export function saveScheduledJob(input: ScheduledJobInput) {
  return invoke<ScheduledJob>("save_scheduled_job", { input });
}

export function enableScheduledJob(id: string) {
  return invoke<ScheduledJob>("enable_scheduled_job", { id });
}

export function disableScheduledJob(id: string) {
  return invoke<ScheduledJob>("disable_scheduled_job", { id });
}

export function runScheduledJobNow(id: string) {
  return invoke<void>("run_scheduled_job_now", { id });
}

export function deleteScheduledJob(id: string) {
  return invoke<void>("delete_scheduled_job", { id });
}

export function readScheduledJobLog(id: string, kind: LogKind) {
  return invoke<JobLog>("read_scheduled_job_log", { id, kind });
}

export function clearScheduledJobLog(id: string, kind: LogKind) {
  return invoke<void>("clear_scheduled_job_log", { id, kind });
}

export function readJobDefinition(id: string) {
  return invoke<string>("read_job_definition", { id });
}

export function generateAutomation(prompt: string) {
  return invoke<AutomationDraft>("generate_automation", {
    input: { prompt },
  });
}

export function runNodeScriptDebug(input: RunNodeScriptDebugInput) {
  return invoke<RunNodeScriptDebugResponse>("run_node_script_debug", { input });
}

export type JobStatus = "enabled" | "disabled" | "missing" | "error";

export type ExecutionMode = "inline_shell" | "script_path" | "interpreter";

type ScheduleMode = "calendar" | "interval";

export type LogKind = "stdout" | "stderr";

interface CalendarSchedule {
  month?: number;
  day?: number;
  hour?: number;
  minute?: number;
  second: number;
}

interface IntervalSchedule {
  seconds: number;
}

export interface JobSchedule {
  mode: ScheduleMode;
  calendar: CalendarSchedule;
  interval: IntervalSchedule;
}

export interface JobExecution {
  mode: ExecutionMode;
  inlineScript: string;
  scriptPath: string;
  interpreter: string;
  arguments: string;
  workingDirectory: string;
  environment: Array<{ key: string; value: string }>;
}

export interface ScheduledJob {
  id: string;
  label: string;
  name: string;
  description: string;
  status: JobStatus;
  schedule: JobSchedule;
  execution: JobExecution;
  stdoutPath: string;
  stderrPath: string;
  definitionPath: string;
  lastModifiedAt: string;
}

export interface ScheduledJobInput {
  id?: string;
  name: string;
  description: string;
  schedule: JobSchedule;
  execution: JobExecution;
}

export interface SchedulerCapabilities {
  platform: string;
  computerLabel: string;
  schedulerName: string;
  definitionLabel: string;
  defaultInterpreter: string;
  scriptPathExample: string;
  workingDirectoryExample: string;
  homeDirectory: string;
  trashLabel: string;
  minimumIntervalSeconds: number;
  maximumIntervalSeconds?: number;
}

export interface NodeRuntimeStatus {
  available: boolean;
  version?: string;
  executablePath?: string;
  reason?: string;
}

export interface JobLog {
  kind: LogKind;
  path: string;
  content: string;
  size: number;
  truncated: boolean;
}

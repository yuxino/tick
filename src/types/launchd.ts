export type JobStatus = "enabled" | "disabled" | "missing" | "error";

export type ExecutionMode = "inline_shell" | "script_path" | "interpreter";

export type ScheduleMode = "calendar" | "interval";

export type LogKind = "stdout" | "stderr";

export interface CalendarSchedule {
  month?: number;
  day?: number;
  hour?: number;
  minute?: number;
  second: number;
}

export interface IntervalSchedule {
  seconds: number;
}

export interface LaunchdSchedule {
  mode: ScheduleMode;
  calendar: CalendarSchedule;
  interval: IntervalSchedule;
}

export interface LaunchdExecution {
  mode: ExecutionMode;
  inlineScript: string;
  scriptPath: string;
  interpreter: string;
  arguments: string;
  workingDirectory: string;
  environment: Array<{ key: string; value: string }>;
}

export interface LaunchdJob {
  id: string;
  label: string;
  name: string;
  description: string;
  status: JobStatus;
  schedule: LaunchdSchedule;
  execution: LaunchdExecution;
  stdoutPath: string;
  stderrPath: string;
  plistPath: string;
  lastModifiedAt: string;
}

export interface LaunchdJobInput {
  id?: string;
  name: string;
  description: string;
  schedule: LaunchdSchedule;
  execution: LaunchdExecution;
}

export interface JobLog {
  kind: LogKind;
  path: string;
  content: string;
  size: number;
  truncated: boolean;
}

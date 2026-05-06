import type { JobStatus, LaunchdExecution, LaunchdJob, LaunchdJobInput, LaunchdSchedule } from "../types/launchd";

export const defaultSchedule: LaunchdSchedule = {
  mode: "calendar",
  calendar: {
    hour: 9,
    minute: 0,
    second: 0,
  },
  interval: {
    seconds: 60,
  },
};

export const defaultExecution: LaunchdExecution = {
  mode: "inline_shell",
  inlineScript: `console.log("tick", new Date().toLocaleString());`,
  scriptPath: "",
  interpreter: "/usr/bin/env node",
  arguments: "",
  workingDirectory: "",
  environment: [],
};

export function emptyJobInput(): LaunchdJobInput {
  return {
    name: "",
    description: "",
    schedule: structuredClone(defaultSchedule),
    execution: structuredClone(defaultExecution),
  };
}

export function toJobInput(job: LaunchdJob): LaunchdJobInput {
  return {
    id: job.id,
    name: job.name,
    description: job.description,
    schedule: structuredClone(job.schedule),
    execution: structuredClone(job.execution),
  };
}

export function scheduleSummary(schedule: LaunchdSchedule) {
  if (schedule.mode === "interval") {
    return `每 ${schedule.interval.seconds} 秒执行一次`;
  }

  const { month, day, hour, minute, second } = schedule.calendar;
  const time = `${pad(hour ?? 0)}:${pad(minute ?? 0)}:${pad(second)}`;

  if (month && day) {
    return `每年 ${month} 月 ${day} 日 ${time}`;
  }
  if (month) {
    return `每年 ${month} 月每天 ${time}`;
  }
  if (day) {
    return `每月 ${day} 日 ${time}`;
  }
  return `每天 ${time}`;
}

export function commandSummary(execution: LaunchdExecution) {
  if (execution.mode === "inline_shell") {
    return execution.inlineScript?.split("\n").find((line) => line.trim())?.trim() || "Node.js 脚本";
  }
  if (execution.mode === "script_path") {
    return execution.scriptPath || "脚本路径";
  }
  return [execution.interpreter, execution.scriptPath].filter(Boolean).join(" ") || "解释器命令";
}

export function statusLabel(status: JobStatus) {
  const labels: Record<JobStatus, string> = {
    enabled: "已启用",
    disabled: "已停用",
    missing: "文件缺失",
    error: "异常",
  };
  return labels[status];
}

function pad(value: number) {
  return String(value).padStart(2, "0");
}

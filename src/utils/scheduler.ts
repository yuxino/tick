import type {
  JobExecution,
  JobSchedule,
  JobStatus,
  ScheduledJob,
  ScheduledJobInput,
  SchedulerCapabilities,
} from "../types/scheduler";

export const fallbackSchedulerCapabilities: SchedulerCapabilities = {
  platform: "unknown",
  computerLabel: "电脑",
  schedulerName: "系统任务调度器",
  definitionLabel: "任务定义",
  defaultInterpreter: "node",
  scriptPathExample: "请输入脚本的绝对路径",
  workingDirectoryExample: "请输入工作目录的绝对路径",
  homeDirectory: "",
  trashLabel: "回收位置",
  minimumIntervalSeconds: 1,
};

export const defaultSchedule: JobSchedule = {
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

export function defaultExecution(capabilities: SchedulerCapabilities): JobExecution {
  return {
    mode: "inline_shell",
    inlineScript: `console.log("tick", new Date().toLocaleString());`,
    scriptPath: "",
    interpreter: capabilities.defaultInterpreter,
    arguments: "",
    workingDirectory: "",
    environment: [],
  };
}

export function emptyJobInput(capabilities: SchedulerCapabilities): ScheduledJobInput {
  const schedule = structuredClone(defaultSchedule);
  schedule.interval.seconds = Math.max(schedule.interval.seconds, capabilities.minimumIntervalSeconds);
  if (capabilities.maximumIntervalSeconds !== undefined) {
    schedule.interval.seconds = Math.min(schedule.interval.seconds, capabilities.maximumIntervalSeconds);
  }

  return {
    name: "",
    description: "",
    schedule,
    execution: defaultExecution(capabilities),
  };
}

export function toJobInput(job: ScheduledJob): ScheduledJobInput {
  return {
    id: job.id,
    name: job.name,
    description: job.description,
    schedule: structuredClone(job.schedule),
    execution: structuredClone(job.execution),
  };
}

export function scheduleSummary(schedule: JobSchedule) {
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

export function commandSummary(execution: JobExecution) {
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
    missing: "任务配置缺失",
    error: "异常",
  };
  return labels[status];
}

function pad(value: number) {
  return String(value).padStart(2, "0");
}

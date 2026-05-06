import type { LaunchdExecution, LaunchdJob, LaunchdJobInput, LaunchdSchedule } from "../types/launchd";

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
  inlineScript: "echo \"tick $(date)\"",
  scriptPath: "",
  interpreter: "/bin/sh",
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
    return `Every ${schedule.interval.seconds} second${schedule.interval.seconds === 1 ? "" : "s"}`;
  }

  const { month, day, hour, minute, second } = schedule.calendar;
  const dateParts = [
    month ? `month ${month}` : "every month",
    day ? `day ${day}` : "every day",
  ];
  const time = `${pad(hour ?? 0)}:${pad(minute ?? 0)}:${pad(second)}`;

  if (month || day) {
    return `${dateParts.join(", ")} at ${time}`;
  }
  return `Every day at ${time}`;
}

export function commandSummary(execution: LaunchdExecution) {
  if (execution.mode === "inline_shell") {
    return "Inline shell";
  }
  if (execution.mode === "script_path") {
    return execution.scriptPath || "Script path";
  }
  return [execution.interpreter, execution.scriptPath].filter(Boolean).join(" ") || "Interpreter";
}

function pad(value: number) {
  return String(value).padStart(2, "0");
}

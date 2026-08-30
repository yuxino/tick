use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Enabled,
    Disabled,
    Missing,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    InlineShell,
    ScriptPath,
    Interpreter,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleMode {
    Calendar,
    Interval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarSchedule {
    pub month: Option<u8>,
    pub day: Option<u8>,
    pub hour: Option<u8>,
    pub minute: Option<u8>,
    pub second: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntervalSchedule {
    pub seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSchedule {
    pub mode: ScheduleMode,
    pub calendar: CalendarSchedule,
    pub interval: IntervalSchedule,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentVar {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobExecution {
    pub mode: ExecutionMode,
    #[serde(rename = "inlineScript")]
    pub inline_script: String,
    #[serde(rename = "scriptPath")]
    pub script_path: String,
    pub interpreter: String,
    pub arguments: String,
    #[serde(rename = "workingDirectory")]
    pub working_directory: String,
    pub environment: Vec<EnvironmentVar>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJobInput {
    pub id: Option<String>,
    pub name: String,
    pub description: String,
    pub schedule: JobSchedule,
    pub execution: JobExecution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub id: String,
    pub label: String,
    pub name: String,
    pub description: String,
    pub status: JobStatus,
    pub schedule: JobSchedule,
    pub execution: JobExecution,
    #[serde(rename = "stdoutPath")]
    pub stdout_path: String,
    #[serde(rename = "stderrPath")]
    pub stderr_path: String,
    #[serde(rename = "definitionPath", alias = "plistPath")]
    pub definition_path: String,
    #[serde(rename = "lastModifiedAt")]
    pub last_modified_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerCapabilities {
    pub platform: &'static str,
    pub computer_label: &'static str,
    pub scheduler_name: &'static str,
    pub definition_label: &'static str,
    pub default_interpreter: &'static str,
    pub script_path_example: &'static str,
    pub working_directory_example: &'static str,
    pub home_directory: String,
    pub trash_label: &'static str,
    pub minimum_interval_seconds: u32,
    pub maximum_interval_seconds: Option<u32>,
}

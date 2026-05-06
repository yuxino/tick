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
pub struct LaunchdSchedule {
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
pub struct LaunchdExecution {
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
pub struct LaunchdJobInput {
    pub id: Option<String>,
    pub name: String,
    pub description: String,
    pub schedule: LaunchdSchedule,
    pub execution: LaunchdExecution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchdJob {
    pub id: String,
    pub label: String,
    pub name: String,
    pub description: String,
    pub status: JobStatus,
    pub schedule: LaunchdSchedule,
    pub execution: LaunchdExecution,
    #[serde(rename = "stdoutPath")]
    pub stdout_path: String,
    #[serde(rename = "stderrPath")]
    pub stderr_path: String,
    #[serde(rename = "plistPath")]
    pub plist_path: String,
    #[serde(rename = "lastModifiedAt")]
    pub last_modified_at: String,
}

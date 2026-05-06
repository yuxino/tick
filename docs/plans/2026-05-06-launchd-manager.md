# Launchd Manager Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a polished Tauri desktop page for creating, editing, enabling, disabling, running, deleting, and inspecting macOS `launchd` jobs with month/day/hour/minute/second scheduling, script command support, code highlighting, and log management.

**Architecture:** The React app owns the Ant Design interface and calls typed Tauri commands. Rust owns all `launchd` operations, plist generation/parsing, log file management, and safe command execution through `launchctl`. Jobs are stored as user LaunchAgents under `~/Library/LaunchAgents/com.gavin.tick.<slug>.plist`, with stdout/stderr logs under the app data log directory.

**Tech Stack:** Tauri 2, React 19, TypeScript, Ant Design 5, CodeMirror 6, Rust, `launchctl`, `plist`, `serde`, `chrono`, `dirs`, `thiserror`.

---

## Product Decisions

- Scope is macOS user LaunchAgents only. Do not touch system daemons in `/Library/LaunchDaemons`.
- Job labels use prefix `com.gavin.tick.` and are derived from a user-facing name plus a stable id.
- Support three execution modes:
  - Inline shell script: write a `.sh` file managed by Tick and execute it through `/bin/sh`.
  - Script file path: execute a user-specified script path with arguments.
  - Interpreter command: choose `node`, `python3`, `bash`, `zsh`, or custom binary, then pass script path or inline script file.
- Support two schedule modes:
  - Calendar schedule: `Month`, `Day`, `Hour`, `Minute`, plus `Second`. Because `StartCalendarInterval` has no seconds field, implement second offset by generating a wrapper command that sleeps `Second` seconds before running the user command.
  - Interval schedule: run every N seconds using `StartInterval`.
- Logs are files, not database rows. Each job gets stdout and stderr log files. The UI can read, refresh, clear, and auto-refresh them.
- "Enabled" means the plist exists and is bootstrapped into `gui/$UID`; disabled means it has been booted out, while the saved job remains visible in Tick's own registry.
- Keep the first version local-only. No cloud sync, no background agent beyond `launchd`.

## Dependencies

Run:

```bash
npm install antd @ant-design/icons @uiw/react-codemirror @codemirror/lang-javascript @codemirror/lang-shell @codemirror/lang-json dayjs
cargo add plist chrono dirs thiserror shell-words uuid
```

Expected:

```text
added packages...
Locking packages...
```

Commit:

```bash
git add package.json package-lock.json src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: add launchd manager dependencies"
```

---

### Task 1: Define Shared TypeScript Models

**Files:**
- Create: `src/types/launchd.ts`

**Step 1: Write the type file**

Create `src/types/launchd.ts`:

```ts
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
```

**Step 2: Run TypeScript**

Run:

```bash
npm run build
```

Expected: PASS.

**Step 3: Commit**

```bash
git add src/types/launchd.ts
git commit -m "feat: add launchd job types"
```

---

### Task 2: Create Rust Domain Models

**Files:**
- Create: `src-tauri/src/launchd/mod.rs`
- Create: `src-tauri/src/launchd/models.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Write the Rust models**

Create `src-tauri/src/launchd/mod.rs`:

```rust
pub mod models;
```

Create `src-tauri/src/launchd/models.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Enabled,
    Disabled,
    Missing,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    InlineShell,
    ScriptPath,
    Interpreter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
```

Modify `src-tauri/src/lib.rs`:

```rust
mod launchd;
```

Place it at the top of the file.

**Step 2: Run Rust check**

Run:

```bash
cd src-tauri && cargo check
```

Expected: PASS, possibly with dead code warnings.

**Step 3: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/launchd
git commit -m "feat: add launchd domain models"
```

---

### Task 3: Add Validation Tests

**Files:**
- Create: `src-tauri/src/launchd/validation.rs`
- Modify: `src-tauri/src/launchd/mod.rs`

**Step 1: Write failing tests**

Create `src-tauri/src/launchd/validation.rs`:

```rust
use super::models::{ExecutionMode, LaunchdJobInput, ScheduleMode};

pub fn validate_job_input(input: &LaunchdJobInput) -> Result<(), String> {
    todo!("validate launchd job input")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launchd::models::{
        CalendarSchedule, IntervalSchedule, LaunchdExecution, LaunchdSchedule,
    };

    fn valid_input() -> LaunchdJobInput {
        LaunchdJobInput {
            id: None,
            name: "Nightly sync".to_string(),
            description: "".to_string(),
            schedule: LaunchdSchedule {
                mode: ScheduleMode::Calendar,
                calendar: CalendarSchedule {
                    month: None,
                    day: None,
                    hour: Some(23),
                    minute: Some(30),
                    second: 5,
                },
                interval: IntervalSchedule { seconds: 300 },
            },
            execution: LaunchdExecution {
                mode: ExecutionMode::InlineShell,
                inline_script: "echo ok".to_string(),
                script_path: "".to_string(),
                interpreter: "/bin/sh".to_string(),
                arguments: "".to_string(),
                working_directory: "".to_string(),
                environment: vec![],
            },
        }
    }

    #[test]
    fn accepts_valid_calendar_job() {
        assert!(validate_job_input(&valid_input()).is_ok());
    }

    #[test]
    fn rejects_empty_name() {
        let mut input = valid_input();
        input.name = "   ".to_string();
        assert_eq!(validate_job_input(&input), Err("Name is required".to_string()));
    }

    #[test]
    fn rejects_invalid_calendar_second() {
        let mut input = valid_input();
        input.schedule.calendar.second = 60;
        assert_eq!(
            validate_job_input(&input),
            Err("Second must be between 0 and 59".to_string())
        );
    }

    #[test]
    fn rejects_invalid_interval() {
        let mut input = valid_input();
        input.schedule.mode = ScheduleMode::Interval;
        input.schedule.interval.seconds = 0;
        assert_eq!(
            validate_job_input(&input),
            Err("Interval must be at least 1 second".to_string())
        );
    }

    #[test]
    fn rejects_empty_inline_script() {
        let mut input = valid_input();
        input.execution.inline_script = " ".to_string();
        assert_eq!(
            validate_job_input(&input),
            Err("Inline shell script is required".to_string())
        );
    }
}
```

Modify `src-tauri/src/launchd/mod.rs`:

```rust
pub mod models;
pub mod validation;
```

**Step 2: Run test to verify it fails**

Run:

```bash
cd src-tauri && cargo test launchd::validation
```

Expected: FAIL because `validate_job_input` is `todo!`.

**Step 3: Implement validation**

Replace `validate_job_input`:

```rust
pub fn validate_job_input(input: &LaunchdJobInput) -> Result<(), String> {
    if input.name.trim().is_empty() {
        return Err("Name is required".to_string());
    }

    match input.schedule.mode {
        ScheduleMode::Calendar => {
            if let Some(month) = input.schedule.calendar.month {
                if !(1..=12).contains(&month) {
                    return Err("Month must be between 1 and 12".to_string());
                }
            }
            if let Some(day) = input.schedule.calendar.day {
                if !(1..=31).contains(&day) {
                    return Err("Day must be between 1 and 31".to_string());
                }
            }
            if let Some(hour) = input.schedule.calendar.hour {
                if hour > 23 {
                    return Err("Hour must be between 0 and 23".to_string());
                }
            }
            if let Some(minute) = input.schedule.calendar.minute {
                if minute > 59 {
                    return Err("Minute must be between 0 and 59".to_string());
                }
            }
            if input.schedule.calendar.second > 59 {
                return Err("Second must be between 0 and 59".to_string());
            }
        }
        ScheduleMode::Interval => {
            if input.schedule.interval.seconds == 0 {
                return Err("Interval must be at least 1 second".to_string());
            }
        }
    }

    match input.execution.mode {
        ExecutionMode::InlineShell if input.execution.inline_script.trim().is_empty() => {
            Err("Inline shell script is required".to_string())
        }
        ExecutionMode::ScriptPath if input.execution.script_path.trim().is_empty() => {
            Err("Script path is required".to_string())
        }
        ExecutionMode::Interpreter if input.execution.interpreter.trim().is_empty() => {
            Err("Interpreter is required".to_string())
        }
        _ => Ok(()),
    }
}
```

**Step 4: Run test to verify it passes**

Run:

```bash
cd src-tauri && cargo test launchd::validation
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src-tauri/src/launchd
git commit -m "feat: validate launchd job input"
```

---

### Task 4: Implement Path and Registry Storage

**Files:**
- Create: `src-tauri/src/launchd/paths.rs`
- Create: `src-tauri/src/launchd/registry.rs`
- Modify: `src-tauri/src/launchd/mod.rs`

**Step 1: Write path helpers**

Create `src-tauri/src/launchd/paths.rs`:

```rust
use std::path::PathBuf;

pub const LABEL_PREFIX: &str = "com.gavin.tick.";

pub fn home_dir() -> Result<PathBuf, String> {
    dirs::home_dir().ok_or_else(|| "Unable to resolve home directory".to_string())
}

pub fn launch_agents_dir() -> Result<PathBuf, String> {
    Ok(home_dir()?.join("Library").join("LaunchAgents"))
}

pub fn app_data_dir() -> Result<PathBuf, String> {
    let dir = dirs::data_dir()
        .ok_or_else(|| "Unable to resolve app data directory".to_string())?
        .join("tick");
    Ok(dir)
}

pub fn registry_path() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("launchd-jobs.json"))
}

pub fn scripts_dir() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("scripts"))
}

pub fn logs_dir() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("logs"))
}

pub fn wrappers_dir() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("wrappers"))
}

pub fn ensure_dirs() -> Result<(), String> {
    for dir in [launch_agents_dir()?, app_data_dir()?, scripts_dir()?, logs_dir()?, wrappers_dir()?] {
        std::fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    }
    Ok(())
}
```

**Step 2: Write registry helpers**

Create `src-tauri/src/launchd/registry.rs` with functions:

```rust
use super::models::LaunchdJob;
use super::paths::{ensure_dirs, registry_path};

pub fn read_registry() -> Result<Vec<LaunchdJob>, String> {
    ensure_dirs()?;
    let path = registry_path()?;
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&content).map_err(|err| err.to_string())
}

pub fn write_registry(jobs: &[LaunchdJob]) -> Result<(), String> {
    ensure_dirs()?;
    let content = serde_json::to_string_pretty(jobs).map_err(|err| err.to_string())?;
    std::fs::write(registry_path()?, content).map_err(|err| err.to_string())
}

pub fn upsert_job(job: LaunchdJob) -> Result<LaunchdJob, String> {
    let mut jobs = read_registry()?;
    if let Some(existing) = jobs.iter_mut().find(|item| item.id == job.id) {
        *existing = job.clone();
    } else {
        jobs.push(job.clone());
    }
    write_registry(&jobs)?;
    Ok(job)
}

pub fn delete_job(id: &str) -> Result<(), String> {
    let jobs = read_registry()?
        .into_iter()
        .filter(|job| job.id != id)
        .collect::<Vec<_>>();
    write_registry(&jobs)
}
```

Modify `src-tauri/src/launchd/mod.rs`:

```rust
pub mod models;
pub mod paths;
pub mod registry;
pub mod validation;
```

**Step 3: Run checks**

Run:

```bash
cd src-tauri && cargo check
```

Expected: PASS.

**Step 4: Commit**

```bash
git add src-tauri/src/launchd
git commit -m "feat: add launchd job registry storage"
```

---

### Task 5: Generate Plists and Wrapper Scripts

**Files:**
- Create: `src-tauri/src/launchd/plist_writer.rs`
- Modify: `src-tauri/src/launchd/mod.rs`

**Step 1: Write failing unit tests**

Create tests inside `plist_writer.rs` for:

```rust
#[test]
fn calendar_schedule_uses_start_calendar_interval() {
    // Build a calendar job with second = 0.
    // Assert generated plist contains StartCalendarInterval.
}

#[test]
fn calendar_schedule_with_second_generates_sleep_wrapper() {
    // Build a calendar job with second = 15.
    // Assert wrapper contains "sleep 15".
}

#[test]
fn interval_schedule_uses_start_interval() {
    // Build interval job with 30 seconds.
    // Assert generated plist contains StartInterval = 30.
}
```

**Step 2: Run tests to verify they fail**

Run:

```bash
cd src-tauri && cargo test launchd::plist_writer
```

Expected: FAIL because implementation is missing.

**Step 3: Implement plist generation**

Implement these functions:

```rust
pub struct MaterializedJob {
    pub plist_path: std::path::PathBuf,
    pub stdout_path: std::path::PathBuf,
    pub stderr_path: std::path::PathBuf,
    pub wrapper_path: Option<std::path::PathBuf>,
    pub inline_script_path: Option<std::path::PathBuf>,
}

pub fn write_job_files(job: &LaunchdJob) -> Result<MaterializedJob, String> {
    // ensure dirs
    // write inline shell script when needed
    // write wrapper script when calendar.second > 0
    // write plist to ~/Library/LaunchAgents/<label>.plist
}
```

Implementation rules:

- `Label`: `job.label`
- `ProgramArguments`: wrapper script path if second offset exists, otherwise direct command parts.
- `WorkingDirectory`: only include when non-empty.
- `EnvironmentVariables`: include when provided.
- `StandardOutPath`: `job.stdout_path`
- `StandardErrorPath`: `job.stderr_path`
- `RunAtLoad`: false by default.
- `StartCalendarInterval`: dictionary with optional `Month`, `Day`, `Hour`, `Minute`.
- `StartInterval`: integer seconds for interval mode.
- Wrapper script:

```sh
#!/bin/sh
sleep 15
exec /actual/program arg1 arg2
```

- Quote command arguments using `shell-words` or avoid shell by writing wrapper `exec "$program" "$arg1"` style.
- Set script permissions to `0o755` on macOS.

**Step 4: Run tests**

Run:

```bash
cd src-tauri && cargo test launchd::plist_writer
cd src-tauri && cargo check
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src-tauri/src/launchd
git commit -m "feat: generate launchd plist files"
```

---

### Task 6: Implement Launchctl Operations

**Files:**
- Create: `src-tauri/src/launchd/launchctl.rs`
- Modify: `src-tauri/src/launchd/mod.rs`

**Step 1: Write command wrapper**

Create `launchctl.rs`:

```rust
use std::path::Path;
use std::process::Command;

fn gui_target() -> Result<String, String> {
    let uid = unsafe { libc::getuid() };
    Ok(format!("gui/{uid}"))
}

fn run_launchctl(args: &[String]) -> Result<String, String> {
    let output = Command::new("launchctl")
        .args(args)
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}
```

Add public functions:

```rust
pub fn bootstrap(plist_path: &Path) -> Result<(), String>;
pub fn bootout(label: &str, plist_path: &Path) -> Result<(), String>;
pub fn kickstart(label: &str) -> Result<(), String>;
pub fn print_job(label: &str) -> Result<String, String>;
```

Use:

```bash
launchctl bootstrap gui/$UID <plist>
launchctl bootout gui/$UID/<label>
launchctl kickstart -k gui/$UID/<label>
launchctl print gui/$UID/<label>
```

Fallback for `bootout`: if label target fails, try `launchctl bootout gui/$UID <plist>`.

**Step 2: Add dependency if needed**

If `libc` is not available transitively, run:

```bash
cd src-tauri && cargo add libc
```

**Step 3: Run checks**

Run:

```bash
cd src-tauri && cargo check
```

Expected: PASS.

**Step 4: Commit**

```bash
git add src-tauri/src/launchd src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat: add launchctl operations"
```

---

### Task 7: Add Tauri Commands

**Files:**
- Create: `src-tauri/src/launchd/commands.rs`
- Modify: `src-tauri/src/launchd/mod.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Implement commands**

Create `commands.rs` with:

```rust
#[tauri::command]
pub fn list_launchd_jobs() -> Result<Vec<LaunchdJob>, String>;

#[tauri::command]
pub fn save_launchd_job(input: LaunchdJobInput) -> Result<LaunchdJob, String>;

#[tauri::command]
pub fn enable_launchd_job(id: String) -> Result<LaunchdJob, String>;

#[tauri::command]
pub fn disable_launchd_job(id: String) -> Result<LaunchdJob, String>;

#[tauri::command]
pub fn run_launchd_job_now(id: String) -> Result<(), String>;

#[tauri::command]
pub fn delete_launchd_job(id: String) -> Result<(), String>;
```

Rules:

- `save_launchd_job` validates input, assigns `uuid`, label, log paths, plist path, writes files, updates registry.
- `enable_launchd_job` writes plist and runs `launchctl bootstrap`.
- `disable_launchd_job` runs `launchctl bootout` and marks disabled.
- `run_launchd_job_now` uses `launchctl kickstart -k`.
- `delete_launchd_job` disables first, removes plist, wrapper, inline script, and registry row.
- `list_launchd_jobs` reads registry and checks each plist existence. Do not silently import unrelated LaunchAgents.

**Step 2: Wire commands**

Modify `src-tauri/src/launchd/mod.rs`:

```rust
pub mod commands;
pub mod launchctl;
pub mod models;
pub mod paths;
pub mod plist_writer;
pub mod registry;
pub mod validation;
```

Modify `src-tauri/src/lib.rs`:

```rust
.invoke_handler(tauri::generate_handler![
    greet,
    launchd::commands::list_launchd_jobs,
    launchd::commands::save_launchd_job,
    launchd::commands::enable_launchd_job,
    launchd::commands::disable_launchd_job,
    launchd::commands::run_launchd_job_now,
    launchd::commands::delete_launchd_job,
])
```

**Step 3: Run checks**

Run:

```bash
cd src-tauri && cargo check
```

Expected: PASS.

**Step 4: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/launchd
git commit -m "feat: expose launchd management commands"
```

---

### Task 8: Add Log Commands

**Files:**
- Create: `src-tauri/src/launchd/logs.rs`
- Modify: `src-tauri/src/launchd/commands.rs`
- Modify: `src-tauri/src/launchd/mod.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Write log helpers**

Create `logs.rs`:

```rust
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobLog {
    pub kind: String,
    pub path: String,
    pub content: String,
    pub size: u64,
    pub truncated: bool,
}

pub fn read_log(kind: &str, path: &Path, max_bytes: u64) -> Result<JobLog, String> {
    if !path.exists() {
        return Ok(JobLog {
            kind: kind.to_string(),
            path: path.display().to_string(),
            content: String::new(),
            size: 0,
            truncated: false,
        });
    }
    let bytes = std::fs::read(path).map_err(|err| err.to_string())?;
    let size = bytes.len() as u64;
    let truncated = size > max_bytes;
    let start = if truncated { (size - max_bytes) as usize } else { 0 };
    let content = String::from_utf8_lossy(&bytes[start..]).to_string();
    Ok(JobLog {
        kind: kind.to_string(),
        path: path.display().to_string(),
        content,
        size,
        truncated,
    })
}

pub fn clear_log(path: &Path) -> Result<(), String> {
    std::fs::write(path, "").map_err(|err| err.to_string())
}
```

**Step 2: Add Tauri commands**

Add commands:

```rust
#[tauri::command]
pub fn read_launchd_log(id: String, kind: String) -> Result<JobLog, String>;

#[tauri::command]
pub fn clear_launchd_log(id: String, kind: String) -> Result<(), String>;
```

Register in `src-tauri/src/lib.rs`.

**Step 3: Run checks**

Run:

```bash
cd src-tauri && cargo check
npm run build
```

Expected: PASS.

**Step 4: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/launchd
git commit -m "feat: add launchd log commands"
```

---

### Task 9: Build Frontend API Client

**Files:**
- Create: `src/services/launchd.ts`

**Step 1: Implement typed invoke wrappers**

Create `src/services/launchd.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";
import type { JobLog, LaunchdJob, LaunchdJobInput, LogKind } from "../types/launchd";

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
```

**Step 2: Run TypeScript**

Run:

```bash
npm run build
```

Expected: PASS.

**Step 3: Commit**

```bash
git add src/services/launchd.ts
git commit -m "feat: add launchd frontend service"
```

---

### Task 10: Add Ant Design App Shell

**Files:**
- Modify: `src/main.tsx`
- Replace: `src/App.tsx`
- Replace: `src/App.css`

**Step 1: Import Ant Design CSS**

Modify `src/main.tsx`:

```ts
import "antd/dist/reset.css";
import "./App.css";
```

Remove `import "./App.css";` from `App.tsx` later if duplicated.

**Step 2: Replace the starter app**

Replace `src/App.tsx` with an Ant Design `Layout`:

- Header: app name `Tick`, primary button `New Job`.
- Content split:
  - Left: jobs table/list with status tag, schedule summary, next action buttons.
  - Right: detail panel with tabs `Overview`, `Logs`, `Plist`.
- Use empty state when no jobs exist.

**Step 3: Style as a serious tool**

Replace `src/App.css`:

```css
html,
body,
#root {
  min-height: 100%;
  margin: 0;
}

body {
  background: #f4f6f8;
}

.app-shell {
  min-height: 100vh;
}

.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 56px;
  padding: 0 20px;
  background: #111827;
}

.app-title {
  color: #fff;
  font-size: 18px;
  font-weight: 650;
}

.app-content {
  padding: 20px;
}

.workspace {
  display: grid;
  grid-template-columns: minmax(360px, 44%) minmax(0, 1fr);
  gap: 16px;
}

@media (max-width: 900px) {
  .workspace {
    grid-template-columns: 1fr;
  }
}
```

**Step 4: Run TypeScript**

Run:

```bash
npm run build
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src/main.tsx src/App.tsx src/App.css
git commit -m "feat: add launchd manager app shell"
```

---

### Task 11: Build Job Form Modal

**Files:**
- Create: `src/components/JobFormModal.tsx`
- Modify: `src/App.tsx`

**Step 1: Create modal form**

Create `JobFormModal.tsx` using Ant Design:

- `Modal`
- `Form`
- `Input`
- `Input.TextArea`
- `Segmented` for schedule mode
- `InputNumber` for Month, Day, Hour, Minute, Second
- `Segmented` for execution mode
- CodeMirror editor for inline shell
- `Input` for script path, interpreter, args, working directory
- dynamic env var list with `Form.List`

Validation:

- name required
- calendar second 0-59
- interval seconds >= 1
- inline shell required for inline mode
- script path required for script path mode
- interpreter required for interpreter mode

**Step 2: Add code highlighting**

Use CodeMirror:

```tsx
import CodeMirror from "@uiw/react-codemirror";
import { shell } from "@codemirror/lang-shell";
import { javascript } from "@codemirror/lang-javascript";
```

Language selection:

- shell mode: `shell()`
- node interpreter: `javascript()`
- otherwise shell fallback

**Step 3: Wire save**

In `App.tsx`, open modal for create/edit and call `saveLaunchdJob`.

Expected UX:

- Save button shows loading.
- Success message after save.
- Validation errors stay inside form.
- Existing job opens pre-filled.

**Step 4: Run TypeScript**

Run:

```bash
npm run build
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src/App.tsx src/components/JobFormModal.tsx
git commit -m "feat: add launchd job editor"
```

---

### Task 12: Build Job List Actions

**Files:**
- Create: `src/components/JobsTable.tsx`
- Modify: `src/App.tsx`

**Step 1: Create table**

Columns:

- Name
- Status
- Schedule
- Command
- Modified
- Actions

Actions:

- Enable/Disable `Switch`
- Run now button
- Edit button
- Delete button with `Popconfirm`

**Step 2: Implement schedule summary**

Examples:

- `Every day at 09:30:05`
- `Every month on day 1 at 00:00:00`
- `Every 30 seconds`

**Step 3: Wire service calls**

Use:

```ts
enableLaunchdJob(job.id)
disableLaunchdJob(job.id)
runLaunchdJobNow(job.id)
deleteLaunchdJob(job.id)
```

Refresh jobs after each action.

**Step 4: Run build**

Run:

```bash
npm run build
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src/App.tsx src/components/JobsTable.tsx
git commit -m "feat: add launchd job actions"
```

---

### Task 13: Build Log Management Panel

**Files:**
- Create: `src/components/LogsPanel.tsx`
- Modify: `src/App.tsx`

**Step 1: Create log panel**

Use Ant Design:

- `Tabs` for stdout/stderr
- `Typography.Text` showing log path
- `Button` refresh
- `Button` clear with confirmation
- `Switch` auto-refresh
- `CodeMirror` read-only viewer

**Step 2: Implement behavior**

- When selected job changes, load stdout by default.
- Auto-refresh every 2 seconds when enabled.
- Show truncated warning when `truncated` is true.
- Clear log writes empty file through Tauri command then refreshes.

**Step 3: Run build**

Run:

```bash
npm run build
```

Expected: PASS.

**Step 4: Commit**

```bash
git add src/App.tsx src/components/LogsPanel.tsx
git commit -m "feat: add launchd log viewer"
```

---

### Task 14: Add Plist Preview and Diagnostics

**Files:**
- Add command to: `src-tauri/src/launchd/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/services/launchd.ts`
- Create: `src/components/PlistPanel.tsx`
- Modify: `src/App.tsx`

**Step 1: Add command**

```rust
#[tauri::command]
pub fn read_launchd_plist(id: String) -> Result<String, String> {
    let job = registry::read_registry()?
        .into_iter()
        .find(|job| job.id == id)
        .ok_or_else(|| "Job not found".to_string())?;
    std::fs::read_to_string(job.plist_path).map_err(|err| err.to_string())
}
```

Register the command.

**Step 2: Add frontend service**

```ts
export function readLaunchdPlist(id: string) {
  return invoke<string>("read_launchd_plist", { id });
}
```

**Step 3: Create panel**

Render read-only CodeMirror with JSON/XML highlighting. If XML language dependency is not installed, use plain text for now.

**Step 4: Run checks**

Run:

```bash
cd src-tauri && cargo check
npm run build
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/launchd src/services/launchd.ts src/components/PlistPanel.tsx src/App.tsx
git commit -m "feat: add launchd plist preview"
```

---

### Task 15: Polish UI States

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/App.css`
- Modify: `src/components/JobFormModal.tsx`
- Modify: `src/components/JobsTable.tsx`
- Modify: `src/components/LogsPanel.tsx`

**Step 1: Add loading/error states**

- App-level spinner while loading jobs.
- `Alert` for failed backend calls.
- Disabled action buttons while request in flight.
- Empty state with a single `Create Job` button.

**Step 2: Add keyboard-friendly behavior**

- Modal saves on primary button only.
- Escape closes modal.
- Table row click selects job.
- Buttons include icons from `@ant-design/icons`.

**Step 3: Improve dense layout**

- Keep toolbar compact.
- Ensure table actions do not wrap awkwardly.
- Keep CodeMirror panes with fixed min-height and scroll.

**Step 4: Run build**

Run:

```bash
npm run build
```

Expected: PASS.

**Step 5: Commit**

```bash
git add src/App.tsx src/App.css src/components
git commit -m "feat: polish launchd manager UI states"
```

---

### Task 16: End-to-End Manual Test

**Files:**
- No code changes unless bugs are found.

**Step 1: Run desktop app**

Run:

```bash
npm run tauri dev
```

Expected: desktop window opens.

**Step 2: Create interval shell job**

Create:

- Name: `Tick smoke interval`
- Schedule: interval, 10 seconds
- Execution: inline shell
- Script:

```sh
echo "tick smoke $(date)"
```

Expected:

- Save succeeds.
- Enable succeeds.
- After 10-20 seconds stdout log contains `tick smoke`.

**Step 3: Create calendar job with second offset**

Create:

- Name: `Tick smoke calendar`
- Schedule: current hour, next minute, second 15
- Execution: inline shell
- Script:

```sh
echo "calendar smoke $(date)"
```

Expected:

- Generated wrapper contains `sleep 15`.
- stdout log receives line around second 15.

**Step 4: Test job actions**

- Disable interval job.
- Run calendar job now.
- Clear stdout log.
- Delete both jobs.

Expected:

- `launchctl print gui/$(id -u)/<label>` fails after disable/delete.
- plist files are removed after delete.
- registry no longer shows deleted jobs.

**Step 5: Run final checks**

Run:

```bash
npm run build
cd src-tauri && cargo test
cd src-tauri && cargo check
```

Expected: PASS.

**Step 6: Commit bug fixes only**

If manual testing required fixes:

```bash
git add <changed-files>
git commit -m "fix: harden launchd manager smoke flow"
```

---

### Task 17: Documentation

**Files:**
- Modify: `README.md`

**Step 1: Add usage notes**

Document:

- This app manages user LaunchAgents only.
- Jobs are stored under `~/Library/LaunchAgents`.
- Tick-managed logs live under the app data directory.
- Calendar seconds are implemented through wrapper scripts because `launchd` calendar triggers are minute-granular.
- Interval schedules use `StartInterval`.
- Shell scripts execute with the configured interpreter and working directory.

**Step 2: Run checks**

Run:

```bash
npm run build
```

Expected: PASS.

**Step 3: Commit**

```bash
git add README.md
git commit -m "docs: document launchd manager behavior"
```

---

## Final Verification Checklist

Run:

```bash
npm run build
cd src-tauri && cargo test
cd src-tauri && cargo check
git status --short
```

Expected:

```text
npm run build exits 0
cargo test exits 0
cargo check exits 0
git status --short has no output
```

## Known Risks

- Full macOS packaging may require full Xcode, not just Command Line Tools.
- `launchd` calendar schedules do not support seconds natively; wrapper sleep is intentional.
- User shell environments in LaunchAgents are sparse. The UI should encourage absolute paths for `node`, `python3`, and script files.
- `launchctl` behavior differs slightly across macOS versions, so manual smoke testing on the target machine is required.

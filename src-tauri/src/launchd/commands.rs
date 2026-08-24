use super::launchctl;
use super::logs::{clear_log, read_log, JobLog};
use super::models::{JobStatus, LaunchdJob, LaunchdJobInput};
use super::paths::{ensure_dirs, launch_agents_dir, logs_dir, LABEL_PREFIX};
use super::plist_writer::{launch_program_args, remove_job_files, write_job_files};
use super::registry;
use super::validation::validate_job_input;
use chrono::Utc;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[tauri::command]
pub fn list_launchd_jobs() -> Result<Vec<LaunchdJob>, String> {
    registry::read_registry()?
        .into_iter()
        .map(refresh_status)
        .collect()
}

#[tauri::command]
pub fn save_launchd_job(input: LaunchdJobInput) -> Result<LaunchdJob, String> {
    validate_job_input(&input)?;
    ensure_dirs()?;

    let existing = input
        .id
        .as_deref()
        .and_then(|id| registry::find_job(id).ok());
    let id = existing
        .as_ref()
        .map(|job| job.id.clone())
        .or(input.id.clone())
        .unwrap_or_else(make_id);
    let label = existing
        .as_ref()
        .map(|job| job.label.clone())
        .unwrap_or_else(|| format!("{LABEL_PREFIX}{}", sanitize_label(&input.name, &id)));
    let plist_path = launch_agents_dir()?.join(format!("{label}.plist"));
    let status = existing
        .as_ref()
        .map(|job| job.status.clone())
        .unwrap_or(JobStatus::Disabled);

    let job = LaunchdJob {
        id: id.clone(),
        label,
        name: input.name.trim().to_string(),
        description: input.description.trim().to_string(),
        status,
        schedule: input.schedule,
        execution: input.execution,
        stdout_path: logs_dir()?
            .join(format!("{id}.stdout.log"))
            .display()
            .to_string(),
        stderr_path: logs_dir()?
            .join(format!("{id}.stderr.log"))
            .display()
            .to_string(),
        plist_path: plist_path.display().to_string(),
        last_modified_at: Utc::now().to_rfc3339(),
    };

    write_job_files(&job)?;
    let mut saved = registry::upsert_job(job)?;
    if saved.status == JobStatus::Enabled {
        let _ = launchctl::bootout(&saved.label, &PathBuf::from(&saved.plist_path));
        launchctl::bootstrap(&PathBuf::from(&saved.plist_path))?;
        saved = registry::upsert_job(mark_status(saved, JobStatus::Enabled))?;
    }
    Ok(saved)
}

#[tauri::command]
pub fn enable_launchd_job(id: String) -> Result<LaunchdJob, String> {
    let job = registry::find_job(&id)?;
    write_job_files(&job)?;
    let plist_path = PathBuf::from(&job.plist_path);
    let _ = launchctl::bootout(&job.label, &plist_path);
    launchctl::bootstrap(&plist_path)?;
    registry::upsert_job(mark_status(job, JobStatus::Enabled))
}

#[tauri::command]
pub fn disable_launchd_job(id: String) -> Result<LaunchdJob, String> {
    let job = registry::find_job(&id)?;
    let plist_path = PathBuf::from(&job.plist_path);
    let _ = launchctl::bootout(&job.label, &plist_path);
    registry::upsert_job(mark_status(job, JobStatus::Disabled))
}

#[tauri::command]
pub fn run_launchd_job_now(id: String) -> Result<(), String> {
    let job = registry::find_job(&id)?;
    let materialized = write_job_files(&job)?;
    let args = launch_program_args(&job, &materialized)?;
    let (program, rest) = args.split_first().ok_or_else(|| "命令为空".to_string())?;
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&job.stdout_path)
        .map_err(|err| err.to_string())?;
    let stderr = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&job.stderr_path)
        .map_err(|err| err.to_string())?;

    let mut command = Command::new(program);
    command
        .args(rest)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    if !job.execution.working_directory.trim().is_empty() {
        command.current_dir(job.execution.working_directory.trim());
    }
    for item in job
        .execution
        .environment
        .iter()
        .filter(|item| !item.key.trim().is_empty())
    {
        command.env(item.key.trim(), &item.value);
    }
    command.spawn().map_err(|err| err.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_launchd_job(id: String) -> Result<(), String> {
    if let Ok(job) = registry::find_job(&id) {
        let _ = launchctl::bootout(&job.label, &PathBuf::from(&job.plist_path));
        remove_job_files(&job);
    }
    registry::delete_job(&id)
}

#[tauri::command]
pub fn read_launchd_log(id: String, kind: String) -> Result<JobLog, String> {
    let job = registry::find_job(&id)?;
    let path = match kind.as_str() {
        "stdout" => PathBuf::from(job.stdout_path),
        "stderr" => PathBuf::from(job.stderr_path),
        _ => return Err("日志类型必须是 stdout 或 stderr".to_string()),
    };
    read_log(&kind, &path, 256 * 1024)
}

#[tauri::command]
pub fn clear_launchd_log(id: String, kind: String) -> Result<(), String> {
    let job = registry::find_job(&id)?;
    let path = match kind.as_str() {
        "stdout" => PathBuf::from(job.stdout_path),
        "stderr" => PathBuf::from(job.stderr_path),
        _ => return Err("日志类型必须是 stdout 或 stderr".to_string()),
    };
    clear_log(&path)
}

#[tauri::command]
pub fn read_launchd_plist(id: String) -> Result<String, String> {
    let job = registry::find_job(&id)?;
    std::fs::read_to_string(job.plist_path).map_err(|err| err.to_string())
}

fn refresh_status(mut job: LaunchdJob) -> Result<LaunchdJob, String> {
    if !PathBuf::from(&job.plist_path).exists() {
        job.status = JobStatus::Missing;
    } else if launchctl::print_job(&job.label).is_ok() {
        job.status = JobStatus::Enabled;
    } else if job.status == JobStatus::Missing {
        job.status = JobStatus::Disabled;
    }
    Ok(job)
}

fn mark_status(mut job: LaunchdJob, status: JobStatus) -> LaunchdJob {
    job.status = status;
    job.last_modified_at = Utc::now().to_rfc3339();
    job
}

fn make_id() -> String {
    format!("job-{}", Utc::now().timestamp_millis())
}

fn sanitize_label(name: &str, id: &str) -> String {
    let slug = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let short_id = id.trim_start_matches("job-");
    if slug.is_empty() {
        short_id.to_string()
    } else {
        format!("{slug}-{short_id}")
    }
}

use super::executor;
use super::logs::{clear_log, read_log, JobLog};
use super::models::{JobStatus, ScheduledJob, ScheduledJobInput, SchedulerCapabilities};
use super::paths::{ensure_dirs, normalize_managed_paths, scheduler_capabilities, LABEL_PREFIX};
use super::platform;
use super::registry;
use super::validation::validate_job_input;
use chrono::Utc;
use std::path::PathBuf;

#[tauri::command]
pub fn get_scheduler_capabilities() -> Result<SchedulerCapabilities, String> {
    scheduler_capabilities()
}

#[tauri::command]
pub fn list_scheduled_jobs() -> Result<Vec<ScheduledJob>, String> {
    let _operation_lock = registry::acquire_scheduler_operation_lock(false)?;
    Ok(registry::read_registry()?
        .into_iter()
        .map(refresh_status)
        .collect())
}

#[tauri::command]
pub fn save_scheduled_job(input: ScheduledJobInput) -> Result<ScheduledJob, String> {
    validate_job_input(&input)?;
    ensure_dirs()?;
    let _operation_lock = registry::acquire_scheduler_operation_lock(true)?;

    let existing = match input.id.as_deref() {
        Some(id) => Some(registry::find_job(id)?),
        None => None,
    };
    let id = existing
        .as_ref()
        .map(|job| job.id.clone())
        .unwrap_or_else(make_id);
    let label = existing
        .as_ref()
        .map(|job| job.label.clone())
        .unwrap_or_else(|| format!("{LABEL_PREFIX}{}", sanitize_label(&input.name, &id)));
    let actual_status = existing.as_ref().map(platform::status).transpose()?;
    let status = match actual_status.as_ref() {
        Some(JobStatus::Enabled) => JobStatus::Enabled,
        Some(JobStatus::Disabled | JobStatus::Missing) | None => JobStatus::Disabled,
        Some(JobStatus::Error) => return Err("无法确认任务的实际状态，已拒绝保存".to_string()),
    };
    let previous_for_rollback = existing.clone().map(|mut previous| {
        previous.status = status.clone();
        previous
    });

    let mut job = ScheduledJob {
        id,
        label,
        name: input.name.trim().to_string(),
        description: input.description.trim().to_string(),
        status,
        schedule: input.schedule,
        execution: input.execution,
        stdout_path: String::new(),
        stderr_path: String::new(),
        definition_path: String::new(),
        last_modified_at: Utc::now().to_rfc3339(),
    };
    normalize_managed_paths(&mut job)?;
    executor::validate_execution(&job)?;
    platform::save(&job)?;
    match registry::upsert_job(job.clone()) {
        Ok(saved) => Ok(saved),
        Err(registry_error) => {
            let rollback = match (previous_for_rollback.as_ref(), actual_status.as_ref()) {
                (Some(previous), Some(JobStatus::Enabled | JobStatus::Disabled)) => {
                    platform::save(previous)
                }
                _ => platform::delete(&job),
            };
            match rollback {
                Ok(()) => Err(format!(
                    "保存任务索引失败，调度配置已回滚：{registry_error}"
                )),
                Err(rollback_error) => Err(format!(
                    "保存任务索引失败：{registry_error}；调度配置回滚失败：{rollback_error}"
                )),
            }
        }
    }
}

#[tauri::command]
pub fn enable_scheduled_job(id: String) -> Result<ScheduledJob, String> {
    let _operation_lock = registry::acquire_scheduler_operation_lock(true)?;
    let job = registry::find_job(&id)?;
    let previous_status = platform::status(&job)?;
    platform::enable(&job)?;
    persist_status_with_rollback(job, JobStatus::Enabled, previous_status)
}

#[tauri::command]
pub fn disable_scheduled_job(id: String) -> Result<ScheduledJob, String> {
    let _operation_lock = registry::acquire_scheduler_operation_lock(true)?;
    let job = registry::find_job(&id)?;
    let previous_status = platform::status(&job)?;
    platform::disable(&job)?;
    persist_status_with_rollback(job, JobStatus::Disabled, previous_status)
}

#[tauri::command]
pub fn run_scheduled_job_now(id: String) -> Result<(), String> {
    let _operation_lock = registry::acquire_scheduler_operation_lock(true)?;
    let job = registry::find_job(&id)?;
    platform::run_now(&job)
}

#[tauri::command]
pub fn delete_scheduled_job(id: String) -> Result<(), String> {
    let _operation_lock = registry::acquire_scheduler_operation_lock(true)?;
    let job = registry::find_job(&id)?;
    platform::delete(&job)?;
    executor::remove_materialized_execution(&job)?;
    executor::remove_logs(&job)?;
    registry::delete_job(&id)
}

#[tauri::command]
pub fn read_scheduled_job_log(id: String, kind: String) -> Result<JobLog, String> {
    let _operation_lock = registry::acquire_scheduler_operation_lock(false)?;
    let job = registry::find_job(&id)?;
    let path = log_path(&job, &kind)?;
    read_log(&kind, &path, 256 * 1024)
}

#[tauri::command]
pub fn clear_scheduled_job_log(id: String, kind: String) -> Result<(), String> {
    let _operation_lock = registry::acquire_scheduler_operation_lock(true)?;
    let job = registry::find_job(&id)?;
    clear_log(&log_path(&job, &kind)?)
}

#[tauri::command]
pub fn read_job_definition(id: String) -> Result<String, String> {
    let _operation_lock = registry::acquire_scheduler_operation_lock(false)?;
    let job = registry::find_job(&id)?;
    platform::read_definition(&job)
}

pub fn run_scheduled_job_runner(id: &str) -> Result<i32, String> {
    let job = {
        let _operation_lock = registry::acquire_scheduler_operation_lock(false)?;
        registry::find_job(id)?
    };
    executor::run_and_wait(&job)
}

fn log_path(job: &ScheduledJob, kind: &str) -> Result<PathBuf, String> {
    match kind {
        "stdout" => Ok(PathBuf::from(&job.stdout_path)),
        "stderr" => Ok(PathBuf::from(&job.stderr_path)),
        _ => Err("日志类型必须是 stdout 或 stderr".to_string()),
    }
}

fn refresh_status(mut job: ScheduledJob) -> ScheduledJob {
    job.status = platform::status(&job).unwrap_or(JobStatus::Error);
    job
}

fn mark_status(mut job: ScheduledJob, status: JobStatus) -> ScheduledJob {
    job.status = status;
    job.last_modified_at = Utc::now().to_rfc3339();
    job
}

fn persist_status_with_rollback(
    job: ScheduledJob,
    status: JobStatus,
    previous_status: JobStatus,
) -> Result<ScheduledJob, String> {
    match registry::upsert_job(mark_status(job.clone(), status)) {
        Ok(saved) => Ok(saved),
        Err(registry_error) => {
            let rollback = match previous_status {
                JobStatus::Enabled => platform::enable(&job),
                JobStatus::Disabled => platform::disable(&job),
                JobStatus::Missing => platform::delete(&job),
                JobStatus::Error => Err("原任务状态异常，无法回滚".to_string()),
            };
            match rollback {
                Ok(()) => Err(format!(
                    "保存任务状态失败，调度状态已回滚：{registry_error}"
                )),
                Err(rollback_error) => Err(format!(
                    "保存任务状态失败：{registry_error}；调度状态回滚失败：{rollback_error}"
                )),
            }
        }
    }
}

fn make_id() -> String {
    const DECIMAL_SPACE: u128 = 100_000_000_000_000_000_000;
    let value = uuid::Uuid::new_v4().as_u128() % DECIMAL_SPACE;
    format!("job-{value:020}")
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
    let maximum_slug_length = 180_usize.saturating_sub(short_id.len() + 1);
    let slug = slug.chars().take(maximum_slug_length).collect::<String>();
    if slug.is_empty() {
        short_id.to_string()
    } else {
        format!("{slug}-{short_id}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_is_ascii_and_keeps_the_generated_identity_suffix() {
        assert_eq!(
            sanitize_label("  每日 Sync / 安全  ", "job-1234567890"),
            "sync-1234567890"
        );
        assert_eq!(sanitize_label("中文", "job-1234567890"), "1234567890");
        let label = sanitize_label(&"a".repeat(200), "job-12345678901234567890");
        assert_eq!(label.len(), 180);
        assert!(label.ends_with("-12345678901234567890"));
    }

    #[test]
    fn generated_ids_fit_the_tick_identity_contract() {
        let id = make_id();
        assert_eq!(id.len(), 24);
        assert!(crate::scheduler::paths::validate_job_id(&id).is_ok());
    }
}

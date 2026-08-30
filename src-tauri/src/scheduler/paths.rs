use super::models::{ScheduledJob, SchedulerCapabilities};
use std::path::PathBuf;

pub const ID_PREFIX: &str = "job-";
pub const LABEL_PREFIX: &str = "com.gavin.tick.";
const MAX_LABEL_BODY_BYTES: usize = 234;
#[cfg(any(target_os = "windows", test))]
pub const TASK_NAME_PREFIX: &str = "Tick.";
#[cfg(any(target_os = "windows", test))]
pub const TASK_SOURCE: &str = "com.gavin.tick";

pub fn home_dir() -> Result<PathBuf, String> {
    dirs::home_dir().ok_or_else(|| "无法定位用户主目录".to_string())
}

#[cfg(target_os = "macos")]
pub fn launch_agents_dir() -> Result<PathBuf, String> {
    Ok(home_dir()?.join("Library").join("LaunchAgents"))
}

pub fn app_data_dir() -> Result<PathBuf, String> {
    dirs::data_dir()
        .map(|dir| dir.join("tick"))
        .ok_or_else(|| "无法定位应用数据目录".to_string())
}

pub fn registry_path() -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    let filename = "launchd-jobs.json";
    #[cfg(target_os = "windows")]
    let filename = "scheduled-jobs.json";
    Ok(app_data_dir()?.join(filename))
}

pub fn registry_lock_path() -> Result<PathBuf, String> {
    Ok(registry_path()?.with_extension("lock"))
}

pub fn scheduler_operation_lock_path() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("scheduler-operation.lock"))
}

pub fn scripts_dir() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("scripts"))
}

pub fn logs_dir() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("logs"))
}

#[cfg(target_os = "macos")]
pub fn wrappers_dir() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("wrappers"))
}

#[cfg(target_os = "macos")]
pub fn definition_path(id: &str, label: &str) -> Result<PathBuf, String> {
    validate_job_identity(id, label)?;
    Ok(launch_agents_dir()?.join(format!("{label}.plist")))
}

pub fn definition_location(id: &str, label: &str) -> Result<String, String> {
    validate_job_identity(id, label)?;
    #[cfg(target_os = "macos")]
    {
        Ok(definition_path(id, label)?.display().to_string())
    }
    #[cfg(target_os = "windows")]
    {
        Ok(format!(r"\{}", task_name(id)?))
    }
}

pub fn inline_script_path(id: &str) -> Result<PathBuf, String> {
    validate_job_id(id)?;
    Ok(scripts_dir()?.join(format!("{id}.js")))
}

pub fn stdout_path(id: &str) -> Result<PathBuf, String> {
    validate_job_id(id)?;
    Ok(logs_dir()?.join(format!("{id}.stdout.log")))
}

pub fn stderr_path(id: &str) -> Result<PathBuf, String> {
    validate_job_id(id)?;
    Ok(logs_dir()?.join(format!("{id}.stderr.log")))
}

#[cfg(target_os = "macos")]
pub fn wrapper_path(id: &str) -> Result<PathBuf, String> {
    validate_job_id(id)?;
    Ok(wrappers_dir()?.join(format!("{id}.sh")))
}

#[cfg(any(target_os = "windows", test))]
pub fn task_name(id: &str) -> Result<String, String> {
    validate_job_id(id)?;
    Ok(format!("{TASK_NAME_PREFIX}{id}"))
}

#[cfg(any(target_os = "windows", test))]
pub fn task_uri(id: &str) -> Result<String, String> {
    validate_job_id(id)?;
    Ok(format!("tick://{TASK_SOURCE}/{id}"))
}

pub fn ensure_dirs() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let dirs = vec![
        app_data_dir()?,
        scripts_dir()?,
        logs_dir()?,
        launch_agents_dir()?,
        wrappers_dir()?,
    ];
    #[cfg(target_os = "windows")]
    let dirs = vec![app_data_dir()?, scripts_dir()?, logs_dir()?];
    for dir in dirs {
        std::fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    }
    Ok(())
}

pub fn validate_job_id(id: &str) -> Result<&str, String> {
    let suffix = id
        .strip_prefix(ID_PREFIX)
        .ok_or_else(|| "任务标识不属于 Tick".to_string())?;
    if suffix.is_empty() || suffix.len() > 20 || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("任务标识格式无效".to_string());
    }
    Ok(suffix)
}

pub fn validate_job_identity(id: &str, label: &str) -> Result<(), String> {
    let suffix = validate_job_id(id)?;
    let body = label
        .strip_prefix(LABEL_PREFIX)
        .ok_or_else(|| "任务标签不属于 Tick".to_string())?;
    if body.is_empty()
        || body.len() > MAX_LABEL_BODY_BYTES
        || !body
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        || !(body == suffix || body.ends_with(&format!("-{suffix}")))
    {
        return Err("任务标签格式无效".to_string());
    }
    Ok(())
}

pub fn normalize_managed_paths(job: &mut ScheduledJob) -> Result<(), String> {
    validate_job_identity(&job.id, &job.label)?;
    job.stdout_path = stdout_path(&job.id)?.display().to_string();
    job.stderr_path = stderr_path(&job.id)?.display().to_string();
    job.definition_path = definition_location(&job.id, &job.label)?;
    Ok(())
}

pub fn scheduler_capabilities() -> Result<SchedulerCapabilities, String> {
    let home_directory = home_dir()?.display().to_string();
    #[cfg(target_os = "macos")]
    {
        Ok(SchedulerCapabilities {
            platform: "macos",
            computer_label: "Mac",
            scheduler_name: "LaunchAgent",
            definition_label: "plist",
            default_interpreter: "/usr/bin/env node",
            script_path_example: "/Users/you/scripts/job.js",
            working_directory_example: "/Users/you/project",
            home_directory,
            trash_label: "废纸篓",
            minimum_interval_seconds: 1,
            maximum_interval_seconds: None,
        })
    }
    #[cfg(target_os = "windows")]
    {
        Ok(SchedulerCapabilities {
            platform: "windows",
            computer_label: "Windows 电脑",
            scheduler_name: "Task Scheduler",
            definition_label: "XML",
            default_interpreter: "node.exe",
            script_path_example: r"C:\Users\you\scripts\job.js",
            working_directory_example: r"C:\Users\you\project",
            home_directory,
            trash_label: "回收站",
            minimum_interval_seconds: 60,
            maximum_interval_seconds: Some(31 * 24 * 60 * 60),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::models::{
        CalendarSchedule, ExecutionMode, IntervalSchedule, JobExecution, JobSchedule, JobStatus,
        ScheduleMode,
    };

    fn job() -> ScheduledJob {
        ScheduledJob {
            id: "job-1234567890".to_string(),
            label: "com.gavin.tick.safe-job-1234567890".to_string(),
            name: "Safe".to_string(),
            description: String::new(),
            status: JobStatus::Disabled,
            schedule: JobSchedule {
                mode: ScheduleMode::Interval,
                calendar: CalendarSchedule {
                    month: None,
                    day: None,
                    hour: Some(9),
                    minute: Some(0),
                    second: 0,
                },
                interval: IntervalSchedule { seconds: 60 },
            },
            execution: JobExecution {
                mode: ExecutionMode::InlineShell,
                inline_script: "console.log('ok')".to_string(),
                script_path: String::new(),
                interpreter: "/usr/bin/env node".to_string(),
                arguments: String::new(),
                working_directory: String::new(),
                environment: vec![],
            },
            stdout_path: "/tmp/forged.stdout".to_string(),
            stderr_path: "/tmp/forged.stderr".to_string(),
            definition_path: "/tmp/forged-definition".to_string(),
            last_modified_at: "2026-08-30T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn rejects_unowned_and_path_like_ids() {
        for id in [
            "other-123",
            "job-../x",
            "job-1/2",
            r"job-1\2",
            "job-\0",
            "job-",
        ] {
            assert!(validate_job_id(id).is_err(), "accepted {id:?}");
        }
        assert!(validate_job_id("job-123456789012345678901").is_err());
        assert_eq!(task_name("job-123").unwrap(), "Tick.job-123");
    }

    #[test]
    fn rejects_forged_labels() {
        assert!(validate_job_identity("job-123", "com.apple.fake-123").is_err());
        assert!(validate_job_identity("job-123", "com.gavin.tick.other-456").is_err());
        assert!(validate_job_identity("job-123", "com.gavin.tick...-123").is_err());
    }

    #[test]
    fn replaces_persisted_paths_with_tick_owned_paths() {
        let mut value = job();
        normalize_managed_paths(&mut value).unwrap();
        assert!(value.stdout_path.ends_with("job-1234567890.stdout.log"));
        assert!(value.stderr_path.ends_with("job-1234567890.stderr.log"));
        assert!(!value.definition_path.contains("forged"));
    }
}

use super::executor::{command_args, materialize_execution};
use super::models::{ScheduleMode, ScheduledJob};
use super::paths::{definition_path, ensure_dirs, wrapper_path};
use plist::{Dictionary, Value};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

pub fn write_job_files(job: &ScheduledJob) -> Result<(), String> {
    ensure_dirs()?;
    let execution = materialize_execution(job)?;
    let base_args = command_args(job, &execution)?;
    let wrapper_path = write_wrapper_if_needed(job, &base_args)?;
    let program_args = if let Some(wrapper) = &wrapper_path {
        vec![wrapper.display().to_string()]
    } else {
        base_args
    };

    for path in [&job.stdout_path, &job.stderr_path] {
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|err| err.to_string())?;
    }

    let path = definition_path(&job.id, &job.label)?;
    let plist = build_plist(job, program_args);
    let mut file = fs::File::create(path).map_err(|err| err.to_string())?;
    Value::Dictionary(plist)
        .to_writer_xml(&mut file)
        .map_err(|err| err.to_string())?;

    Ok(())
}

fn write_wrapper_if_needed(
    job: &ScheduledJob,
    command_args: &[String],
) -> Result<Option<PathBuf>, String> {
    if job.schedule.mode != ScheduleMode::Calendar || job.schedule.calendar.second == 0 {
        return Ok(None);
    }
    let path = wrapper_path(&job.id)?;
    let command = command_args
        .iter()
        .map(|arg| shell_words::quote(arg).into_owned())
        .collect::<Vec<_>>()
        .join(" ");
    let content = wrapper_contents(job.schedule.calendar.second, &command);
    fs::write(&path, content).map_err(|err| err.to_string())?;
    make_executable(&path)?;
    Ok(Some(path))
}

fn wrapper_contents(second: u8, command: &str) -> String {
    format!("#!/bin/sh\nsleep {second}\nexec {command}\n")
}

fn build_plist(job: &ScheduledJob, program_args: Vec<String>) -> Dictionary {
    let mut dict = Dictionary::new();
    dict.insert("Label".into(), Value::String(job.label.clone()));
    dict.insert(
        "ProgramArguments".into(),
        Value::Array(program_args.into_iter().map(Value::String).collect()),
    );
    dict.insert("RunAtLoad".into(), Value::Boolean(false));
    dict.insert(
        "StandardOutPath".into(),
        Value::String(job.stdout_path.clone()),
    );
    dict.insert(
        "StandardErrorPath".into(),
        Value::String(job.stderr_path.clone()),
    );

    if !job.execution.working_directory.trim().is_empty() {
        dict.insert(
            "WorkingDirectory".into(),
            Value::String(job.execution.working_directory.trim().to_string()),
        );
    }

    let environment = job
        .execution
        .environment
        .iter()
        .filter(|item| !item.key.trim().is_empty())
        .map(|item| {
            (
                item.key.trim().to_string(),
                Value::String(item.value.clone()),
            )
        })
        .collect::<Dictionary>();
    if !environment.is_empty() {
        dict.insert(
            "EnvironmentVariables".into(),
            Value::Dictionary(environment),
        );
    }

    match job.schedule.mode {
        ScheduleMode::Calendar => {
            let mut schedule = Dictionary::new();
            if let Some(month) = job.schedule.calendar.month {
                schedule.insert("Month".into(), Value::Integer(month.into()));
            }
            if let Some(day) = job.schedule.calendar.day {
                schedule.insert("Day".into(), Value::Integer(day.into()));
            }
            if let Some(hour) = job.schedule.calendar.hour {
                schedule.insert("Hour".into(), Value::Integer(hour.into()));
            }
            if let Some(minute) = job.schedule.calendar.minute {
                schedule.insert("Minute".into(), Value::Integer(minute.into()));
            }
            dict.insert("StartCalendarInterval".into(), Value::Dictionary(schedule));
        }
        ScheduleMode::Interval => {
            dict.insert(
                "StartInterval".into(),
                Value::Integer(job.schedule.interval.seconds.into()),
            );
        }
    }

    dict
}

fn make_executable(path: &PathBuf) -> Result<(), String> {
    let mut permissions = fs::metadata(path)
        .map_err(|err| err.to_string())?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).map_err(|err| err.to_string())
}

pub fn remove_job_files(job: &ScheduledJob) -> Result<(), String> {
    remove_file_if_exists(&definition_path(&job.id, &job.label)?)?;
    remove_file_if_exists(&wrapper_path(&job.id)?)
}

fn remove_file_if_exists(path: &std::path::Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("无法删除 {}：{err}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::models::{
        CalendarSchedule, EnvironmentVar, ExecutionMode, IntervalSchedule, JobExecution,
        JobSchedule, JobStatus, ScheduleMode, ScheduledJob,
    };

    fn job(mode: ScheduleMode, second: u8, interval: u32) -> ScheduledJob {
        ScheduledJob {
            id: "job-1234567890".to_string(),
            label: "com.gavin.tick.test-job-1234567890".to_string(),
            name: "Test".to_string(),
            description: "".to_string(),
            status: JobStatus::Disabled,
            schedule: JobSchedule {
                mode,
                calendar: CalendarSchedule {
                    month: None,
                    day: None,
                    hour: Some(9),
                    minute: Some(30),
                    second,
                },
                interval: IntervalSchedule { seconds: interval },
            },
            execution: JobExecution {
                mode: ExecutionMode::InlineShell,
                inline_script: "echo ok".to_string(),
                script_path: "".to_string(),
                interpreter: "/bin/sh".to_string(),
                arguments: "".to_string(),
                working_directory: "".to_string(),
                environment: vec![EnvironmentVar {
                    key: "FOO".to_string(),
                    value: "bar".to_string(),
                }],
            },
            stdout_path: "/tmp/tick-test.out.log".to_string(),
            stderr_path: "/tmp/tick-test.err.log".to_string(),
            definition_path: "/tmp/com.gavin.tick.test-job.plist".to_string(),
            last_modified_at: "2026-05-06T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn calendar_schedule_uses_start_calendar_interval() {
        let plist = build_plist(
            &job(ScheduleMode::Calendar, 0, 30),
            vec!["/bin/sh".to_string()],
        );
        assert!(plist.contains_key("StartCalendarInterval"));
        assert!(!plist.contains_key("StartInterval"));
    }

    #[test]
    fn interval_schedule_uses_start_interval() {
        let plist = build_plist(
            &job(ScheduleMode::Interval, 0, 30),
            vec!["/bin/sh".to_string()],
        );
        assert_eq!(plist.get("StartInterval"), Some(&Value::Integer(30.into())));
    }

    #[test]
    fn quotes_calendar_second_wrapper_arguments() {
        let args = ["/bin/sh".to_string(), "/tmp/a b.sh".to_string()];
        let command = args
            .iter()
            .map(|arg| shell_words::quote(arg).into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        let content = wrapper_contents(5, &command);
        assert!(content.contains("sleep 5"));
        assert!(content.contains("'/tmp/a b.sh'"));
    }
}

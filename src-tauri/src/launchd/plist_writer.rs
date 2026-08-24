use super::models::{ExecutionMode, LaunchdJob, ScheduleMode};
use super::paths::{ensure_dirs, launch_agents_dir, scripts_dir, wrappers_dir};
use plist::{Dictionary, Value};
use std::fs;
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub struct MaterializedJob {
    pub wrapper_path: Option<PathBuf>,
    pub inline_script_path: Option<PathBuf>,
}

pub fn write_job_files(job: &LaunchdJob) -> Result<MaterializedJob, String> {
    ensure_dirs()?;
    let inline_script_path = write_inline_script(job)?;
    let base_args = command_args(job, inline_script_path.as_ref())?;
    let wrapper_path = write_wrapper_if_needed(job, &base_args)?;
    let program_args = if let Some(wrapper) = &wrapper_path {
        vec![wrapper.display().to_string()]
    } else {
        base_args
    };

    let plist_path = launch_agents_dir()?.join(format!("{}.plist", job.label));
    let stdout_path = PathBuf::from(&job.stdout_path);
    let stderr_path = PathBuf::from(&job.stderr_path);

    if let Some(parent) = stdout_path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&stdout_path)
        .map_err(|err| err.to_string())?;
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&stderr_path)
        .map_err(|err| err.to_string())?;

    let plist = build_plist(job, program_args);
    let mut file = fs::File::create(&plist_path).map_err(|err| err.to_string())?;
    Value::Dictionary(plist)
        .to_writer_xml(&mut file)
        .map_err(|err| err.to_string())?;

    Ok(MaterializedJob {
        wrapper_path,
        inline_script_path,
    })
}

pub fn launch_program_args(
    job: &LaunchdJob,
    materialized: &MaterializedJob,
) -> Result<Vec<String>, String> {
    if let Some(wrapper_path) = &materialized.wrapper_path {
        Ok(vec![wrapper_path.display().to_string()])
    } else {
        command_args(job, materialized.inline_script_path.as_ref())
    }
}

fn write_inline_script(job: &LaunchdJob) -> Result<Option<PathBuf>, String> {
    if job.execution.mode != ExecutionMode::InlineShell {
        return Ok(None);
    }
    let extension = if job.execution.interpreter.contains("node") {
        "js"
    } else {
        "sh"
    };
    let path = scripts_dir()?.join(format!("{}.{}", job.id, extension));
    fs::write(&path, normalize_script(job)).map_err(|err| err.to_string())?;
    make_executable(&path)?;
    Ok(Some(path))
}

fn normalize_script(job: &LaunchdJob) -> String {
    let script = &job.execution.inline_script;
    if script.starts_with("#!") {
        format!("{script}\n")
    } else if job.execution.interpreter.contains("node") {
        format!("#!/usr/bin/env node\n{script}\n")
    } else {
        format!("#!/bin/sh\n{script}\n")
    }
}

fn command_args(
    job: &LaunchdJob,
    inline_script_path: Option<&PathBuf>,
) -> Result<Vec<String>, String> {
    let extra_args = parse_arguments(&job.execution.arguments)?;
    match job.execution.mode {
        ExecutionMode::InlineShell => {
            let script = inline_script_path.ok_or_else(|| "缺少内联脚本路径".to_string())?;
            let interpreter_parts = if job.execution.interpreter.trim().is_empty() {
                vec!["/bin/sh".to_string()]
            } else {
                parse_arguments(&job.execution.interpreter)?
            };
            let mut args = interpreter_parts;
            args.push(script.display().to_string());
            args.extend(extra_args);
            Ok(args)
        }
        ExecutionMode::ScriptPath => {
            let mut args = vec![job.execution.script_path.trim().to_string()];
            args.extend(extra_args);
            Ok(args)
        }
        ExecutionMode::Interpreter => {
            let mut args = vec![job.execution.interpreter.trim().to_string()];
            if !job.execution.script_path.trim().is_empty() {
                args.push(job.execution.script_path.trim().to_string());
            }
            args.extend(extra_args);
            Ok(args)
        }
    }
}

fn parse_arguments(arguments: &str) -> Result<Vec<String>, String> {
    if arguments.trim().is_empty() {
        Ok(vec![])
    } else {
        shell_words::split(arguments).map_err(|err| err.to_string())
    }
}

fn write_wrapper_if_needed(
    job: &LaunchdJob,
    command_args: &[String],
) -> Result<Option<PathBuf>, String> {
    if job.schedule.mode != ScheduleMode::Calendar || job.schedule.calendar.second == 0 {
        return Ok(None);
    }
    let path = wrappers_dir()?.join(format!("{}.sh", job.id));
    let command = command_args
        .iter()
        .map(|arg| shell_words::quote(arg).into_owned())
        .collect::<Vec<_>>()
        .join(" ");
    let content = format!(
        "#!/bin/sh\nsleep {}\nexec {}\n",
        job.schedule.calendar.second, command
    );
    fs::write(&path, content).map_err(|err| err.to_string())?;
    make_executable(&path)?;
    Ok(Some(path))
}

fn build_plist(job: &LaunchdJob, program_args: Vec<String>) -> Dictionary {
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
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(path)
            .map_err(|err| err.to_string())?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).map_err(|err| err.to_string())?;
    }
    Ok(())
}

pub fn remove_job_files(job: &LaunchdJob) {
    let _ = fs::remove_file(&job.plist_path);
    let _ = fs::remove_file(
        scripts_dir()
            .map(|dir| dir.join(format!("{}.sh", job.id)))
            .unwrap_or_default(),
    );
    let _ = fs::remove_file(
        scripts_dir()
            .map(|dir| dir.join(format!("{}.js", job.id)))
            .unwrap_or_default(),
    );
    let _ = fs::remove_file(
        wrappers_dir()
            .map(|dir| dir.join(format!("{}.sh", job.id)))
            .unwrap_or_default(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launchd::models::{
        CalendarSchedule, EnvironmentVar, IntervalSchedule, JobStatus, LaunchdExecution,
        LaunchdSchedule,
    };

    fn job(mode: ScheduleMode, second: u8, interval: u32) -> LaunchdJob {
        LaunchdJob {
            id: "test-job".to_string(),
            label: "com.gavin.tick.test-job".to_string(),
            name: "Test".to_string(),
            description: "".to_string(),
            status: JobStatus::Disabled,
            schedule: LaunchdSchedule {
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
            execution: LaunchdExecution {
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
            plist_path: "/tmp/com.gavin.tick.test-job.plist".to_string(),
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
    fn parses_quoted_arguments() {
        assert_eq!(
            parse_arguments("--name \"hello world\"").unwrap(),
            vec!["--name".to_string(), "hello world".to_string()]
        );
    }

    #[test]
    fn inline_interpreter_can_include_arguments() {
        let mut job = job(ScheduleMode::Calendar, 0, 30);
        job.execution.interpreter = "/usr/bin/env node".to_string();
        let script = PathBuf::from("/tmp/tick-inline.js");

        assert_eq!(
            command_args(&job, Some(&script)).unwrap(),
            vec![
                "/usr/bin/env".to_string(),
                "node".to_string(),
                "/tmp/tick-inline.js".to_string()
            ]
        );
    }
}

use super::models::{ExecutionMode, ScheduleMode, ScheduledJobInput};
use super::paths::validate_job_id;

const MAX_NAME_CHARS: usize = 200;
const MAX_DESCRIPTION_CHARS: usize = 2_000;

pub fn validate_job_input(input: &ScheduledJobInput) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let interval_limits = (60, Some(31 * 24 * 60 * 60));
    #[cfg(not(target_os = "windows"))]
    let interval_limits = (1, None);
    validate_job_input_with_interval_limits(input, interval_limits.0, interval_limits.1)
}

fn validate_job_input_with_interval_limits(
    input: &ScheduledJobInput,
    minimum_interval_seconds: u32,
    maximum_interval_seconds: Option<u32>,
) -> Result<(), String> {
    if let Some(id) = input.id.as_deref() {
        validate_job_id(id)?;
    }
    if input.name.trim().is_empty() {
        return Err("请输入名称".to_string());
    }
    validate_xml_text("名称", &input.name, MAX_NAME_CHARS)?;
    validate_xml_text("描述", &input.description, MAX_DESCRIPTION_CHARS)?;

    match input.schedule.mode {
        ScheduleMode::Calendar => {
            if let Some(month) = input.schedule.calendar.month {
                if !(1..=12).contains(&month) {
                    return Err("月份必须在 1 到 12 之间".to_string());
                }
            }
            if let Some(day) = input.schedule.calendar.day {
                if !(1..=31).contains(&day) {
                    return Err("日期必须在 1 到 31 之间".to_string());
                }
            }
            if let Some(hour) = input.schedule.calendar.hour {
                if hour > 23 {
                    return Err("小时必须在 0 到 23 之间".to_string());
                }
            }
            if let Some(minute) = input.schedule.calendar.minute {
                if minute > 59 {
                    return Err("分钟必须在 0 到 59 之间".to_string());
                }
            }
            if input.schedule.calendar.second > 59 {
                return Err("秒必须在 0 到 59 之间".to_string());
            }
        }
        ScheduleMode::Interval => {
            if input.schedule.interval.seconds < minimum_interval_seconds {
                return Err(format!("间隔至少 {minimum_interval_seconds} 秒"));
            }
            if let Some(maximum) = maximum_interval_seconds {
                if input.schedule.interval.seconds > maximum {
                    return Err(format!("间隔不能超过 {maximum} 秒"));
                }
            }
        }
    }

    if input.execution.inline_script.contains('\0') {
        return Err("命令和路径不能包含空字符".to_string());
    }
    for value in [
        input.execution.script_path.as_str(),
        input.execution.interpreter.as_str(),
        input.execution.arguments.as_str(),
        input.execution.working_directory.as_str(),
    ] {
        validate_xml_characters("命令和路径", value)?;
    }

    for item in &input.execution.environment {
        let key = item.key.trim();
        if key.is_empty() {
            continue;
        }
        if key.contains('=') || key.contains('\0') {
            return Err("环境变量名称不能包含 = 或空字符".to_string());
        }
        validate_xml_characters("环境变量名称", key)?;
        validate_xml_characters("环境变量值", &item.value)?;
    }

    match input.execution.mode {
        ExecutionMode::InlineShell if input.execution.inline_script.trim().is_empty() => {
            Err("请输入脚本内容".to_string())
        }
        ExecutionMode::ScriptPath if input.execution.script_path.trim().is_empty() => {
            Err("请输入脚本路径".to_string())
        }
        ExecutionMode::Interpreter if input.execution.interpreter.trim().is_empty() => {
            Err("请输入解释器路径".to_string())
        }
        _ => Ok(()),
    }
}

fn validate_xml_text(label: &str, value: &str, maximum_chars: usize) -> Result<(), String> {
    if value.chars().count() > maximum_chars {
        return Err(format!("{label}不能超过 {maximum_chars} 个字符"));
    }
    validate_xml_characters(label, value)
}

fn validate_xml_characters(label: &str, value: &str) -> Result<(), String> {
    if value.contains('\0') {
        return Err(format!("{label}不能包含空字符"));
    }
    if value.chars().any(|character| {
        !matches!(
            character,
            '\u{9}' | '\u{A}' | '\u{D}' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}'
        )
    }) {
        return Err(format!("{label}不能包含 XML 不支持的控制字符"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::models::{CalendarSchedule, IntervalSchedule, JobExecution, JobSchedule};

    fn valid_input() -> ScheduledJobInput {
        ScheduledJobInput {
            id: None,
            name: "Nightly sync".to_string(),
            description: "".to_string(),
            schedule: JobSchedule {
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
            execution: JobExecution {
                mode: ExecutionMode::InlineShell,
                inline_script: "echo ok".to_string(),
                script_path: "".to_string(),
                interpreter: "/usr/bin/env node".to_string(),
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
        assert_eq!(validate_job_input(&input), Err("请输入名称".to_string()));
    }

    #[test]
    fn validates_xml_text_and_description_length() {
        let mut input = valid_input();
        input.description = "line one\n\tline two\r".to_string();
        assert!(validate_job_input(&input).is_ok());

        input.description = "bad\u{1}description".to_string();
        assert_eq!(
            validate_job_input(&input),
            Err("描述不能包含 XML 不支持的控制字符".to_string())
        );

        input.description = "x".repeat(MAX_DESCRIPTION_CHARS + 1);
        assert_eq!(
            validate_job_input(&input),
            Err(format!("描述不能超过 {MAX_DESCRIPTION_CHARS} 个字符"))
        );

        input.description.clear();
        input.name = "bad\0name".to_string();
        assert_eq!(
            validate_job_input(&input),
            Err("名称不能包含空字符".to_string())
        );
    }

    #[test]
    fn rejects_invalid_calendar_second() {
        let mut input = valid_input();
        input.schedule.calendar.second = 60;
        assert_eq!(
            validate_job_input(&input),
            Err("秒必须在 0 到 59 之间".to_string())
        );
    }

    #[test]
    fn rejects_invalid_interval() {
        let mut input = valid_input();
        input.schedule.mode = ScheduleMode::Interval;
        input.schedule.interval.seconds = 0;
        let minimum = if cfg!(target_os = "windows") { 60 } else { 1 };
        assert_eq!(
            validate_job_input(&input),
            Err(format!("间隔至少 {minimum} 秒"))
        );
    }

    #[test]
    fn rejects_empty_inline_script() {
        let mut input = valid_input();
        input.execution.inline_script = " ".to_string();
        assert_eq!(
            validate_job_input(&input),
            Err("请输入脚本内容".to_string())
        );
    }

    #[test]
    fn applies_windows_task_scheduler_interval_bounds() {
        let mut input = valid_input();
        input.schedule.mode = ScheduleMode::Interval;
        input.schedule.interval.seconds = 59;
        assert_eq!(
            validate_job_input_with_interval_limits(&input, 60, Some(2_678_400)),
            Err("间隔至少 60 秒".to_string())
        );
        input.schedule.interval.seconds = 60;
        assert!(validate_job_input_with_interval_limits(&input, 60, Some(2_678_400)).is_ok());
        input.schedule.interval.seconds = 2_678_401;
        assert_eq!(
            validate_job_input_with_interval_limits(&input, 60, Some(2_678_400)),
            Err("间隔不能超过 2678400 秒".to_string())
        );
    }

    #[test]
    fn rejects_invalid_environment_names_and_nul_values() {
        let mut input = valid_input();
        input
            .execution
            .environment
            .push(crate::scheduler::models::EnvironmentVar {
                key: "BAD=NAME".to_string(),
                value: "value".to_string(),
            });
        assert_eq!(
            validate_job_input(&input),
            Err("环境变量名称不能包含 = 或空字符".to_string())
        );

        let mut input = valid_input();
        input.execution.working_directory = "bad\0path".to_string();
        assert_eq!(
            validate_job_input(&input),
            Err("命令和路径不能包含空字符".to_string())
        );
    }
}

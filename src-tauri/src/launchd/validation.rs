use super::models::{ExecutionMode, LaunchdJobInput, ScheduleMode};

pub fn validate_job_input(input: &LaunchdJobInput) -> Result<(), String> {
    if input.name.trim().is_empty() {
        return Err("请输入名称".to_string());
    }

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
            if input.schedule.interval.seconds == 0 {
                return Err("间隔至少 1 秒".to_string());
            }
        }
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
        assert_eq!(validate_job_input(&input), Err("间隔至少 1 秒".to_string()));
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
}

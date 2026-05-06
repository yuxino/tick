use super::models::{ExecutionMode, LaunchdJobInput, ScheduleMode};

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
        assert_eq!(
            validate_job_input(&input),
            Err("Name is required".to_string())
        );
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

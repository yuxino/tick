use super::models::{JobStatus, ScheduleMode, ScheduledJob};
use super::paths::{task_uri, TASK_SOURCE};
use std::path::Path;

const ALL_MONTHS: &str = "<January/><February/><March/><April/><May/><June/><July/><August/><September/><October/><November/><December/>";

pub fn build_task_xml(
    job: &ScheduledJob,
    tick_executable: &Path,
    start_boundary: &str,
) -> Result<String, String> {
    let executable = tick_executable
        .to_str()
        .ok_or_else(|| "Tick 可执行文件路径不是有效 Unicode".to_string())?;
    let working_directory = executable_working_directory(tick_executable)?;
    let uri = task_uri(&job.id)?;
    let trigger = build_trigger(job, start_boundary)?;
    let enabled = if job.status == JobStatus::Enabled {
        "true"
    } else {
        "false"
    };

    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Task version="1.4" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Author>Tick</Author>
    <Description>{description}</Description>
    <URI>{uri}</URI>
    <Source>{source}</Source>
  </RegistrationInfo>
  <Triggers>
{trigger}
  </Triggers>
  <Principals>
    <Principal id="Author">
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>false</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <IdleSettings>
      <StopOnIdleEnd>false</StopOnIdleEnd>
      <RestartOnIdle>false</RestartOnIdle>
    </IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>{enabled}</Enabled>
    <Hidden>false</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>7</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{executable}</Command>
      <Arguments>--run-scheduled-job {id}</Arguments>
      <WorkingDirectory>{working_directory}</WorkingDirectory>
    </Exec>
  </Actions>
</Task>
"#,
        description = xml_escape(&job.description),
        uri = xml_escape(&uri),
        source = TASK_SOURCE,
        executable = xml_escape(executable),
        id = job.id,
        working_directory = xml_escape(&working_directory),
    ))
}

#[cfg(target_os = "windows")]
fn executable_working_directory(tick_executable: &Path) -> Result<String, String> {
    tick_executable
        .parent()
        .and_then(Path::to_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "无法确定 Tick 工作目录".to_string())
}

#[cfg(not(target_os = "windows"))]
fn executable_working_directory(tick_executable: &Path) -> Result<String, String> {
    // Host-side tests run on macOS, where backslashes are not path separators.
    // Preserve a Windows volume/share root separator to match Path::parent on Windows.
    let executable = tick_executable
        .to_str()
        .ok_or_else(|| "Tick 可执行文件路径不是有效 Unicode".to_string())?;
    let separator = executable
        .rfind(['/', '\\'])
        .ok_or_else(|| "无法确定 Tick 工作目录".to_string())?;
    let root_separator = windows_root_separator(executable);
    let end = if root_separator == Some(separator) {
        separator + 1
    } else {
        separator
    };
    if end == 0 {
        Err("无法确定 Tick 工作目录".to_string())
    } else {
        Ok(executable[..end].to_string())
    }
}

#[cfg(not(target_os = "windows"))]
fn windows_root_separator(value: &str) -> Option<usize> {
    let bytes = value.as_bytes();
    if bytes.len() >= 3 && bytes[1] == b':' && matches!(bytes[2], b'\\' | b'/') {
        return Some(2);
    }
    if value.starts_with(r"\\?\UNC\") {
        return nth_separator_from(value, 8, 2);
    }
    if value.starts_with(r"\\?\")
        && bytes.len() >= 7
        && bytes[5] == b':'
        && matches!(bytes[6], b'\\' | b'/')
    {
        return Some(6);
    }
    if value.starts_with(r"\\") {
        return nth_separator_from(value, 2, 2);
    }
    None
}

#[cfg(not(target_os = "windows"))]
fn nth_separator_from(value: &str, start: usize, count: usize) -> Option<usize> {
    value
        .char_indices()
        .skip_while(|(index, _)| *index < start)
        .filter(|(_, character)| matches!(character, '/' | '\\'))
        .nth(count - 1)
        .map(|(index, _)| index)
}

#[cfg(test)]
pub fn has_tick_ownership(xml: &str, id: &str) -> Result<bool, String> {
    let uri = task_uri(id)?;
    Ok(xml.contains(&format!("<Source>{TASK_SOURCE}</Source>"))
        && xml.contains(&format!("<URI>{}</URI>", xml_escape(&uri)))
        && xml.contains(&format!("<Arguments>--run-scheduled-job {id}</Arguments>")))
}

fn build_trigger(job: &ScheduledJob, start_boundary: &str) -> Result<String, String> {
    let start_boundary = xml_escape(start_boundary);
    match job.schedule.mode {
        ScheduleMode::Interval => Ok(format!(
            "    <TimeTrigger>\n      <Repetition>\n        <Interval>{}</Interval>\n        <StopAtDurationEnd>false</StopAtDurationEnd>\n      </Repetition>\n      <StartBoundary>{start_boundary}</StartBoundary>\n      <Enabled>true</Enabled>\n    </TimeTrigger>",
            iso_duration(job.schedule.interval.seconds)
        )),
        ScheduleMode::Calendar => {
            let calendar = &job.schedule.calendar;
            let schedule = match (calendar.month, calendar.day) {
                (None, None) => {
                    "<ScheduleByDay><DaysInterval>1</DaysInterval></ScheduleByDay>".to_string()
                }
                (month, day) => {
                    let days = if let Some(day) = day {
                        format!("<Day>{day}</Day>")
                    } else {
                        (1..=31)
                            .map(|day| format!("<Day>{day}</Day>"))
                            .collect::<String>()
                    };
                    let months = month
                        .map(month_element)
                        .transpose()?
                        .unwrap_or_else(|| ALL_MONTHS.to_string());
                    format!(
                        "<ScheduleByMonth><DaysOfMonth>{days}</DaysOfMonth><Months>{months}</Months></ScheduleByMonth>"
                    )
                }
            };
            Ok(format!(
                "    <CalendarTrigger>\n      <StartBoundary>{start_boundary}</StartBoundary>\n      <Enabled>true</Enabled>\n      {schedule}\n    </CalendarTrigger>"
            ))
        }
    }
}

fn month_element(month: u8) -> Result<String, String> {
    let name = match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => return Err("月份必须在 1 到 12 之间".to_string()),
    };
    Ok(format!("<{name}/>"))
}

fn iso_duration(seconds: u32) -> String {
    let days = seconds / 86_400;
    let remainder = seconds % 86_400;
    let hours = remainder / 3_600;
    let remainder = remainder % 3_600;
    let minutes = remainder / 60;
    let seconds = remainder % 60;
    let mut value = String::new();
    if days > 0 {
        value.push_str(&format!("{days}D"));
    }
    if hours > 0 || minutes > 0 || seconds > 0 || days == 0 {
        value.push('T');
        if hours > 0 {
            value.push_str(&format!("{hours}H"));
        }
        if minutes > 0 {
            value.push_str(&format!("{minutes}M"));
        }
        if seconds > 0 || (hours == 0 && minutes == 0) {
            value.push_str(&format!("{seconds}S"));
        }
    }
    format!("P{value}")
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::models::{
        CalendarSchedule, ExecutionMode, IntervalSchedule, JobExecution, JobSchedule,
    };
    use std::path::PathBuf;

    fn job(mode: ScheduleMode) -> ScheduledJob {
        ScheduledJob {
            id: "job-1234567890".to_string(),
            label: "com.gavin.tick.xml-test-1234567890".to_string(),
            name: "XML".to_string(),
            description: "安全 & <测试> \"引号\" 'ok'".to_string(),
            status: JobStatus::Disabled,
            schedule: JobSchedule {
                mode,
                calendar: CalendarSchedule {
                    month: None,
                    day: None,
                    hour: Some(9),
                    minute: Some(30),
                    second: 17,
                },
                interval: IntervalSchedule { seconds: 60 },
            },
            execution: JobExecution {
                mode: ExecutionMode::Interpreter,
                inline_script: "ignored & < >".to_string(),
                script_path: r"C:\用户 输入\unsafe & script.js".to_string(),
                interpreter: r#""C:\Program Files\nodejs\node.exe""#.to_string(),
                arguments: "& calc.exe".to_string(),
                working_directory: r"C:\unsafe & work".to_string(),
                environment: vec![],
            },
            stdout_path: String::new(),
            stderr_path: String::new(),
            definition_path: String::new(),
            last_modified_at: "2026-08-30T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn action_contains_only_tick_runner_and_validated_id() {
        let value = build_task_xml(
            &job(ScheduleMode::Calendar),
            &PathBuf::from(r"C:\Program Files\Tick & Safe\tick.exe"),
            "2026-08-30T09:30:17",
        )
        .unwrap();
        assert!(value.contains("<LogonType>InteractiveToken</LogonType>"));
        assert!(value.contains("<RunLevel>LeastPrivilege</RunLevel>"));
        assert!(value.contains("<Command>C:\\Program Files\\Tick &amp; Safe\\tick.exe</Command>"));
        assert!(value.contains("<Arguments>--run-scheduled-job job-1234567890</Arguments>"));
        assert!(!value.contains("calc.exe"));
        assert!(!value.contains("unsafe & script"));
        assert!(value.contains("安全 &amp; &lt;测试&gt; &quot;引号&quot; &apos;ok&apos;"));
        assert!(has_tick_ownership(&value, "job-1234567890").unwrap());
        assert!(!has_tick_ownership(&value, "job-999").unwrap());
    }

    #[test]
    fn preserves_the_separator_for_a_drive_root_working_directory() {
        let value = build_task_xml(
            &job(ScheduleMode::Calendar),
            &PathBuf::from(r"C:\tick.exe"),
            "2026-08-30T09:30:17",
        )
        .unwrap();
        assert!(value.contains("<WorkingDirectory>C:\\</WorkingDirectory>"));
    }

    #[test]
    fn preserves_unc_and_extended_root_working_directories() {
        for (path, expected) in [
            (r"\\server\share\tick.exe", r"\\server\share\"),
            (r"\\?\C:\tick.exe", r"\\?\C:\"),
            (r"\\?\UNC\server\share\tick.exe", r"\\?\UNC\server\share\"),
        ] {
            let value = build_task_xml(
                &job(ScheduleMode::Calendar),
                &PathBuf::from(path),
                "2026-08-30T09:30:17",
            )
            .unwrap();
            assert!(
                value.contains(&format!("<WorkingDirectory>{expected}</WorkingDirectory>")),
                "missing working directory for {path:?}"
            );
        }
    }

    #[test]
    fn emits_daily_monthly_yearly_and_second_precision() {
        let executable = PathBuf::from(r"C:\Tick\tick.exe");
        let mut value = job(ScheduleMode::Calendar);
        let daily = build_task_xml(&value, &executable, "2026-08-30T09:30:17").unwrap();
        assert!(daily.contains("<ScheduleByDay>"));
        assert!(daily.contains("T09:30:17"));

        value.schedule.calendar.day = Some(15);
        let monthly = build_task_xml(&value, &executable, "2026-08-30T09:30:17").unwrap();
        assert!(monthly.contains("<Day>15</Day>"));
        assert!(monthly.contains("<January/><February/>"));

        value.schedule.calendar.month = Some(12);
        let yearly = build_task_xml(&value, &executable, "2026-08-30T09:30:17").unwrap();
        assert!(yearly.contains("<December/>"));
        assert!(!yearly.contains("<January/>"));
    }

    #[test]
    fn formats_supported_repetition_durations() {
        assert_eq!(iso_duration(60), "PT1M");
        assert_eq!(iso_duration(3_661), "PT1H1M1S");
        assert_eq!(iso_duration(2_678_400), "P31D");
    }
}

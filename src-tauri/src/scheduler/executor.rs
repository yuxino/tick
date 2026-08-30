use super::models::{ExecutionMode, ScheduledJob};
#[cfg(target_os = "macos")]
use super::paths::scripts_dir;
use super::paths::{ensure_dirs, inline_script_path, stderr_path, stdout_path};
use std::fs::OpenOptions;
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::process::{Child, Command, Stdio};

pub struct MaterializedExecution {
    pub inline_script_path: Option<PathBuf>,
}

pub fn materialize_execution(job: &ScheduledJob) -> Result<MaterializedExecution, String> {
    ensure_dirs()?;
    let inline_script_path = if job.execution.mode == ExecutionMode::InlineShell {
        let path = inline_script_path(&job.id)?;
        std::fs::write(&path, normalize_inline_script(job)).map_err(|err| err.to_string())?;
        Some(path)
    } else {
        None
    };
    Ok(MaterializedExecution { inline_script_path })
}

pub fn command_args(
    job: &ScheduledJob,
    materialized: &MaterializedExecution,
) -> Result<Vec<String>, String> {
    let extra_args = parse_command_line(&job.execution.arguments)?;
    match job.execution.mode {
        ExecutionMode::InlineShell => {
            let script = materialized
                .inline_script_path
                .as_ref()
                .ok_or_else(|| "缺少内联脚本路径".to_string())?;
            let mut args = interpreter_parts(&job.execution.interpreter)?;
            args.push(script.display().to_string());
            args.extend(extra_args);
            Ok(args)
        }
        ExecutionMode::ScriptPath => {
            let script = job.execution.script_path.trim().to_string();
            if job.execution.interpreter.trim().is_empty() {
                let mut args = vec![script];
                args.extend(extra_args);
                Ok(args)
            } else {
                let mut args = interpreter_parts(&job.execution.interpreter)?;
                args.push(script);
                args.extend(extra_args);
                Ok(args)
            }
        }
        ExecutionMode::Interpreter => {
            let mut args = interpreter_parts(&job.execution.interpreter)?;
            if !job.execution.script_path.trim().is_empty() {
                args.push(job.execution.script_path.trim().to_string());
            }
            args.extend(extra_args);
            Ok(args)
        }
    }
}

pub fn validate_execution(job: &ScheduledJob) -> Result<(), String> {
    let materialized = MaterializedExecution {
        inline_script_path: if job.execution.mode == ExecutionMode::InlineShell {
            Some(inline_script_path(&job.id)?)
        } else {
            None
        },
    };
    let args = command_args(job, &materialized)?;
    #[cfg(target_os = "windows")]
    build_windows_command_line(&args)?;
    #[cfg(not(target_os = "windows"))]
    drop(args);
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn spawn_detached(job: &ScheduledJob) -> Result<(), String> {
    let materialized = materialize_execution(job)?;
    let args = command_args(job, &materialized)?;
    spawn_process(job, args).map(|_| ())
}

pub fn run_and_wait(job: &ScheduledJob) -> Result<i32, String> {
    let materialized = materialize_execution(job)?;
    let args = command_args(job, &materialized)?;

    #[cfg(target_os = "windows")]
    {
        run_windows_job(job, &args)
    }

    #[cfg(target_os = "macos")]
    {
        let mut child = spawn_process(job, args)?;
        let status = child.wait().map_err(|err| {
            let message = format!("等待任务进程失败：{err}");
            append_runner_error(job, &message);
            message
        })?;
        Ok(status.code().unwrap_or(1))
    }
}

pub fn remove_materialized_execution(job: &ScheduledJob) -> Result<(), String> {
    remove_file_if_exists(&inline_script_path(&job.id)?)?;
    #[cfg(target_os = "macos")]
    remove_file_if_exists(&scripts_dir()?.join(format!("{}.sh", job.id)))?;
    Ok(())
}

pub fn remove_logs(job: &ScheduledJob) -> Result<(), String> {
    remove_file_if_exists(&stdout_path(&job.id)?)?;
    remove_file_if_exists(&stderr_path(&job.id)?)
}

fn remove_file_if_exists(path: &std::path::Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("无法删除 {}：{err}", path.display())),
    }
}

#[cfg(target_os = "macos")]
fn spawn_process(job: &ScheduledJob, args: Vec<String>) -> Result<Child, String> {
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

    command.spawn().map_err(|err| {
        let message = format!("启动任务进程失败：{err}");
        append_runner_error(job, &message);
        message
    })
}

fn append_runner_error(job: &ScheduledJob, message: &str) {
    use std::io::Write;
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&job.stderr_path)
    {
        let _ = writeln!(file, "[Tick] {message}");
    }
}

pub fn append_runner_error_for_id(id: &str, message: &str) -> Result<(), String> {
    ensure_dirs()?;
    let path = stderr_path(id)?;
    use std::io::Write;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|err| format!("无法打开 {}：{err}", path.display()))?;
    writeln!(file, "[Tick] {message}").map_err(|err| err.to_string())
}

fn normalize_inline_script(job: &ScheduledJob) -> String {
    let script = &job.execution.inline_script;
    if script.starts_with("#!") {
        format!("{script}\n")
    } else if job
        .execution
        .interpreter
        .to_ascii_lowercase()
        .contains("node")
    {
        format!("#!/usr/bin/env node\n{script}\n")
    } else {
        format!("{script}\n")
    }
}

fn interpreter_parts(value: &str) -> Result<Vec<String>, String> {
    let value = value.trim();
    if value.is_empty() {
        #[cfg(target_os = "windows")]
        return Ok(vec!["node.exe".to_string()]);
        #[cfg(not(target_os = "windows"))]
        return Ok(vec!["/usr/bin/env".to_string(), "node".to_string()]);
    }
    let parts = parse_command_line(value)?;
    if parts.is_empty() {
        Err("解释器命令为空".to_string())
    } else {
        Ok(parts)
    }
}

fn parse_command_line(value: &str) -> Result<Vec<String>, String> {
    if value.trim().is_empty() {
        return Ok(vec![]);
    }
    #[cfg(target_os = "windows")]
    {
        split_windows_command_line(value)
    }
    #[cfg(not(target_os = "windows"))]
    {
        shell_words::split(value).map_err(|err| err.to_string())
    }
}

#[cfg(any(target_os = "windows", test))]
fn validate_windows_command_line_length(value: &str) -> Result<(), String> {
    // CreateProcessW accepts at most 32,767 UTF-16 code units including the trailing NUL.
    if value.encode_utf16().count() + 1 > 32_767 {
        Err("Windows 命令行不能超过 32767 个 UTF-16 代码单元".to_string())
    } else {
        Ok(())
    }
}

#[cfg(any(target_os = "windows", test))]
fn build_windows_command_line(args: &[String]) -> Result<String, String> {
    if args.is_empty() {
        return Err("命令为空".to_string());
    }
    let command_line = args
        .iter()
        .map(|argument| quote_windows_argument(argument))
        .collect::<Vec<_>>()
        .join(" ");
    validate_windows_command_line_length(&command_line)?;
    Ok(command_line)
}

#[cfg(any(target_os = "windows", test))]
fn split_windows_command_line(value: &str) -> Result<Vec<String>, String> {
    let chars = value.chars().collect::<Vec<_>>();
    let mut index = 0;
    let mut result = Vec::new();

    while index < chars.len() {
        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }
        if index == chars.len() {
            break;
        }

        let mut argument = String::new();
        let mut in_quotes = false;
        let mut started = false;
        while index < chars.len() {
            if !in_quotes && chars[index].is_whitespace() {
                break;
            }

            let mut backslashes = 0;
            while index < chars.len() && chars[index] == '\\' {
                backslashes += 1;
                index += 1;
            }

            if index < chars.len() && chars[index] == '"' {
                argument.extend(std::iter::repeat_n('\\', backslashes / 2));
                if backslashes % 2 == 1 {
                    argument.push('"');
                } else if in_quotes && index + 1 < chars.len() && chars[index + 1] == '"' {
                    argument.push('"');
                    index += 1;
                } else {
                    in_quotes = !in_quotes;
                }
                started = true;
                index += 1;
                continue;
            }

            argument.extend(std::iter::repeat_n('\\', backslashes));
            if backslashes > 0 {
                started = true;
            }
            if index == chars.len() || (!in_quotes && chars[index].is_whitespace()) {
                break;
            }
            argument.push(chars[index]);
            started = true;
            index += 1;
        }

        if in_quotes {
            return Err("Windows 参数中的双引号未闭合".to_string());
        }
        if started {
            result.push(argument);
        }
        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }
    }

    Ok(result)
}

#[cfg(target_os = "windows")]
fn run_windows_job(job: &ScheduledJob, args: &[String]) -> Result<i32, String> {
    use std::cmp::Ordering;
    use std::ffi::{OsStr, OsString};
    use std::fs::File;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::AsRawHandle;
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Foundation::{
        DuplicateHandle, DUPLICATE_SAME_ACCESS, HANDLE, WAIT_OBJECT_0,
    };
    use windows::Win32::Globalization::{
        CompareStringOrdinal, CSTR_EQUAL, CSTR_GREATER_THAN, CSTR_LESS_THAN,
    };
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows::Win32::System::Threading::{
        CreateProcessW, GetCurrentProcess, GetExitCodeProcess, ResumeThread, TerminateProcess,
        WaitForSingleObject, CREATE_NO_WINDOW, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT,
        INFINITE, PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, STARTF_USESTDHANDLES, STARTUPINFOW,
    };

    fn compare_environment_keys(left: &OsStr, right: &OsStr) -> Ordering {
        let left_wide = left.encode_wide().collect::<Vec<_>>();
        let right_wide = right.encode_wide().collect::<Vec<_>>();
        match unsafe { CompareStringOrdinal(&left_wide, &right_wide, true) } {
            CSTR_LESS_THAN => Ordering::Less,
            CSTR_EQUAL => Ordering::Equal,
            CSTR_GREATER_THAN => Ordering::Greater,
            _ => left_wide.cmp(&right_wide),
        }
    }

    fn wide_null(value: &OsStr) -> Vec<u16> {
        value.encode_wide().chain(std::iter::once(0)).collect()
    }

    fn duplicate_inheritable(file: &File) -> Result<OwnedWindowsHandle, String> {
        let current = unsafe { GetCurrentProcess() };
        let source = HANDLE(file.as_raw_handle());
        let mut duplicate = HANDLE::default();
        unsafe {
            DuplicateHandle(
                current,
                source,
                current,
                &mut duplicate,
                0,
                true,
                DUPLICATE_SAME_ACCESS,
            )
        }
        .map_err(|err| format!("无法准备任务日志句柄：{err}"))?;
        Ok(OwnedWindowsHandle(duplicate))
    }

    fn environment_block(job: &ScheduledJob) -> Vec<u16> {
        let mut values = std::env::vars_os().collect::<Vec<(OsString, OsString)>>();
        for item in job
            .execution
            .environment
            .iter()
            .filter(|item| !item.key.trim().is_empty())
        {
            let key = OsString::from(item.key.trim());
            if let Some(existing) = values.iter_mut().find(|(candidate, _)| {
                compare_environment_keys(candidate.as_os_str(), key.as_os_str()) == Ordering::Equal
            }) {
                *existing = (key, OsString::from(&item.value));
            } else {
                values.push((key, OsString::from(&item.value)));
            }
        }
        values.sort_by(|left, right| {
            compare_environment_keys(left.0.as_os_str(), right.0.as_os_str())
        });

        let mut block = Vec::new();
        for (key, value) in values {
            block.extend(key.encode_wide());
            block.push('=' as u16);
            block.extend(value.encode_wide());
            block.push(0);
        }
        block.push(0);
        if block.len() == 1 {
            block.push(0);
        }
        block
    }

    let (program, _) = args.split_first().ok_or_else(|| "命令为空".to_string())?;
    let command_line = build_windows_command_line(args)?;
    let mut command_line = wide_null(OsStr::new(&command_line));
    let program = wide_null(OsStr::new(program));
    let working_directory = if job.execution.working_directory.trim().is_empty() {
        None
    } else {
        Some(wide_null(OsStr::new(
            job.execution.working_directory.trim(),
        )))
    };
    let environment = environment_block(job);

    let stdout_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&job.stdout_path)
        .map_err(|err| format!("无法打开 stdout 日志：{err}"))?;
    let stderr_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&job.stderr_path)
        .map_err(|err| format!("无法打开 stderr 日志：{err}"))?;
    let stdin_file = OpenOptions::new()
        .read(true)
        .open("NUL")
        .map_err(|err| format!("无法打开空输入设备：{err}"))?;
    let stdout = duplicate_inheritable(&stdout_file)?;
    let stderr = duplicate_inheritable(&stderr_file)?;
    let stdin = duplicate_inheritable(&stdin_file)?;

    let job_object = unsafe {
        let handle = CreateJobObjectW(None, PCWSTR::null())
            .map_err(|err| format!("无法创建任务进程组：{err}"))?;
        let owned = OwnedWindowsHandle(handle);
        let mut information = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        information.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        SetInformationJobObject(
            owned.0,
            JobObjectExtendedLimitInformation,
            std::ptr::addr_of!(information).cast(),
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
        .map_err(|err| format!("无法保护任务进程组：{err}"))?;
        owned
    };

    let startup = STARTUPINFOW {
        cb: std::mem::size_of::<STARTUPINFOW>() as u32,
        dwFlags: STARTF_USESTDHANDLES,
        hStdInput: stdin.0,
        hStdOutput: stdout.0,
        hStdError: stderr.0,
        ..Default::default()
    };
    let mut process_information = PROCESS_INFORMATION::default();
    let creation_flags = PROCESS_CREATION_FLAGS(
        CREATE_SUSPENDED.0 | CREATE_UNICODE_ENVIRONMENT.0 | CREATE_NO_WINDOW.0,
    );
    let current_directory = working_directory
        .as_ref()
        .map_or(PCWSTR::null(), |value| PCWSTR(value.as_ptr()));

    unsafe {
        CreateProcessW(
            PCWSTR(program.as_ptr()),
            Some(PWSTR(command_line.as_mut_ptr())),
            None,
            None,
            true,
            creation_flags,
            Some(environment.as_ptr().cast()),
            current_directory,
            &startup,
            &mut process_information,
        )
    }
    .map_err(|err| {
        let message = format!("启动任务进程失败：{err}");
        append_runner_error(job, &message);
        message
    })?;

    let process = OwnedWindowsHandle(process_information.hProcess);
    let thread = OwnedWindowsHandle(process_information.hThread);
    if let Err(err) = unsafe { AssignProcessToJobObject(job_object.0, process.0) } {
        unsafe {
            let _ = TerminateProcess(process.0, 1);
            WaitForSingleObject(process.0, INFINITE);
        }
        let message = format!("无法保护任务子进程：{err}");
        append_runner_error(job, &message);
        return Err(message);
    }
    if unsafe { ResumeThread(thread.0) } == u32::MAX {
        unsafe {
            let _ = TerminateProcess(process.0, 1);
            WaitForSingleObject(process.0, INFINITE);
        }
        let message = "无法恢复任务子进程".to_string();
        append_runner_error(job, &message);
        return Err(message);
    }

    let wait_result = unsafe { WaitForSingleObject(process.0, INFINITE) };
    if wait_result != WAIT_OBJECT_0 {
        let message = format!("等待任务进程失败：Windows 状态 {}", wait_result.0);
        append_runner_error(job, &message);
        return Err(message);
    }
    let mut exit_code = 1_u32;
    unsafe { GetExitCodeProcess(process.0, &mut exit_code) }
        .map_err(|err| format!("读取任务退出状态失败：{err}"))?;
    Ok(exit_code as i32)
}

#[cfg(target_os = "windows")]
struct OwnedWindowsHandle(windows::Win32::Foundation::HANDLE);

#[cfg(target_os = "windows")]
impl Drop for OwnedWindowsHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(any(target_os = "windows", test))]
fn quote_windows_argument(value: &str) -> String {
    if !value.is_empty()
        && !value
            .chars()
            .any(|character| character.is_whitespace() || character == '"')
    {
        return value.to_string();
    }

    let mut quoted = String::from("\"");
    let mut backslashes = 0;
    for character in value.chars() {
        if character == '\\' {
            backslashes += 1;
        } else if character == '"' {
            quoted.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
            quoted.push('"');
            backslashes = 0;
        } else {
            quoted.extend(std::iter::repeat_n('\\', backslashes));
            backslashes = 0;
            quoted.push(character);
        }
    }
    quoted.extend(std::iter::repeat_n('\\', backslashes * 2));
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_windows_paths_quotes_empty_values_and_metacharacters() {
        assert_eq!(
            split_windows_command_line(
                r#""C:\Program Files\nodejs\node.exe" "C:\工作 目录\job.js" "" "a&b|c<d>e^f%g""#,
            )
            .unwrap(),
            vec![
                r"C:\Program Files\nodejs\node.exe",
                r"C:\工作 目录\job.js",
                "",
                "a&b|c<d>e^f%g",
            ]
        );
    }

    #[test]
    fn preserves_windows_backslashes_and_escaped_quotes() {
        assert_eq!(
            split_windows_command_line(r#"C:\tools\node.exe "say \"hello\"" C:\tail\"#).unwrap(),
            vec![r"C:\tools\node.exe", "say \"hello\"", r"C:\tail\"]
        );
        assert_eq!(
            split_windows_command_line(r"\ \\").unwrap(),
            vec![r"\", r"\\"]
        );
    }

    #[test]
    fn rejects_windows_command_lines_over_the_create_process_limit() {
        assert!(validate_windows_command_line_length(&"a".repeat(32_766)).is_ok());
        assert!(validate_windows_command_line_length(&"a".repeat(32_767)).is_err());
        assert!(validate_windows_command_line_length(&"界".repeat(32_766)).is_ok());
        assert!(validate_windows_command_line_length(&"😀".repeat(16_383)).is_ok());
        assert!(validate_windows_command_line_length(&"😀".repeat(16_384)).is_err());
        assert!(build_windows_command_line(&[]).is_err());
        assert!(
            build_windows_command_line(&["node.exe".to_string(), "a&b".to_string()])
                .unwrap()
                .ends_with("node.exe a&b")
        );
    }

    #[test]
    fn rejects_unclosed_windows_quotes() {
        assert!(split_windows_command_line(r#""C:\Program Files\node.exe"#).is_err());
    }

    #[test]
    fn quotes_windows_arguments_without_shell_interpretation() {
        assert_eq!(quote_windows_argument("plain"), "plain");
        assert_eq!(quote_windows_argument(""), r#""""#);
        assert_eq!(
            quote_windows_argument(r"C:\work dir\"),
            r#""C:\work dir\\""#
        );
        assert_eq!(quote_windows_argument("a&b|c<d>e^f%g"), "a&b|c<d>e^f%g");
        assert_eq!(quote_windows_argument("say \"hi\""), r#""say \"hi\"""#);
    }
}

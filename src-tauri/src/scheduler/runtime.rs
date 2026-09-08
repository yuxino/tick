use super::executor::interpreter_parts;
use super::models::{ExecutionMode, JobExecution};
use crate::process_output::capture_output;
use serde::Serialize;
use std::path::Path;
#[cfg(any(target_os = "windows", test))]
use std::path::PathBuf;
use tokio::process::Command;
use tokio::time::Duration;

const NODE_DETECTION_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_NODE_OUTPUT_BYTES: usize = 4 * 1024;
const MAX_DETAIL_CHARS: usize = 200;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeRuntimeStatus {
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

struct DetectedNodeRuntime {
    status: NodeRuntimeStatus,
    command: Option<Vec<String>>,
}

pub async fn detect_default_node_runtime() -> NodeRuntimeStatus {
    resolve_default_node_runtime().await.status
}

pub async fn default_node_command() -> Result<Vec<String>, String> {
    let detected = resolve_default_node_runtime().await;
    detected.command.ok_or_else(|| {
        missing_node_error(detected.status.reason.as_deref().unwrap_or("未知检测错误"))
    })
}

pub async fn ensure_node_runtime_for_execution(execution: &JobExecution) -> Result<(), String> {
    let Some(command) = node_command_for_execution(execution)? else {
        return Ok(());
    };
    let status = if is_default_node_command(&command) {
        resolve_default_node_runtime().await.status
    } else {
        probe_node_command(&command).await
    };
    if status.available {
        Ok(())
    } else {
        Err(missing_node_error(
            status.reason.as_deref().unwrap_or("未知检测错误"),
        ))
    }
}

fn missing_node_error(detail: &str) -> String {
    format!(
        "未检测到可用的 Node.js，Tick 的 JavaScript 任务无法运行。请先自行安装 Node.js，然后回到 Tick 点击“重新检测”；Tick 不会自动安装或修改 PATH。若刚安装仍未识别，请完全退出 Tick 后重新打开。（检测详情：{detail}）"
    )
}

async fn resolve_default_node_runtime() -> DetectedNodeRuntime {
    let commands = default_node_commands();
    let mut last_reason = None;
    for command in commands {
        let status = probe_node_command(&command).await;
        if status.available {
            return DetectedNodeRuntime {
                status,
                command: Some(command),
            };
        }
        last_reason = status.reason;
    }

    #[cfg(target_os = "windows")]
    let reason = format!(
        "已检查当前 PATH、Node.js 注册表和常见安装目录，仍未找到可运行的 node.exe{}",
        last_reason
            .as_deref()
            .map(|detail| format!("；最后一次错误：{detail}"))
            .unwrap_or_default()
    );
    #[cfg(not(target_os = "windows"))]
    let reason = last_reason.unwrap_or_else(|| "没有找到可运行的 Node.js".to_string());

    DetectedNodeRuntime {
        status: unavailable(reason),
        command: None,
    }
}

fn default_node_commands() -> Vec<Vec<String>> {
    #[cfg(target_os = "windows")]
    {
        let mut commands = windows_node_candidates()
            .into_iter()
            .map(|path| vec![path.to_string_lossy().to_string()])
            .collect::<Vec<_>>();
        commands.push(vec!["node.exe".to_string()]);
        commands
    }
    #[cfg(not(target_os = "windows"))]
    {
        vec![interpreter_parts("").expect("默认 Node.js 命令必须有效")]
    }
}

fn node_command_for_execution(execution: &JobExecution) -> Result<Option<Vec<String>>, String> {
    let interpreter = match execution.mode {
        ExecutionMode::InlineShell | ExecutionMode::Interpreter => {
            interpreter_parts(&execution.interpreter)?
        }
        ExecutionMode::ScriptPath if execution.interpreter.trim().is_empty() => return Ok(None),
        ExecutionMode::ScriptPath => interpreter_parts(&execution.interpreter)?,
    };
    Ok(is_node_command(&interpreter).then_some(interpreter))
}

fn is_default_node_command(command: &[String]) -> bool {
    #[cfg(target_os = "windows")]
    {
        command
            .first()
            .is_some_and(|program| is_node_executable(program) && is_bare_program(program))
    }
    #[cfg(not(target_os = "windows"))]
    {
        command == ["/usr/bin/env", "node"]
            || command
                .first()
                .is_some_and(|program| is_node_executable(program) && is_bare_program(program))
    }
}

fn is_node_command(command: &[String]) -> bool {
    let Some(program) = command.first() else {
        return false;
    };
    if is_node_executable(program) {
        return true;
    }
    matches!(executable_name(program).as_str(), "env" | "env.exe")
        && command
            .get(1)
            .is_some_and(|value| is_node_executable(value))
}

fn is_node_executable(value: &str) -> bool {
    matches!(
        executable_name(value).as_str(),
        "node" | "node.exe" | "nodejs"
    )
}

fn is_bare_program(value: &str) -> bool {
    Path::new(value)
        .parent()
        .is_none_or(|parent| parent.as_os_str().is_empty())
}

fn executable_name(value: &str) -> String {
    Path::new(value)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(value)
        .to_ascii_lowercase()
}

async fn probe_node_command(command: &[String]) -> NodeRuntimeStatus {
    let Some(program) = command.first() else {
        return unavailable("Node.js 检测命令为空".to_string());
    };
    let mut process = Command::new(program);
    if matches!(executable_name(program).as_str(), "env" | "env.exe") {
        if let Some(node) = command.get(1) {
            process.arg(node);
        }
    }
    process.arg("--version");

    #[cfg(target_os = "windows")]
    hide_windows_console(&mut process);

    probe_node_process(&mut process, &command_label(command)).await
}

async fn probe_node_process(process: &mut Command, label: &str) -> NodeRuntimeStatus {
    match capture_output(process, NODE_DETECTION_TIMEOUT, MAX_NODE_OUTPUT_BYTES).await {
        Ok(output) if output.timed_out => unavailable(format!("Node.js 检测超过 3 秒：{label}")),
        Ok(output) if output.stdout.truncated || output.stderr.truncated => {
            unavailable("Node.js 检测输出超过 4 KiB，已忽略异常结果".to_string())
        }
        Ok(output) if output.status.is_some_and(|status| status.success()) => {
            let stdout = String::from_utf8_lossy(&output.stdout.bytes)
                .trim()
                .to_string();
            let stderr = String::from_utf8_lossy(&output.stderr.bytes)
                .trim()
                .to_string();
            let version = if stdout.is_empty() { stderr } else { stdout };
            NodeRuntimeStatus {
                available: true,
                version: (!version.is_empty()).then_some(version),
                executable_path: Some(label.to_string()),
                reason: None,
            }
        }
        Ok(output) => {
            let stderr = bounded_detail(&String::from_utf8_lossy(&output.stderr.bytes));
            unavailable(if stderr.is_empty() {
                let status = output
                    .status
                    .map(|status| status.to_string())
                    .unwrap_or_else(|| "未知".to_string());
                format!("Node.js 检测命令退出码为 {status}")
            } else {
                format!("Node.js 检测失败：{stderr}")
            })
        }
        Err(error) => unavailable(format!("无法启动 {label}：{error}")),
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn hide_windows_console(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.as_std_mut().creation_flags(CREATE_NO_WINDOW);
}

fn command_label(command: &[String]) -> String {
    command.join(" ")
}

fn unavailable(reason: String) -> NodeRuntimeStatus {
    NodeRuntimeStatus {
        available: false,
        version: None,
        executable_path: None,
        reason: Some(reason),
    }
}

fn bounded_detail(value: &str) -> String {
    value.trim().chars().take(MAX_DETAIL_CHARS).collect()
}

#[cfg(target_os = "windows")]
pub fn resolve_windows_node_program(program: &str) -> Option<PathBuf> {
    if !is_node_executable(program) || !is_bare_program(program) {
        return None;
    }
    windows_node_candidates().into_iter().next()
}

#[cfg(target_os = "windows")]
fn windows_node_candidates() -> Vec<PathBuf> {
    let path_match = search_windows_path_for_node();
    let registry_paths = windows_registry_install_paths();
    windows_node_candidates_from(
        path_match,
        std::env::var_os("NVM_SYMLINK").map(PathBuf::from),
        std::env::var_os("ProgramW6432").map(PathBuf::from),
        std::env::var_os("ProgramFiles").map(PathBuf::from),
        std::env::var_os("ProgramFiles(x86)").map(PathBuf::from),
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
        registry_paths,
    )
    .into_iter()
    .filter(|path| path.is_file())
    .collect()
}

#[cfg(target_os = "windows")]
fn search_windows_path_for_node() -> Option<PathBuf> {
    use windows::core::w;
    use windows::Win32::Storage::FileSystem::SearchPathW;

    let mut buffer = vec![0_u16; 32_768];
    let length =
        unsafe { SearchPathW(None, w!("node.exe"), None, Some(&mut buffer), None) } as usize;
    (length > 0 && length < buffer.len())
        .then(|| PathBuf::from(String::from_utf16_lossy(&buffer[..length])))
}

#[cfg(target_os = "windows")]
fn windows_registry_install_paths() -> Vec<PathBuf> {
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    let mut paths = Vec::new();
    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        for subkey in [r"SOFTWARE\Node.js", r"SOFTWARE\WOW6432Node\Node.js"] {
            for value_name in ["InstallPath", ""] {
                if let Some(value) = read_windows_registry_string(root, subkey, value_name) {
                    paths.push(PathBuf::from(value));
                }
            }
        }
    }
    paths
}

#[cfg(target_os = "windows")]
fn read_windows_registry_string(
    root: windows::Win32::System::Registry::HKEY,
    subkey: &str,
    value_name: &str,
) -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{RegGetValueW, RRF_RT_REG_SZ};

    let subkey = std::ffi::OsStr::new(subkey)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let value_name = std::ffi::OsStr::new(value_name)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut buffer = vec![0_u16; 32_768];
    let mut size = (buffer.len() * std::mem::size_of::<u16>()) as u32;
    let result = unsafe {
        RegGetValueW(
            root,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value_name.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if result != ERROR_SUCCESS || size < 2 {
        return None;
    }
    let length = (size as usize / std::mem::size_of::<u16>()).min(buffer.len());
    let value = String::from_utf16_lossy(&buffer[..length])
        .trim_end_matches('\0')
        .trim()
        .trim_matches('"')
        .to_string();
    (!value.is_empty()).then_some(value)
}

#[cfg(any(target_os = "windows", test))]
fn windows_node_candidates_from(
    path_match: Option<PathBuf>,
    nvm_symlink: Option<PathBuf>,
    program_w6432: Option<PathBuf>,
    program_files: Option<PathBuf>,
    program_files_x86: Option<PathBuf>,
    local_app_data: Option<PathBuf>,
    registry_paths: Vec<PathBuf>,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = path_match {
        candidates.push(node_executable_path(path));
    }
    if let Some(path) = nvm_symlink {
        candidates.push(node_executable_path(path));
    }
    for directory in [program_w6432, program_files, program_files_x86]
        .into_iter()
        .flatten()
    {
        candidates.push(directory.join("nodejs").join("node.exe"));
    }
    if let Some(directory) = local_app_data {
        candidates.push(directory.join("Programs").join("nodejs").join("node.exe"));
        candidates.push(directory.join("nodejs").join("node.exe"));
    }
    candidates.extend(registry_paths.into_iter().map(node_executable_path));

    let mut unique = Vec::new();
    for candidate in candidates {
        let key = candidate.to_string_lossy().to_ascii_lowercase();
        if !unique.iter().any(|(existing, _)| existing == &key) {
            unique.push((key, candidate));
        }
    }
    unique.into_iter().map(|(_, path)| path).collect()
}

#[cfg(any(target_os = "windows", test))]
fn node_executable_path(path: PathBuf) -> PathBuf {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("node.exe"))
    {
        path
    } else {
        path.join("node.exe")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn detects_a_real_node_version() {
        let status = probe_node_command(&["node".to_string()]).await;
        assert!(status.available, "{:?}", status.reason);
        assert!(status.version.unwrap().starts_with('v'));
    }

    #[tokio::test]
    async fn rejects_excessive_detection_output_from_either_stream() {
        for stream in ["stdout", "stderr"] {
            let mut process = Command::new("node");
            process.args([
                "-e",
                &format!("process.{stream}.write('x'.repeat(1048576));"),
            ]);
            let status = probe_node_process(&mut process, "noisy Node.js fixture").await;
            assert!(!status.available, "accepted excessive {stream}");
            assert!(status.version.is_none());
            assert!(status.reason.unwrap().contains("输出超过"));
        }
    }

    #[tokio::test]
    async fn keeps_failure_details_bounded_without_splitting_unicode() {
        let mut process = Command::new("node");
        process.args([
            "-e",
            "process.stderr.write('错'.repeat(300)); process.exitCode = 2;",
        ]);
        let status = probe_node_process(&mut process, "failing Node.js fixture").await;
        assert!(!status.available);
        assert_eq!(
            status.reason.unwrap(),
            format!("Node.js 检测失败：{}", "错".repeat(MAX_DETAIL_CHARS))
        );
    }

    #[tokio::test]
    async fn reports_detection_spawn_failure() {
        let status = probe_node_command(&["tick-nonexistent-node-runtime".to_string()]).await;
        assert!(!status.available);
        assert!(status.reason.unwrap().contains("无法启动"));
    }

    #[tokio::test]
    async fn reports_detection_timeout() {
        let mut process = Command::new("node");
        process.args(["-e", "setInterval(() => {}, 1000);"]);
        let status = probe_node_process(&mut process, "hanging Node.js fixture").await;
        assert!(!status.available);
        assert!(status.reason.unwrap().contains("检测超过 3 秒"));
    }

    fn execution(mode: ExecutionMode, interpreter: &str) -> JobExecution {
        JobExecution {
            mode,
            inline_script: "console.log('ok')".to_string(),
            script_path: String::new(),
            interpreter: interpreter.to_string(),
            arguments: String::new(),
            working_directory: String::new(),
            environment: vec![],
        }
    }

    #[test]
    fn default_inline_execution_requires_node() {
        assert!(
            node_command_for_execution(&execution(ExecutionMode::InlineShell, ""))
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn explicit_node_path_requires_node() {
        let interpreter = if cfg!(target_os = "windows") {
            r#""C:\Program Files\nodejs\node.exe" --no-warnings"#
        } else {
            "/opt/node/bin/node --no-warnings"
        };
        assert!(
            node_command_for_execution(&execution(ExecutionMode::Interpreter, interpreter))
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn non_node_interpreter_is_not_blocked() {
        assert!(
            node_command_for_execution(&execution(ExecutionMode::Interpreter, "python3"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn directly_executed_script_is_not_blocked() {
        assert!(
            node_command_for_execution(&execution(ExecutionMode::ScriptPath, ""))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn missing_node_error_explains_cause_next_step_and_install_boundary() {
        let error = missing_node_error("找不到 node.exe");
        assert!(error.contains("未检测到可用的 Node.js"));
        assert!(error.contains("自行安装 Node.js"));
        assert!(error.contains("重新检测"));
        assert!(error.contains("不会自动安装或修改 PATH"));
        assert!(error.contains("完全退出 Tick 后重新打开"));
        assert!(error.contains("找不到 node.exe"));
    }

    #[test]
    fn bare_node_command_uses_default_discovery() {
        assert!(is_bare_program("node.exe"));
        assert!(is_bare_program("node"));
        assert!(!is_bare_program("C:/nodejs/node.exe"));
        assert!(!is_bare_program("/opt/node/bin/node"));
    }

    #[test]
    fn windows_candidates_cover_path_nvm_official_user_and_registry_locations() {
        let candidates = windows_node_candidates_from(
            Some(PathBuf::from("C:/path/node.exe")),
            Some(PathBuf::from("C:/nvm/current")),
            Some(PathBuf::from("C:/Program Files")),
            Some(PathBuf::from("C:/Program Files")),
            Some(PathBuf::from("C:/Program Files (x86)")),
            Some(PathBuf::from("C:/Users/test/AppData/Local")),
            vec![PathBuf::from("D:/Custom Node")],
        );
        let labels = candidates
            .iter()
            .map(|path| path.to_string_lossy().replace('\\', "/"))
            .collect::<Vec<_>>();
        assert_eq!(labels[0], "C:/path/node.exe");
        assert!(labels.contains(&"C:/nvm/current/node.exe".to_string()));
        assert_eq!(
            labels
                .iter()
                .filter(|value| *value == "C:/Program Files/nodejs/node.exe")
                .count(),
            1
        );
        assert!(
            labels.contains(&"C:/Users/test/AppData/Local/Programs/nodejs/node.exe".to_string())
        );
        assert!(labels.contains(&"D:/Custom Node/node.exe".to_string()));
    }
}

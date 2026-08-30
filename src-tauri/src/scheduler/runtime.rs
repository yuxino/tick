use super::executor::interpreter_parts;
use super::models::{ExecutionMode, JobExecution};
use serde::Serialize;
use std::path::Path;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

const NODE_DETECTION_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_DETAIL_CHARS: usize = 200;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeRuntimeStatus {
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

pub async fn detect_default_node_runtime() -> NodeRuntimeStatus {
    let command = interpreter_parts("").expect("默认 Node.js 命令必须有效");
    probe_node_command(&command).await
}

pub async fn ensure_node_runtime_for_execution(execution: &JobExecution) -> Result<(), String> {
    let Some(command) = node_command_for_execution(execution)? else {
        return Ok(());
    };
    let status = probe_node_command(&command).await;
    if status.available {
        Ok(())
    } else {
        let detail = status.reason.unwrap_or_else(|| "未知检测错误".to_string());
        Err(missing_node_error(&detail))
    }
}

pub async fn ensure_default_node_runtime() -> Result<(), String> {
    let status = detect_default_node_runtime().await;
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
        "未检测到可用的 Node.js，Tick 的 JavaScript 任务无法运行。请先自行安装 Node.js，然后回到 Tick 点击“重新检测”；Tick 不会自动安装。（检测详情：{detail}）"
    )
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

fn is_node_command(command: &[String]) -> bool {
    let Some(program) = command.first() else {
        return false;
    };
    let program_name = executable_name(program);
    if matches!(program_name.as_str(), "node" | "node.exe" | "nodejs") {
        return true;
    }
    matches!(program_name.as_str(), "env" | "env.exe")
        && command.get(1).is_some_and(|value| {
            matches!(
                executable_name(value).as_str(),
                "node" | "node.exe" | "nodejs"
            )
        })
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
    process.arg("--version").kill_on_drop(true);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        process.as_std_mut().creation_flags(CREATE_NO_WINDOW);
    }

    match timeout(NODE_DETECTION_TIMEOUT, process.output()).await {
        Ok(Ok(output)) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            NodeRuntimeStatus {
                available: true,
                version: (!version.is_empty()).then_some(version),
                reason: None,
            }
        }
        Ok(Ok(output)) => {
            let stderr = bounded_detail(&String::from_utf8_lossy(&output.stderr));
            unavailable(if stderr.is_empty() {
                format!("Node.js 检测命令退出码为 {}", output.status)
            } else {
                format!("Node.js 检测失败：{stderr}")
            })
        }
        Ok(Err(error)) => unavailable(format!("无法启动 {program}：{error}")),
        Err(_) => unavailable("Node.js 检测超过 3 秒".to_string()),
    }
}

fn unavailable(reason: String) -> NodeRuntimeStatus {
    NodeRuntimeStatus {
        available: false,
        version: None,
        reason: Some(reason),
    }
}

fn bounded_detail(value: &str) -> String {
    value.trim().chars().take(MAX_DETAIL_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(error.contains("不会自动安装"));
        assert!(error.contains("找不到 node.exe"));
    }
}

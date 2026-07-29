use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::process::Command as TokioCommand;
use tokio::time::{timeout, Duration};

const DEEPSEEK_URL: &str = "https://api.deepseek.com/chat/completions";
const DEEPSEEK_MODEL: &str = "deepseek-chat";
const KEYCHAIN_SERVICE: &str = "com.gavin.tick";
const KEYCHAIN_ACCOUNT: &str = "deepseek-api-key";
const MAX_API_KEY_LEN: usize = 512;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepSeekConfigStatus {
    pub configured: bool,
    pub masked_hint: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDeepSeekApiKeyRequest {
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateNodeScriptRequest {
    pub prompt: String,
    pub current_script: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateNodeScriptResponse {
    pub script: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunNodeScriptDebugRequest {
    pub script: String,
    pub working_directory: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunNodeScriptDebugResponse {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub duration_ms: u128,
    pub timed_out: bool,
}

#[tauri::command]
pub fn get_deepseek_config_status() -> Result<DeepSeekConfigStatus, String> {
    match keychain_entry()?.get_password() {
        Ok(key) if !key.trim().is_empty() => Ok(DeepSeekConfigStatus {
            configured: true,
            masked_hint: Some(mask_api_key(key.trim())),
        }),
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(DeepSeekConfigStatus {
            configured: false,
            masked_hint: None,
        }),
        Err(_) => Err("无法读取 macOS 钥匙串，请检查 Tick 的钥匙串访问权限".to_string()),
    }
}

#[tauri::command]
pub fn save_deepseek_api_key(
    input: SaveDeepSeekApiKeyRequest,
) -> Result<DeepSeekConfigStatus, String> {
    let key = validate_api_key(&input.api_key)?;
    keychain_entry()?
        .set_password(key)
        .map_err(|_| "无法写入 macOS 钥匙串，请检查 Tick 的钥匙串访问权限".to_string())?;

    Ok(DeepSeekConfigStatus {
        configured: true,
        masked_hint: Some(mask_api_key(key)),
    })
}

#[tauri::command]
pub fn delete_deepseek_api_key() -> Result<(), String> {
    match keychain_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err("无法从 macOS 钥匙串删除 DeepSeek API Key".to_string()),
    }
}

#[tauri::command]
pub async fn test_deepseek_connection() -> Result<(), String> {
    let api_key = deepseek_api_key()?;
    let request = DeepSeekRequest {
        model: DEEPSEEK_MODEL.to_string(),
        temperature: 0.0,
        max_tokens: 1,
        messages: vec![DeepSeekMessage {
            role: "user".to_string(),
            content: "Reply OK.".to_string(),
        }],
    };

    send_deepseek_request(&api_key, &request).await?;
    Ok(())
}

#[derive(Debug, Serialize)]
struct DeepSeekRequest {
    model: String,
    messages: Vec<DeepSeekMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Serialize)]
struct DeepSeekMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct DeepSeekResponse {
    choices: Vec<DeepSeekChoice>,
}

#[derive(Debug, Deserialize)]
struct DeepSeekChoice {
    message: DeepSeekResponseMessage,
}

#[derive(Debug, Deserialize)]
struct DeepSeekResponseMessage {
    content: String,
}

#[tauri::command]
pub async fn generate_node_script(
    input: GenerateNodeScriptRequest,
) -> Result<GenerateNodeScriptResponse, String> {
    let prompt = input.prompt.trim();
    if prompt.is_empty() {
        return Err("先描述你希望脚本做什么".to_string());
    }

    let api_key = deepseek_api_key()?;
    let request = DeepSeekRequest {
        model: DEEPSEEK_MODEL.to_string(),
        temperature: 0.2,
        max_tokens: 1800,
        messages: vec![
            DeepSeekMessage {
                role: "system".to_string(),
                content: system_prompt(),
            },
            DeepSeekMessage {
                role: "user".to_string(),
                content: user_prompt(prompt, input.current_script.as_deref()),
            },
        ],
    };

    let completion = send_deepseek_request(&api_key, &request).await?;

    let content = completion
        .choices
        .first()
        .map(|choice| choice.message.content.trim())
        .filter(|content| !content.is_empty())
        .ok_or_else(|| "DeepSeek 没有返回脚本内容".to_string())?;

    Ok(GenerateNodeScriptResponse {
        script: extract_script(content),
    })
}

#[tauri::command]
pub async fn run_node_script_debug(
    input: RunNodeScriptDebugRequest,
) -> Result<RunNodeScriptDebugResponse, String> {
    let script = input.script.trim();
    if script.is_empty() {
        return Err("没有可运行的脚本内容".to_string());
    }

    let script_path = write_debug_script(script)?;
    let started_at = Instant::now();
    let mut command = TokioCommand::new("/usr/bin/env");
    command.arg("node").arg(&script_path);

    if let Some(directory) = input.working_directory.as_deref().map(str::trim) {
        if !directory.is_empty() {
            command.current_dir(directory);
        }
    }

    let output_result = timeout(Duration::from_secs(15), command.output()).await;
    let duration_ms = started_at.elapsed().as_millis();
    let _ = std::fs::remove_file(&script_path);

    match output_result {
        Ok(Ok(output)) => Ok(RunNodeScriptDebugResponse {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
            duration_ms,
            timed_out: false,
        }),
        Ok(Err(err)) => Err(format!("运行 Node.js 脚本失败：{err}")),
        Err(_) => Ok(RunNodeScriptDebugResponse {
            stdout: String::new(),
            stderr: "调试运行超过 15 秒，已停止等待。".to_string(),
            exit_code: None,
            duration_ms,
            timed_out: true,
        }),
    }
}

fn deepseek_api_key() -> Result<String, String> {
    let key = keychain_entry()?.get_password().map_err(|err| match err {
        keyring::Error::NoEntry => "还没有配置 DeepSeek API Key，请在 Tick 设置中添加".to_string(),
        _ => "无法读取 macOS 钥匙串，请检查 Tick 的钥匙串访问权限".to_string(),
    })?;
    validate_api_key(&key).map(str::to_string)
}

fn keychain_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .map_err(|_| "无法连接 macOS 钥匙串".to_string())
}

fn validate_api_key(value: &str) -> Result<&str, String> {
    let key = value.trim();
    if key.is_empty() {
        return Err("请输入 DeepSeek API Key".to_string());
    }
    if key.len() < 12 {
        return Err("API Key 看起来过短，请检查后重试".to_string());
    }
    if key.len() > MAX_API_KEY_LEN {
        return Err("API Key 长度异常，请检查后重试".to_string());
    }
    Ok(key)
}

fn mask_api_key(key: &str) -> String {
    let suffix: String = key
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if key.starts_with("sk-") {
        format!("sk-••••{suffix}")
    } else {
        format!("••••{suffix}")
    }
}

async fn send_deepseek_request(
    api_key: &str,
    request: &DeepSeekRequest,
) -> Result<DeepSeekResponse, String> {
    let response = reqwest::Client::new()
        .post(DEEPSEEK_URL)
        .bearer_auth(api_key)
        .json(request)
        .send()
        .await
        .map_err(|err| format!("请求 DeepSeek 失败：{err}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "DeepSeek 返回错误 {status}：{}",
            trim_error_body(&body)
        ));
    }

    response
        .json::<DeepSeekResponse>()
        .await
        .map_err(|err| format!("解析 DeepSeek 响应失败：{err}"))
}

fn write_debug_script(script: &str) -> Result<PathBuf, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| err.to_string())?
        .as_millis();
    let path = std::env::temp_dir().join(format!("tick-debug-{timestamp}.js"));
    std::fs::write(&path, format!("{script}\n")).map_err(|err| err.to_string())?;
    Ok(path)
}

fn system_prompt() -> String {
    [
        "你是 Tick 的 Node.js 脚本助手。",
        "用户会描述一个 macOS 定时任务，你只输出可以直接运行的 Node.js 代码。",
        "要求：",
        "- 只输出 JavaScript 代码，不要 Markdown，不要解释。",
        "- 代码会由 /usr/bin/env node 执行。",
        "- 使用 Node.js 内置模块，避免第三方依赖。",
        "- 可以用 node:child_process 调 macOS 命令，例如 osascript 发通知。",
        "- 需要错误处理，把错误写到 console.error，并设置 process.exitCode = 1。",
        "- 输出必要日志到 console.log，便于 Tick 日志面板查看。",
        "- 不要读取密钥、token、SSH key、浏览器 cookie、钥匙串或私人文件，除非用户明确要求并限定路径。",
    ]
    .join("\n")
}

fn user_prompt(prompt: &str, current_script: Option<&str>) -> String {
    let mut content = format!("用户需求：\n{prompt}");
    if let Some(script) = current_script {
        let script = script.trim();
        if !script.is_empty() {
            content.push_str("\n\n当前脚本，可按需求改写：\n");
            content.push_str(script);
        }
    }
    content
}

fn extract_script(content: &str) -> String {
    let trimmed = content.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }

    let mut lines = trimmed.lines();
    let first = lines.next().unwrap_or_default();
    if !first.starts_with("```") {
        return trimmed.to_string();
    }

    let mut code_lines = Vec::new();
    for line in lines {
        if line.trim_start().starts_with("```") {
            break;
        }
        code_lines.push(line);
    }

    code_lines.join("\n").trim().to_string()
}

fn trim_error_body(body: &str) -> String {
    const MAX_LEN: usize = 500;
    let trimmed = body.trim();
    if trimmed.chars().count() <= MAX_LEN {
        return trimmed.to_string();
    }
    format!("{}...", trimmed.chars().take(MAX_LEN).collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::{mask_api_key, validate_api_key};

    #[test]
    fn validates_api_key_length_without_requiring_a_specific_prefix() {
        assert!(validate_api_key("sk-123456789").is_ok());
        assert!(validate_api_key("another-provider-shaped-key").is_ok());
        assert!(validate_api_key("short").is_err());
    }

    #[test]
    fn masks_all_but_a_safe_hint() {
        assert_eq!(mask_api_key("sk-1234567890"), "sk-••••7890");
        assert_eq!(mask_api_key("abcdefghijkl"), "••••ijkl");
    }
}

use crate::scheduler::models::ScheduledJobInput;
use crate::scheduler::validation::validate_job_input;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::process::Command as TokioCommand;
use tokio::time::{timeout, Duration};

const DEEPSEEK_URL: &str = "https://api.deepseek.com/chat/completions";
const DEEPSEEK_MODEL: &str = "deepseek-chat";
const MAX_API_KEY_LEN: usize = 512;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    deepseek_api_key: Option<String>,
}

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
pub struct GenerateAutomationRequest {
    pub prompt: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationDraft {
    pub job: ScheduledJobInput,
    pub summary: String,
    pub risks: Vec<String>,
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
    match read_settings()?.deepseek_api_key {
        Some(key) if !key.trim().is_empty() => Ok(DeepSeekConfigStatus {
            configured: true,
            masked_hint: Some(mask_api_key(key.trim())),
        }),
        _ => Ok(DeepSeekConfigStatus {
            configured: false,
            masked_hint: None,
        }),
    }
}

#[tauri::command]
pub fn save_deepseek_api_key(
    input: SaveDeepSeekApiKeyRequest,
) -> Result<DeepSeekConfigStatus, String> {
    let key = validate_api_key(&input.api_key)?;
    let mut settings = read_settings()?;
    settings.deepseek_api_key = Some(key.to_string());
    write_settings(&settings)?;

    Ok(DeepSeekConfigStatus {
        configured: true,
        masked_hint: Some(mask_api_key(key)),
    })
}

#[tauri::command]
pub fn delete_deepseek_api_key() -> Result<(), String> {
    let mut settings = read_settings()?;
    settings.deepseek_api_key = None;
    write_settings(&settings)
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
pub async fn generate_automation(
    input: GenerateAutomationRequest,
) -> Result<AutomationDraft, String> {
    let prompt = input.prompt.trim();
    if prompt.is_empty() {
        return Err("先描述你想让电脑自动做什么".to_string());
    }

    let api_key = deepseek_api_key()?;
    let request = DeepSeekRequest {
        model: DEEPSEEK_MODEL.to_string(),
        temperature: 0.15,
        max_tokens: 2600,
        messages: vec![
            DeepSeekMessage {
                role: "system".to_string(),
                content: automation_system_prompt(),
            },
            DeepSeekMessage {
                role: "user".to_string(),
                content: automation_user_prompt(prompt),
            },
        ],
    };
    let completion = send_deepseek_request(&api_key, &request).await?;
    let content = completion
        .choices
        .first()
        .map(|choice| choice.message.content.trim())
        .filter(|content| !content.is_empty())
        .ok_or_else(|| "DeepSeek 没有返回自动化方案".to_string())?;

    let draft = serde_json::from_str::<AutomationDraft>(&extract_json(content))
        .map_err(|_| "DeepSeek 返回的任务格式不完整，请换一种说法再试".to_string())?;
    validate_job_input(&draft.job).map_err(|err| format!("AI 生成的任务无法使用：{err}"))?;
    if draft.job.execution.mode != crate::scheduler::models::ExecutionMode::InlineShell {
        return Err("AI 生成了 Tick 暂不支持自动创建的脚本类型，请重试".to_string());
    }
    validate_native_capabilities(prompt, &draft.job.execution.inline_script)?;
    Ok(draft)
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
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = TokioCommand::new("/usr/bin/env");
        command.arg("node");
        command
    };
    #[cfg(target_os = "windows")]
    let mut command = TokioCommand::new("node.exe");
    command.kill_on_drop(true).arg(&script_path);

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
    let key = read_settings()?
        .deepseek_api_key
        .ok_or_else(|| "还没有配置 DeepSeek API Key，请在 Tick 设置中添加".to_string())?;
    validate_api_key(&key).map(str::to_string)
}

fn settings_path() -> Result<PathBuf, String> {
    dirs::config_dir()
        .map(|path| path.join("com.gavin.tick").join("settings.json"))
        .ok_or_else(|| "无法找到应用配置目录".to_string())
}

fn read_settings() -> Result<AppSettings, String> {
    let path = settings_path()?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let content = std::fs::read_to_string(path).map_err(|_| "无法读取 Tick 设置".to_string())?;
    serde_json::from_str(&content).map_err(|_| "Tick 设置文件格式损坏".to_string())
}

fn write_settings(settings: &AppSettings) -> Result<(), String> {
    let path = settings_path()?;
    let directory = path
        .parent()
        .ok_or_else(|| "无法确定应用配置目录".to_string())?;
    std::fs::create_dir_all(directory).map_err(|_| "无法创建 Tick 设置目录".to_string())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700))
            .map_err(|_| "无法保护 Tick 设置目录".to_string())?;
    }

    let temporary_path = path.with_extension("json.tmp");
    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temporary_path)
        .map_err(|_| "无法写入 Tick 设置".to_string())?;
    let content =
        serde_json::to_vec_pretty(settings).map_err(|_| "无法保存 Tick 设置".to_string())?;
    file.write_all(&content)
        .and_then(|_| file.sync_all())
        .map_err(|_| "无法保存 Tick 设置".to_string())?;
    std::fs::rename(temporary_path, path).map_err(|_| "无法替换 Tick 设置".to_string())
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
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("tick-debug-{}-{timestamp}.js", std::process::id()));
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&path).map_err(|err| err.to_string())?;
    file.write_all(format!("{script}\n").as_bytes())
        .map_err(|err| err.to_string())?;
    Ok(path)
}

#[cfg(target_os = "macos")]
fn automation_system_prompt() -> String {
    [
        "你是 Tick 的 macOS 自动化规划器。把用户需求转换成一个完整 LaunchAgent 任务草稿。",
        "只输出 JSON，不要 Markdown，不要解释。JSON 必须严格符合以下结构：",
        r#"{"job":{"name":"任务名","description":"一句话说明","schedule":{"mode":"calendar","calendar":{"month":null,"day":null,"hour":9,"minute":0,"second":0},"interval":{"seconds":3600}},"execution":{"mode":"inline_shell","inlineScript":"可直接由 Node.js 执行的 JavaScript","scriptPath":"","interpreter":"/usr/bin/env node","arguments":"","workingDirectory":"","environment":[]}},"summary":"这个自动化会做什么","risks":["风险或权限提示"]}"#,
        "规则：",
        "- schedule.mode 只能是 calendar 或 interval。",
        "- 用户没说时间时，默认每天 09:00，并在 risks 中说明这个假设。",
        "- execution.mode 必须是 inline_shell，脚本必须是完整 Node.js JavaScript，只使用内置模块。",
        "- 需要调用 macOS 能力时可用 node:child_process 的 execFile/execFileSync，禁止拼接 shell 命令。",
        "- 用户说“提醒我”“通知我”“完成后告诉我”或“失败时通知”时，必须实际调用 /usr/bin/osascript 的 display notification，让消息出现在 macOS Notification Center。",
        r#"- 通知的实现形态应类似：execFileSync("/usr/bin/osascript", ["-e", "display notification \"喝水时间到了\" with title \"Tick\""])。动态文本必须转义反斜杠和双引号。"#,
        "- console.log/console.error 只写入 Tick 的 stdout/stderr 日志，用于排错；绝不能把它们当成用户提示。",
        "- 用户要求打开网页、文件或应用时，使用 /usr/bin/open。用户要求删除文件时，默认移动到 ~/.Trash，不要调用 rm 永久删除。",
        "- summary 中声称完成的每一项行为，都必须能在 inlineScript 中找到对应实现。",
        "- 文件路径使用用户给出的绝对路径；没给路径时不要猜私人目录，写入 risks 并用安全的占位逻辑。",
        "- 删除操作默认移到废纸篓，不要永久删除；在 risks 中明确列出会修改、移动或联网的行为。",
        "- 不读取钥匙串、SSH key、浏览器 cookie 或其他密钥。",
        "- 脚本要有错误处理、console.log/console.error，并设置 process.exitCode。",
        "- risks 没有风险时返回空数组。job.id 不要输出。",
    ]
    .join("\n")
}

#[cfg(target_os = "windows")]
fn automation_system_prompt() -> String {
    [
        "你是 Tick 的 Windows 自动化规划器。把用户需求转换成一个完整的当前用户计划任务草稿。",
        "只输出 JSON，不要 Markdown，不要解释。JSON 必须严格符合以下结构：",
        r#"{"job":{"name":"任务名","description":"一句话说明","schedule":{"mode":"calendar","calendar":{"month":null,"day":null,"hour":9,"minute":0,"second":0},"interval":{"seconds":3600}},"execution":{"mode":"inline_shell","inlineScript":"可直接由 Node.js 执行的 JavaScript","scriptPath":"","interpreter":"node.exe","arguments":"","workingDirectory":"","environment":[]}},"summary":"这个自动化会做什么","risks":["风险或权限提示"]}"#,
        "规则：",
        "- schedule.mode 只能是 calendar 或 interval；interval.seconds 必须在 60 到 2678400 之间。",
        "- 用户没说时间时，默认每天 09:00，并在 risks 中说明这个假设。",
        "- execution.mode 必须是 inline_shell，脚本必须是完整 Node.js JavaScript，只使用内置模块。",
        "- 调用 Windows 程序时只使用 node:child_process 的 execFile/execFileSync 和参数数组，禁止拼接 cmd、PowerShell 或其他 shell 命令。",
        "- console.log/console.error 只写入 Tick 的 stdout/stderr 日志，绝不能把它们当成用户通知。",
        "- 用户要求打开网页、文件或应用时，可用 explorer.exe 和参数数组。",
        "- 用户要求通知、回收站删除或其他无法仅靠 Node.js 内置模块可靠完成的原生行为时，不要伪造实现；在 risks 中明确写出未实现部分。",
        "- summary 中声称完成的每一项行为，都必须能在 inlineScript 中找到对应实现。",
        "- 文件路径使用用户给出的绝对路径；没给路径时不要猜私人目录，写入 risks 并使用安全的占位逻辑。",
        "- 不读取凭据管理器、SSH key、浏览器 cookie 或其他密钥。",
        "- 脚本要有错误处理、console.log/console.error，并设置 process.exitCode。",
        "- risks 没有风险时返回空数组。job.id 不要输出。",
    ]
    .join("\n")
}

fn automation_user_prompt(prompt: &str) -> String {
    format!(
        "今天是 {}。\n用户想要的自动化：\n{}",
        chrono::Local::now().format("%Y-%m-%d"),
        prompt
    )
}

fn extract_json(content: &str) -> String {
    let trimmed = content.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }
    trimmed
        .lines()
        .skip(1)
        .take_while(|line| !line.trim_start().starts_with("```"))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[cfg(target_os = "macos")]
fn validate_native_capabilities(prompt: &str, script: &str) -> Result<(), String> {
    let asks_for_notification = [
        "提醒我",
        "通知我",
        "发通知",
        "系统通知",
        "完成后告诉我",
        "失败时通知",
    ]
    .iter()
    .any(|keyword| prompt.contains(keyword));
    let implements_notification =
        script.contains("/usr/bin/osascript") && script.contains("display notification");

    if asks_for_notification && !implements_notification {
        return Err("AI 只生成了日志，没有真正调用 macOS 系统通知；请重试生成".to_string());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn validate_native_capabilities(_prompt: &str, _script: &str) -> Result<(), String> {
    Ok(())
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
    use super::{mask_api_key, validate_api_key, validate_native_capabilities};

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

    #[cfg(target_os = "macos")]
    #[test]
    fn rejects_console_log_as_a_notification_substitute() {
        assert!(validate_native_capabilities("每天提醒我喝水", "console.log('喝水')").is_err());
        assert!(validate_native_capabilities(
            "每天提醒我喝水",
            r#"execFileSync("/usr/bin/osascript", ["-e", "display notification \"喝水\""])"#
        )
        .is_ok());
    }
}

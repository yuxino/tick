use crate::file_ops::replace_file;
use crate::scheduler::models::ScheduledJobInput;
use crate::scheduler::validation::validate_job_input;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
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
        response_format: None,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<DeepSeekResponseFormat>,
}

#[derive(Debug, Serialize)]
struct DeepSeekResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
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
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeepSeekResponseMessage {
    content: Option<String>,
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
    let system_prompt = automation_system_prompt();
    let user_prompt = automation_user_prompt(prompt);
    let first_messages = vec![
        DeepSeekMessage {
            role: "system".to_string(),
            content: system_prompt.clone(),
        },
        DeepSeekMessage {
            role: "user".to_string(),
            content: user_prompt.clone(),
        },
    ];
    let first_completion = request_automation_completion(&api_key, first_messages).await?;

    match parse_automation_completion(prompt, &first_completion) {
        Ok(draft) => Ok(draft),
        Err(first_error) => {
            let repair_messages = vec![
                DeepSeekMessage {
                    role: "system".to_string(),
                    content: system_prompt,
                },
                DeepSeekMessage {
                    role: "user".to_string(),
                    content: user_prompt,
                },
                DeepSeekMessage {
                    role: "assistant".to_string(),
                    content: first_completion.content.clone(),
                },
                DeepSeekMessage {
                    role: "user".to_string(),
                    content: automation_repair_prompt(&first_error),
                },
            ];
            let repaired_completion = request_automation_completion(&api_key, repair_messages)
                .await
                .map_err(|err| {
                    format!(
                        "DeepSeek 首次返回的草稿无法使用（{first_error}），自动修复请求也失败：{err}。请检查网络后重试，或点击“手动填写”。"
                    )
                })?;
            parse_automation_completion(prompt, &repaired_completion).map_err(|err| {
                format!(
                    "DeepSeek 自动修复后仍未生成可用草稿：{err}。请点击“手动填写”直接创建任务。"
                )
            })
        }
    }
}

#[derive(Debug)]
struct AutomationCompletion {
    content: String,
    finish_reason: Option<String>,
}

async fn request_automation_completion(
    api_key: &str,
    messages: Vec<DeepSeekMessage>,
) -> Result<AutomationCompletion, String> {
    let request = DeepSeekRequest {
        model: DEEPSEEK_MODEL.to_string(),
        temperature: 0.1,
        max_tokens: 2600,
        response_format: Some(DeepSeekResponseFormat {
            kind: "json_object",
        }),
        messages,
    };
    let completion = send_deepseek_request(api_key, &request).await?;
    let choice = completion
        .choices
        .first()
        .ok_or_else(|| "DeepSeek 响应中没有候选结果".to_string())?;
    Ok(AutomationCompletion {
        content: choice.message.content.clone().unwrap_or_default(),
        finish_reason: choice.finish_reason.clone(),
    })
}

fn parse_automation_completion(
    prompt: &str,
    completion: &AutomationCompletion,
) -> Result<AutomationDraft, String> {
    if completion.finish_reason.as_deref() == Some("length") {
        return Err("输出达到长度上限，JSON 被截断".to_string());
    }
    if completion.content.trim().is_empty() {
        return Err("没有返回任务内容".to_string());
    }

    let mut value = parse_json_document(&completion.content)?;
    normalize_automation_value(prompt, &mut value)?;
    let draft = serde_json::from_value::<AutomationDraft>(value)
        .map_err(|err| format!("任务字段不完整或类型不正确：{err}"))?;
    validate_job_input(&draft.job).map_err(|err| format!("任务参数无效：{err}"))?;
    if draft.job.execution.mode != crate::scheduler::models::ExecutionMode::InlineShell {
        return Err("脚本类型不是 Tick AI 草稿支持的内联 JavaScript".to_string());
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

    let node_command = crate::scheduler::runtime::default_node_command().await?;
    let (node_program, node_arguments) = node_command
        .split_first()
        .ok_or_else(|| "Node.js 检测命令为空".to_string())?;

    let script_path = write_debug_script(script)?;
    let started_at = Instant::now();
    let mut command = TokioCommand::new(node_program);
    command
        .args(node_arguments)
        .kill_on_drop(true)
        .arg(&script_path);

    #[cfg(target_os = "windows")]
    {
        crate::scheduler::runtime::hide_windows_console(&mut command);
        command.env(
            "TICK_EXECUTABLE",
            std::env::current_exe().map_err(|err| format!("无法确定 Tick 程序路径：{err}"))?,
        );
    }

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
    drop(file);

    if let Err(err) = replace_file(&temporary_path, &path, "Tick 设置") {
        let _ = std::fs::remove_file(&temporary_path);
        return Err(err);
    }
    Ok(())
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
        "- JSON 示例中的字段必须全部保留；当前模式用不到的 calendar、interval 和 execution 字段也不能省略。",
        "- “N 分钟/小时后”按固定间隔草稿处理，并在 risks 中明确说明它会重复运行，需要用户在首次运行后手动停用。",
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
        "- JSON 示例中的字段必须全部保留；当前模式用不到的 calendar、interval 和 execution 字段也不能省略。",
        "- “N 分钟/小时后”按固定间隔草稿处理，并在 risks 中明确说明它会重复运行，需要用户在首次运行后手动停用。",
        "- 用户没说时间时，默认每天 09:00，并在 risks 中说明这个假设。",
        "- execution.mode 必须是 inline_shell，脚本必须是完整 Node.js JavaScript，只使用内置模块。",
        "- 调用 Windows 程序时只使用 node:child_process 的 execFile/execFileSync 和参数数组，禁止拼接 cmd、PowerShell 或其他 shell 命令。",
        "- console.log/console.error 只写入 Tick 的 stdout/stderr 日志，绝不能把它们当成用户通知。",
        "- 用户要求打开网页、文件或应用时，可用 explorer.exe 和参数数组。",
        "- Tick 会在任务进程中提供 TICK_EXECUTABLE 环境变量，指向当前 Tick.exe。用户说“提醒我”“提示我”“通知我”“完成后告诉我”或“失败时通知”时，必须调用 Tick 自带的 Windows 原生提示窗口。",
        r#"- Windows 提示的实现形态应类似：execFileSync(process.env.TICK_EXECUTABLE, ["--show-message", "喝水时间到了"], { windowsHide: true })。消息必须作为独立参数传入，不能拼接命令。"#,
        "- 回收站删除或其他 Tick 未提供的原生行为不能伪造；在 risks 中明确写出未实现部分。",
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

fn automation_repair_prompt(error: &str) -> String {
    format!(
        "上一个 JSON 没有通过 Tick 校验，原因：{error}。请保留用户原意，补齐或修正字段，只输出一份完整 JSON。"
    )
}

fn parse_json_document(content: &str) -> Result<Value, String> {
    let trimmed = content.trim().trim_start_matches('\u{feff}').trim();
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return Ok(value);
    }

    let object = extract_first_json_object(trimmed).or_else(|| {
        let start = trimmed.find('{')?;
        Some(&trimmed[start..])
    });
    let object = object.ok_or_else(|| "返回内容里没有找到 JSON 对象".to_string())?;
    serde_json::from_str::<Value>(object).map_err(|err| {
        if err.is_eof() {
            "返回的 JSON 在结束前被截断".to_string()
        } else {
            format!(
                "返回的 JSON 语法错误（第 {} 行第 {} 列）",
                err.line(),
                err.column()
            )
        }
    })
}

fn extract_first_json_object(content: &str) -> Option<&str> {
    let start = content.find('{')?;
    let mut depth = 0_u32;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, character) in content[start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        match character {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    let end = start + offset + character.len_utf8();
                    return Some(&content[start..end]);
                }
            }
            _ => {}
        }
    }
    None
}

fn normalize_automation_value(prompt: &str, value: &mut Value) -> Result<(), String> {
    let root = value
        .as_object_mut()
        .ok_or_else(|| "顶层内容不是 JSON 对象".to_string())?;
    alias_field(root, "job", &["task", "automation"]);

    if !root.contains_key("job")
        && (root.contains_key("schedule") || root.contains_key("execution"))
    {
        let job = Value::Object(root.clone());
        root.insert("job".to_string(), job);
    }
    let inferred_interval = interval_seconds_from_prompt(prompt);
    let relative_time = relative_duration_from_prompt(prompt).is_some();

    let job = root
        .get_mut("job")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "缺少 job 任务对象".to_string())?;
    alias_field(job, "name", &["title", "label"]);
    alias_field(job, "description", &["detail"]);
    if job.get("description").is_none_or(Value::is_null) {
        job.insert("description".to_string(), Value::String(String::new()));
    }
    normalize_schedule(job, inferred_interval)?;
    normalize_execution(job)?;

    if root.get("summary").is_none_or(Value::is_null) {
        root.insert(
            "summary".to_string(),
            Value::String("AI 生成的任务草稿".to_string()),
        );
    }
    normalize_risks(root);
    if relative_time {
        append_risk(
            root,
            "Tick 会把“几分钟/小时后”转换为固定间隔任务，因此会重复运行；如果只需要一次，请在首次运行后停用任务。",
        );
    }
    Ok(())
}

fn normalize_schedule(
    job: &mut Map<String, Value>,
    inferred_interval: Option<u32>,
) -> Result<(), String> {
    job.entry("schedule".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let schedule = job
        .get_mut("schedule")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "job.schedule 不是对象".to_string())?;
    alias_field(schedule, "mode", &["type"]);

    let mode = inferred_interval.map(|_| "interval").or_else(|| {
        schedule
            .get("mode")
            .and_then(Value::as_str)
            .map(canonical_schedule_mode)
    });
    let mode = mode.unwrap_or("calendar");
    schedule.insert("mode".to_string(), Value::String(mode.to_string()));

    if let Some(interval) = schedule.get_mut("interval") {
        if !interval.is_object() {
            let seconds = value_to_u32(interval).unwrap_or_default();
            *interval = serde_json::json!({ "seconds": seconds });
        }
    } else {
        schedule.insert("interval".to_string(), Value::Object(Map::new()));
    }
    let direct_seconds = schedule
        .get("seconds")
        .and_then(value_to_u32)
        .or_else(|| schedule.get("intervalSeconds").and_then(value_to_u32));
    let interval = schedule
        .get_mut("interval")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "job.schedule.interval 不是对象".to_string())?;
    alias_field(interval, "seconds", &["intervalSeconds"]);
    let unit_seconds = interval
        .get("minutes")
        .and_then(value_to_u32)
        .and_then(|value| value.checked_mul(60))
        .or_else(|| {
            interval
                .get("hours")
                .and_then(value_to_u32)
                .and_then(|value| value.checked_mul(3600))
        });
    let seconds = inferred_interval
        .or_else(|| interval.get("seconds").and_then(value_to_u32))
        .or(direct_seconds)
        .or(unit_seconds)
        .unwrap_or(if mode == "interval" { 0 } else { 3600 });
    interval.insert("seconds".to_string(), Value::from(seconds));

    schedule
        .entry("calendar".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let calendar = schedule
        .get_mut("calendar")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "job.schedule.calendar 不是对象".to_string())?;
    for field in ["month", "day"] {
        calendar.entry(field.to_string()).or_insert(Value::Null);
    }
    calendar.entry("hour".to_string()).or_insert_with(|| {
        if mode == "calendar" {
            Value::from(9)
        } else {
            Value::Null
        }
    });
    calendar.entry("minute".to_string()).or_insert_with(|| {
        if mode == "calendar" {
            Value::from(0)
        } else {
            Value::Null
        }
    });
    calendar
        .entry("second".to_string())
        .or_insert_with(|| Value::from(0));
    normalize_numeric_fields(calendar, &["month", "day", "hour", "minute", "second"]);
    Ok(())
}

fn normalize_execution(job: &mut Map<String, Value>) -> Result<(), String> {
    job.entry("execution".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let execution = job
        .get_mut("execution")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "job.execution 不是对象".to_string())?;
    alias_field(
        execution,
        "inlineScript",
        &["inline_script", "script", "code"],
    );
    alias_field(execution, "scriptPath", &["script_path"]);
    alias_field(execution, "workingDirectory", &["working_directory", "cwd"]);
    alias_field(execution, "environment", &["env"]);

    let mode = execution
        .get("mode")
        .and_then(Value::as_str)
        .map(canonical_execution_mode)
        .unwrap_or("inline_shell");
    execution.insert("mode".to_string(), Value::String(mode.to_string()));
    for field in [
        "inlineScript",
        "scriptPath",
        "arguments",
        "workingDirectory",
    ] {
        execution
            .entry(field.to_string())
            .or_insert_with(|| Value::String(String::new()));
    }
    execution
        .entry("interpreter".to_string())
        .or_insert_with(|| Value::String(default_ai_interpreter().to_string()));

    match execution.get_mut("environment") {
        Some(Value::Object(values)) => {
            let entries = std::mem::take(values)
                .into_iter()
                .map(|(key, value)| {
                    serde_json::json!({
                        "key": key,
                        "value": value.as_str().unwrap_or_default()
                    })
                })
                .collect();
            execution.insert("environment".to_string(), Value::Array(entries));
        }
        Some(Value::Array(_)) => {}
        Some(_) | None => {
            execution.insert("environment".to_string(), Value::Array(Vec::new()));
        }
    }
    Ok(())
}

fn normalize_risks(root: &mut Map<String, Value>) {
    match root.get_mut("risks") {
        Some(Value::String(risk)) => {
            let risk = std::mem::take(risk);
            root.insert("risks".to_string(), Value::Array(vec![Value::String(risk)]));
        }
        Some(Value::Array(_)) => {}
        Some(_) | None => {
            root.insert("risks".to_string(), Value::Array(Vec::new()));
        }
    }
}

fn append_risk(root: &mut Map<String, Value>, risk: &str) {
    if let Some(risks) = root.get_mut("risks").and_then(Value::as_array_mut) {
        if !risks.iter().any(|value| value.as_str() == Some(risk)) {
            risks.push(Value::String(risk.to_string()));
        }
    }
}

fn alias_field(object: &mut Map<String, Value>, canonical: &str, aliases: &[&str]) {
    if object.contains_key(canonical) {
        return;
    }
    for alias in aliases {
        if let Some(value) = object.remove(*alias) {
            object.insert(canonical.to_string(), value);
            return;
        }
    }
}

fn canonical_schedule_mode(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "interval" | "fixed_interval" | "repeat" | "recurring" | "relative" | "once" => "interval",
        _ => "calendar",
    }
}

fn canonical_execution_mode(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "inline" | "inline_js" | "javascript" | "node" => "inline_shell",
        "file" => "script_path",
        "script_path" => "script_path",
        "interpreter" => "interpreter",
        _ => "inline_shell",
    }
}

fn normalize_numeric_fields(object: &mut Map<String, Value>, fields: &[&str]) {
    for field in fields {
        if let Some(value) = object.get_mut(*field) {
            if let Some(number) = value_to_u32(value) {
                *value = Value::from(number);
            }
        }
    }
}

fn value_to_u32(value: &Value) -> Option<u32> {
    value
        .as_u64()
        .and_then(|number| u32::try_from(number).ok())
        .or_else(|| value.as_str()?.trim().parse().ok())
}

fn interval_seconds_from_prompt(prompt: &str) -> Option<u32> {
    relative_duration_from_prompt(prompt).or_else(|| repeated_duration_from_prompt(prompt))
}

fn relative_duration_from_prompt(prompt: &str) -> Option<u32> {
    duration_from_prompt(prompt, true)
}

fn repeated_duration_from_prompt(prompt: &str) -> Option<u32> {
    duration_from_prompt(prompt, false)
}

fn duration_from_prompt(prompt: &str, relative: bool) -> Option<u32> {
    for (unit, multiplier) in [("分钟", 60_u32), ("小时", 3600), ("秒", 1), ("天", 86400)] {
        for (index, _) in prompt.match_indices(unit) {
            let prefix = &prompt[..index];
            let suffix = &prompt[index + unit.len()..];
            let Some((number, before_number)) = trailing_ascii_number(prefix) else {
                continue;
            };
            let matches_expression = if relative {
                suffix.trim_start().starts_with('后')
            } else {
                before_number.trim_end().ends_with('每')
            };
            if matches_expression {
                return number.checked_mul(multiplier);
            }
        }
    }
    None
}

fn trailing_ascii_number(value: &str) -> Option<(u32, &str)> {
    let trimmed = value.trim_end();
    let digits_start = trimmed
        .char_indices()
        .rev()
        .take_while(|(_, character)| character.is_ascii_digit())
        .map(|(index, _)| index)
        .last()?;
    let number = trimmed[digits_start..].parse().ok()?;
    Some((number, &trimmed[..digits_start]))
}

#[cfg(target_os = "macos")]
fn default_ai_interpreter() -> &'static str {
    "/usr/bin/env node"
}

#[cfg(target_os = "windows")]
fn default_ai_interpreter() -> &'static str {
    "node.exe"
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
fn validate_native_capabilities(prompt: &str, script: &str) -> Result<(), String> {
    validate_windows_native_capabilities(prompt, script)
}

#[cfg(any(target_os = "windows", test))]
fn validate_windows_native_capabilities(prompt: &str, script: &str) -> Result<(), String> {
    let asks_for_notification = [
        "提醒我",
        "提示我",
        "通知我",
        "发通知",
        "系统通知",
        "完成后告诉我",
        "失败时通知",
    ]
    .iter()
    .any(|keyword| prompt.contains(keyword));
    let implements_notification = script.contains("process.env.TICK_EXECUTABLE")
        && script.contains("--show-message")
        && (script.contains("execFile(") || script.contains("execFileSync("));

    if asks_for_notification && !implements_notification {
        return Err(
            "AI 没有调用 Tick 自带的 Windows 原生提示窗口；正在要求 DeepSeek 修复脚本".to_string(),
        );
    }
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
    #[cfg(target_os = "macos")]
    use super::validate_native_capabilities;
    use super::{
        mask_api_key, parse_automation_completion, parse_json_document, validate_api_key,
        validate_windows_native_capabilities, AutomationCompletion,
    };
    use crate::scheduler::models::{ExecutionMode, ScheduleMode};

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

    #[test]
    fn extracts_json_from_fences_or_surrounding_explanation() {
        let fenced = "```json\n{\"value\":\"brace } in string\"}\n```";
        assert_eq!(
            parse_json_document(fenced).unwrap()["value"],
            "brace } in string"
        );

        let explained = "这是草稿：\n{\"value\": 1}\n请检查。";
        assert_eq!(parse_json_document(explained).unwrap()["value"], 1);
    }

    #[test]
    fn reports_truncated_json_instead_of_a_generic_format_error() {
        let error = parse_json_document("```json\n{\"job\": {").unwrap_err();
        assert!(error.contains("截断"), "{error}");
    }

    #[test]
    fn repairs_common_ai_shape_for_relative_minute_prompt() {
        let completion = AutomationCompletion {
            content: r#"
                Here is the JSON draft:
                {
                  "task": {
                    "title": "一分钟后提示",
                    "schedule": {
                      "type": "calendar",
                      "interval": { "minutes": 5 }
                    },
                    "execution": {
                      "mode": "javascript",
                      "script": "const { execFileSync } = require('node:child_process'); execFileSync(process.env.TICK_EXECUTABLE, ['--show-message', '时间到了'], { windowsHide: true });"
                    }
                  },
                  "summary": "一分钟后执行提示脚本",
                  "risks": "Windows 原生通知需要用户确认实现方式"
                }
            "#
            .to_string(),
            finish_reason: Some("stop".to_string()),
        };

        let draft = parse_automation_completion("1分钟后提示我", &completion).unwrap();
        assert_eq!(draft.job.name, "一分钟后提示");
        assert_eq!(draft.job.schedule.mode, ScheduleMode::Interval);
        assert_eq!(draft.job.schedule.interval.seconds, 60);
        assert_eq!(draft.job.execution.mode, ExecutionMode::InlineShell);
        assert!(draft.job.execution.inline_script.contains("--show-message"));
        assert!(draft.risks.iter().any(|risk| risk.contains("重复运行")));
    }

    #[test]
    fn rejects_completion_cut_off_by_token_limit() {
        let completion = AutomationCompletion {
            content: "{\"job\":".to_string(),
            finish_reason: Some("length".to_string()),
        };
        let error = parse_automation_completion("每天运行", &completion).unwrap_err();
        assert!(error.contains("长度上限"), "{error}");
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

    #[test]
    fn windows_notification_requires_the_tick_native_message_command() {
        assert!(
            validate_windows_native_capabilities("1分钟后提示我", "console.log('时间到了')")
                .is_err()
        );
        assert!(validate_windows_native_capabilities(
            "1分钟后提示我",
            r#"execFileSync(process.env.TICK_EXECUTABLE, ["--show-message", "时间到了"], { windowsHide: true })"#,
        )
        .is_ok());
    }
}

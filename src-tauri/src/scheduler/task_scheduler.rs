use super::models::{JobStatus, ScheduleMode, ScheduledJob};
use super::paths::{task_name, task_uri, TASK_SOURCE};
use super::task_xml::build_task_xml;
use chrono::{Duration, Local};
use windows::core::{Error as WindowsError, Interface, BSTR, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, LocalFree, HANDLE, HLOCAL};
use windows::Win32::Security::Authorization::ConvertStringSidToSidW;
use windows::Win32::Security::{
    EqualSid, GetTokenInformation, LookupAccountNameW, TokenUser, PSID, SID_NAME_USE, TOKEN_QUERY,
    TOKEN_USER,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::TaskScheduler::{
    IExecAction, IRegisteredTask, ITaskFolder, ITaskService, TaskScheduler, TASK_ACTION_EXEC,
    TASK_CREATE, TASK_LOGON_INTERACTIVE_TOKEN, TASK_RUNLEVEL_LUA, TASK_STATE_DISABLED, TASK_UPDATE,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::Win32::System::Variant::VARIANT;

const HRESULT_FILE_NOT_FOUND: i32 = 0x8007_0002_u32 as i32;
const HRESULT_INSUFFICIENT_BUFFER: i32 = 0x8007_007A_u32 as i32;
const SCHED_E_TASK_NOT_FOUND: i32 = 0x8004_130F_u32 as i32;

pub fn save(job: &ScheduledJob) -> Result<(), String> {
    let executable =
        std::env::current_exe().map_err(|err| format!("无法定位 Tick 可执行文件：{err}"))?;
    let xml = build_task_xml(job, &executable, &start_boundary(job))?;
    let id = job.id.clone();
    with_root(move |root, identity| unsafe {
        let name = task_name(&id)?;
        let flags = match root.GetTask(&BSTR::from(name.as_str())) {
            Ok(task) => {
                ensure_owned(&task, &id)?;
                TASK_UPDATE.0
            }
            Err(err) if is_task_missing(&err) => TASK_CREATE.0,
            Err(err) => return Err(format_windows_error("查询 Windows 任务失败", err)),
        };
        let empty = VARIANT::default();
        let user = VARIANT::from(identity.as_str());
        root.RegisterTask(
            &BSTR::from(name.as_str()),
            &BSTR::from(xml.as_str()),
            flags,
            &user,
            &empty,
            TASK_LOGON_INTERACTIVE_TOKEN,
            &empty,
        )
        .map_err(|err| format_windows_error("注册 Windows 任务失败", err))?;
        Ok(())
    })
}

pub fn enable(job: &ScheduledJob) -> Result<(), String> {
    let id = job.id.clone();
    with_root(move |root, _| unsafe {
        let task = get_owned_task(&root, &id)?;
        task.SetEnabled(true.into())
            .map_err(|err| format_windows_error("启用 Windows 任务失败", err))
    })
}

pub fn disable(job: &ScheduledJob) -> Result<(), String> {
    let id = job.id.clone();
    with_root(move |root, _| unsafe {
        let task = get_owned_task(&root, &id)?;
        disable_and_stop(&task)
    })
}

pub fn run_now(job: &ScheduledJob) -> Result<(), String> {
    let id = job.id.clone();
    with_root(move |root, _| unsafe {
        let task = get_owned_task(&root, &id)?;
        let was_enabled = task
            .Enabled()
            .map_err(|err| format_windows_error("读取 Windows 任务状态失败", err))?
            .as_bool();
        if !was_enabled {
            task.SetEnabled(true.into())
                .map_err(|err| format_windows_error("临时启用 Windows 任务失败", err))?;
        }
        let run_result = task
            .Run(&VARIANT::default())
            .map(|_| ())
            .map_err(|err| format_windows_error("立即运行 Windows 任务失败", err));
        let restore_result = if !was_enabled {
            task.SetEnabled(false.into())
                .map_err(|err| format_windows_error("恢复 Windows 任务停用状态失败", err))
        } else {
            Ok(())
        };
        match (run_result, restore_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(run), Ok(())) => Err(run),
            (Ok(()), Err(restore)) => Err(restore),
            (Err(run), Err(restore)) => Err(format!("{run}；{restore}")),
        }
    })
}

pub fn delete(job: &ScheduledJob) -> Result<(), String> {
    let id = job.id.clone();
    with_root(move |root, _| unsafe {
        let name = task_name(&id)?;
        let task = match root.GetTask(&BSTR::from(name.as_str())) {
            Ok(task) => task,
            Err(err) if is_task_missing(&err) => return Ok(()),
            Err(err) => return Err(format_windows_error("查询 Windows 任务失败", err)),
        };
        ensure_owned(&task, &id)?;
        disable_and_stop(&task)?;
        root.DeleteTask(&BSTR::from(name.as_str()), 0)
            .map_err(|err| format_windows_error("删除 Windows 任务失败", err))
    })
}

pub fn status(job: &ScheduledJob) -> Result<JobStatus, String> {
    let id = job.id.clone();
    with_root(move |root, _| unsafe {
        let name = task_name(&id)?;
        let task = match root.GetTask(&BSTR::from(name.as_str())) {
            Ok(task) => task,
            Err(err) if is_task_missing(&err) => return Ok(JobStatus::Missing),
            Err(err) => return Err(format_windows_error("查询 Windows 任务失败", err)),
        };
        ensure_owned(&task, &id)?;
        let enabled = task
            .Enabled()
            .map_err(|err| format_windows_error("读取 Windows 任务状态失败", err))?
            .as_bool();
        Ok(if enabled {
            JobStatus::Enabled
        } else {
            JobStatus::Disabled
        })
    })
}

pub fn read_definition(job: &ScheduledJob) -> Result<String, String> {
    let id = job.id.clone();
    with_root(move |root, _| unsafe {
        let task = get_owned_task(&root, &id)?;
        task.Xml()
            .map(|value| value.to_string())
            .map_err(|err| format_windows_error("读取 Windows 任务定义失败", err))
    })
}

unsafe fn get_owned_task(root: &ITaskFolder, id: &str) -> Result<IRegisteredTask, String> {
    let name = task_name(id)?;
    let task = unsafe { root.GetTask(&BSTR::from(name.as_str())) }
        .map_err(|err| format_windows_error("找不到 Windows 任务", err))?;
    unsafe { ensure_owned(&task, id)? };
    Ok(task)
}

unsafe fn ensure_owned(task: &IRegisteredTask, id: &str) -> Result<(), String> {
    let executable =
        std::env::current_exe().map_err(|err| format!("无法定位 Tick 可执行文件：{err}"))?;
    let expected_executable = executable
        .to_str()
        .ok_or_else(|| "Tick 可执行文件路径不是有效 Unicode".to_string())?;
    let expected_working_directory = executable
        .parent()
        .and_then(|path| path.to_str())
        .ok_or_else(|| "无法确定 Tick 工作目录".to_string())?;

    let definition = unsafe { task.Definition() }
        .map_err(|err| format_windows_error("无法读取 Windows 任务定义", err))?;
    let registration = unsafe { definition.RegistrationInfo() }
        .map_err(|err| format_windows_error("无法读取 Windows 任务来源", err))?;
    let mut source = BSTR::new();
    unsafe { registration.Source(&mut source) }
        .map_err(|err| format_windows_error("无法读取 Windows 任务来源", err))?;
    let mut uri = BSTR::new();
    unsafe { registration.URI(&mut uri) }
        .map_err(|err| format_windows_error("无法读取 Windows 任务 URI", err))?;

    let principal = unsafe { definition.Principal() }
        .map_err(|err| format_windows_error("无法读取 Windows 任务身份", err))?;
    let mut user_id = BSTR::new();
    unsafe { principal.UserId(&mut user_id) }
        .map_err(|err| format_windows_error("无法读取 Windows 任务用户", err))?;
    let mut logon_type = Default::default();
    unsafe { principal.LogonType(&mut logon_type) }
        .map_err(|err| format_windows_error("无法读取 Windows 登录类型", err))?;
    let mut run_level = Default::default();
    unsafe { principal.RunLevel(&mut run_level) }
        .map_err(|err| format_windows_error("无法读取 Windows 运行级别", err))?;

    let actions = unsafe { definition.Actions() }
        .map_err(|err| format_windows_error("无法读取 Windows 任务动作", err))?;
    let mut action_count = 0;
    unsafe { actions.Count(&mut action_count) }
        .map_err(|err| format_windows_error("无法读取 Windows 任务动作数", err))?;
    if action_count != 1 {
        return Err("同名 Windows 任务包含非 Tick 动作，已拒绝修改".to_string());
    }
    let action = unsafe { actions.get_Item(1) }
        .map_err(|err| format_windows_error("无法读取 Windows 任务动作", err))?;
    let mut action_type = Default::default();
    unsafe { action.Type(&mut action_type) }
        .map_err(|err| format_windows_error("无法读取 Windows 任务动作类型", err))?;
    if action_type != TASK_ACTION_EXEC {
        return Err("同名 Windows 任务动作类型不属于 Tick，已拒绝修改".to_string());
    }
    let action = action
        .cast::<IExecAction>()
        .map_err(|err| format_windows_error("无法读取 Windows 执行动作", err))?;
    let mut path = BSTR::new();
    unsafe { action.Path(&mut path) }
        .map_err(|err| format_windows_error("无法读取 Windows 任务程序", err))?;
    let mut arguments = BSTR::new();
    unsafe { action.Arguments(&mut arguments) }
        .map_err(|err| format_windows_error("无法读取 Windows 任务参数", err))?;
    let mut working_directory = BSTR::new();
    unsafe { action.WorkingDirectory(&mut working_directory) }
        .map_err(|err| format_windows_error("无法读取 Windows 任务工作目录", err))?;

    let principal_matches = principal_matches(&user_id.to_string())?;
    let uri_matches = ownership_uri_matches(&uri.to_string(), id)?;
    let expected_arguments = format!("--run-scheduled-job {id}");
    let source_matches = source == TASK_SOURCE;
    let permission_matches =
        logon_type == TASK_LOGON_INTERACTIVE_TOKEN && run_level == TASK_RUNLEVEL_LUA;
    let action_matches = path.to_string().eq_ignore_ascii_case(expected_executable)
        && arguments == expected_arguments
        && working_directory
            .to_string()
            .eq_ignore_ascii_case(expected_working_directory);
    let mut mismatches = Vec::new();
    if !source_matches {
        mismatches.push("来源标记");
    }
    if !uri_matches {
        mismatches.push("任务标识");
    }
    if !principal_matches {
        mismatches.push("运行账户");
    }
    if !permission_matches {
        mismatches.push("权限设置");
    }
    if !action_matches {
        mismatches.push("执行程序或参数");
    }
    if mismatches.is_empty() {
        return Ok(());
    }
    Err(format!(
        "同名 Windows 任务的{}与 Tick 记录不一致，为保护现有任务已拒绝修改。请在 Windows 任务计划程序中检查任务 {}，确认无用后移除冲突任务，或改用新的任务名称后重试。",
        mismatches.join("、"),
        task_name(id)?
    ))
}

fn principal_matches(registered_identity: &str) -> Result<bool, String> {
    let registered_identity = registered_identity.trim();
    if registered_identity.is_empty() {
        return Ok(false);
    }
    let expected =
        current_process_user_sid().map_err(|err| format!("无法验证当前 Windows 账户：{err}"))?;
    let registered = match resolve_account_sid(registered_identity) {
        Ok(sid) => sid,
        Err(_) => return Ok(false),
    };
    Ok(unsafe { EqualSid(expected.as_psid(), registered.as_psid()).is_ok() })
}

fn current_process_user_sid() -> Result<ResolvedSid, String> {
    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }
        .map_err(|err| format_windows_error("打开当前 Windows 进程令牌失败", err))?;
    let _token = OwnedHandle(token);

    let mut token_bytes = 0;
    let probe = unsafe { GetTokenInformation(token, TokenUser, None, 0, &mut token_bytes) };
    match probe {
        Err(err) if err.code().0 == HRESULT_INSUFFICIENT_BUFFER => {}
        Err(err) => return Err(format_windows_error("查询当前 Windows 用户 SID 失败", err)),
        Ok(()) => return Err("Windows 进程令牌未返回用户 SID 缓冲区大小".to_string()),
    }
    if token_bytes == 0 {
        return Err("Windows 进程令牌的用户 SID 为空".to_string());
    }

    let word_size = std::mem::size_of::<usize>() as u32;
    let mut buffer = vec![0usize; token_bytes.div_ceil(word_size) as usize];
    unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            Some(buffer.as_mut_ptr().cast()),
            token_bytes,
            &mut token_bytes,
        )
    }
    .map_err(|err| format_windows_error("读取当前 Windows 用户 SID 失败", err))?;
    if token_bytes < std::mem::size_of::<TOKEN_USER>() as u32 {
        return Err("Windows 进程令牌返回的用户 SID 数据不完整".to_string());
    }
    let token_user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
    let sid = token_user.User.Sid;
    if sid.is_invalid() {
        return Err("Windows 进程令牌返回了无效用户 SID".to_string());
    }
    Ok(ResolvedSid::Account {
        _buffer: buffer,
        sid,
    })
}

fn ownership_uri_matches(registered_uri: &str, id: &str) -> Result<bool, String> {
    let canonical_uri = task_uri(id)?;
    let legacy_uri = format!("tick://{TASK_SOURCE}/{id}");
    Ok(registered_uri == canonical_uri || registered_uri == legacy_uri)
}

enum ResolvedSid {
    Local(PSID),
    Account { _buffer: Vec<usize>, sid: PSID },
}

struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

impl ResolvedSid {
    fn as_psid(&self) -> PSID {
        match self {
            Self::Local(sid) | Self::Account { sid, .. } => *sid,
        }
    }
}

impl Drop for ResolvedSid {
    fn drop(&mut self) {
        if let Self::Local(sid) = self {
            unsafe {
                let _ = LocalFree(Some(HLOCAL(sid.0)));
            }
        }
    }
}

fn resolve_account_sid(identity: &str) -> Result<ResolvedSid, String> {
    let identity = identity.trim();
    if identity.is_empty() {
        return Err("账户标识为空".to_string());
    }
    let identity_wide = BSTR::from(identity);
    if identity
        .as_bytes()
        .get(..2)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"S-"))
    {
        let mut sid = PSID::default();
        unsafe { ConvertStringSidToSidW(PCWSTR(identity_wide.as_ptr()), &mut sid) }
            .map_err(|err| format_windows_error("解析 Windows SID 失败", err))?;
        return Ok(ResolvedSid::Local(sid));
    }

    let mut sid_bytes = 0;
    let mut domain_chars = 0;
    let mut sid_kind = SID_NAME_USE::default();
    let probe = unsafe {
        LookupAccountNameW(
            PCWSTR::null(),
            PCWSTR(identity_wide.as_ptr()),
            None,
            &mut sid_bytes,
            None,
            &mut domain_chars,
            &mut sid_kind,
        )
    };
    match probe {
        Err(err) if err.code().0 == HRESULT_INSUFFICIENT_BUFFER => {}
        Err(err) => return Err(format_windows_error("查询 Windows 账户失败", err)),
        Ok(()) => return Err("Windows 账户查询未返回 SID 缓冲区大小".to_string()),
    }
    if sid_bytes == 0 {
        return Err("Windows 无法解析该账户".to_string());
    }

    let word_size = std::mem::size_of::<usize>() as u32;
    let mut sid_buffer = vec![0usize; sid_bytes.div_ceil(word_size) as usize];
    let sid = PSID(sid_buffer.as_mut_ptr().cast());
    let mut domain = vec![0u16; domain_chars.max(1) as usize];
    unsafe {
        LookupAccountNameW(
            PCWSTR::null(),
            PCWSTR(identity_wide.as_ptr()),
            Some(sid),
            &mut sid_bytes,
            Some(PWSTR(domain.as_mut_ptr())),
            &mut domain_chars,
            &mut sid_kind,
        )
    }
    .map_err(|err| format_windows_error("解析 Windows 账户失败", err))?;
    Ok(ResolvedSid::Account {
        _buffer: sid_buffer,
        sid,
    })
}

unsafe fn disable_and_stop(task: &IRegisteredTask) -> Result<(), String> {
    unsafe { task.SetEnabled(false.into()) }
        .map_err(|err| format_windows_error("停用 Windows 任务失败", err))?;
    unsafe { stop_instances(task) }
}

unsafe fn stop_instances(task: &IRegisteredTask) -> Result<(), String> {
    if let Err(err) = unsafe { task.Stop(0) } {
        if !unsafe { task_is_settled(task)? } {
            return Err(format_windows_error("停止 Windows 任务失败", err));
        }
    }
    let mut consecutive_settled_checks = 0;
    for _ in 0..40 {
        if unsafe { task_is_settled(task)? } {
            consecutive_settled_checks += 1;
            if consecutive_settled_checks >= 4 {
                return Ok(());
            }
        } else {
            consecutive_settled_checks = 0;
            let _ = unsafe { task.Stop(0) };
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Err("Windows 任务实例未能停止，已保留任务记录供重试".to_string())
}

unsafe fn task_is_settled(task: &IRegisteredTask) -> Result<bool, String> {
    let state = unsafe { task.State() }
        .map_err(|err| format_windows_error("读取 Windows 任务运行状态失败", err))?;
    Ok(state == TASK_STATE_DISABLED && unsafe { running_instance_count(task)? } == 0)
}

unsafe fn running_instance_count(task: &IRegisteredTask) -> Result<i32, String> {
    let instances = unsafe { task.GetInstances(0) }
        .map_err(|err| format_windows_error("查询 Windows 任务进程失败", err))?;
    unsafe { instances.Count() }
        .map_err(|err| format_windows_error("读取 Windows 任务进程数失败", err))
}

fn with_root<T, F>(operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(ITaskFolder, String) -> Result<T, String> + Send + 'static,
{
    std::thread::spawn(move || unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .ok()
            .map_err(|err| format_windows_error("初始化 Windows 任务服务失败", err))?;
        let _guard = ComGuard;
        let service: ITaskService = CoCreateInstance(&TaskScheduler, None, CLSCTX_INPROC_SERVER)
            .map_err(|err| format_windows_error("连接 Windows 任务服务失败", err))?;
        let empty = VARIANT::default();
        service
            .Connect(&empty, &empty, &empty, &empty)
            .map_err(|err| format_windows_error("连接 Windows 任务服务失败", err))?;
        let user = service
            .ConnectedUser()
            .map_err(|err| format_windows_error("读取当前 Windows 用户失败", err))?
            .to_string();
        let domain = service
            .ConnectedDomain()
            .map_err(|err| format_windows_error("读取当前 Windows 域失败", err))?
            .to_string();
        let identity = if domain.is_empty() || user.contains('\\') {
            user
        } else {
            format!(r"{domain}\{user}")
        };
        let root = service
            .GetFolder(&BSTR::from(r"\"))
            .map_err(|err| format_windows_error("打开 Windows 任务根目录失败", err))?;
        operation(root, identity)
    })
    .join()
    .map_err(|_| "Windows 任务服务线程意外退出".to_string())?
}

struct ComGuard;

impl Drop for ComGuard {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

fn is_task_missing(error: &WindowsError) -> bool {
    matches!(
        error.code().0,
        HRESULT_FILE_NOT_FOUND | SCHED_E_TASK_NOT_FOUND
    )
}

fn format_windows_error(context: &str, error: WindowsError) -> String {
    format!("{context}：{error}")
}

fn start_boundary(job: &ScheduledJob) -> String {
    let mut start = Local::now();
    if job.schedule.mode == ScheduleMode::Interval {
        start += Duration::seconds(i64::from(job.schedule.interval.seconds));
        start.format("%Y-%m-%dT%H:%M:%S").to_string()
    } else {
        format!(
            "{}T{:02}:{:02}:{:02}",
            start.format("%Y-%m-%d"),
            job.schedule.calendar.hour.unwrap_or(0),
            job.schedule.calendar.minute.unwrap_or(0),
            job.schedule.calendar.second,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::models::{
        CalendarSchedule, ExecutionMode, IntervalSchedule, JobExecution, JobSchedule,
    };

    struct RegisteredTaskCleanup {
        id: String,
    }

    impl Drop for RegisteredTaskCleanup {
        fn drop(&mut self) {
            let id = self.id.clone();
            let cleanup = with_root(move |root, _| unsafe {
                let name = task_name(&id)?;
                match root.GetTask(&BSTR::from(name.as_str())) {
                    Ok(task) => {
                        let _ = task.SetEnabled(false.into());
                        let _ = task.Stop(0);
                    }
                    Err(err) if is_task_missing(&err) => return Ok(()),
                    Err(err) => {
                        return Err(format_windows_error("查询 Windows 所有权测试任务失败", err))
                    }
                }
                root.DeleteTask(&BSTR::from(name.as_str()), 0)
                    .map_err(|err| format_windows_error("清理 Windows 所有权测试任务失败", err))
            });
            if let Err(err) = cleanup {
                eprintln!("failed to clean up Windows ownership test task: {err}");
            }
        }
    }

    fn ownership_round_trip_job() -> ScheduledJob {
        const DECIMAL_SPACE: u128 = 100_000_000_000_000_000_000;
        let value = uuid::Uuid::new_v4().as_u128() % DECIMAL_SPACE;
        let id = format!("job-{value:020}");
        ScheduledJob {
            label: format!("com.gavin.tick.windows-roundtrip-{value:020}"),
            id,
            name: "Windows ownership round trip".to_string(),
            description: "Validates Task Scheduler normalization".to_string(),
            status: JobStatus::Disabled,
            schedule: JobSchedule {
                mode: ScheduleMode::Interval,
                calendar: CalendarSchedule {
                    month: None,
                    day: None,
                    hour: None,
                    minute: None,
                    second: 0,
                },
                interval: IntervalSchedule { seconds: 2_678_400 },
            },
            execution: JobExecution {
                mode: ExecutionMode::InlineShell,
                inline_script: "console.log('not executed')".to_string(),
                script_path: String::new(),
                interpreter: "node.exe".to_string(),
                arguments: String::new(),
                working_directory: String::new(),
                environment: vec![],
            },
            stdout_path: String::new(),
            stderr_path: String::new(),
            definition_path: String::new(),
            last_modified_at: "2026-08-31T00:00:00Z".to_string(),
        }
    }

    fn register_test_task(job: &ScheduledJob, legacy_uri: bool) -> Result<(), String> {
        let executable =
            std::env::current_exe().map_err(|err| format!("无法定位测试可执行文件：{err}"))?;
        let mut xml = build_task_xml(job, &executable, &start_boundary(job))?;
        if legacy_uri {
            let canonical = format!("<URI>{}</URI>", task_uri(&job.id)?);
            let legacy = format!("<URI>tick://{TASK_SOURCE}/{}</URI>", job.id);
            if !xml.contains(&canonical) {
                return Err("测试任务 XML 缺少 canonical URI".to_string());
            }
            xml = xml.replacen(&canonical, &legacy, 1);
        }

        let id = job.id.clone();
        with_root(move |root, identity| unsafe {
            let name = task_name(&id)?;
            let empty = VARIANT::default();
            let user = VARIANT::from(identity.as_str());
            root.RegisterTask(
                &BSTR::from(name.as_str()),
                &BSTR::from(xml.as_str()),
                TASK_CREATE.0,
                &user,
                &empty,
                TASK_LOGON_INTERACTIVE_TOKEN,
                &empty,
            )
            .map(|_| ())
            .map_err(|err| format_windows_error("注册 Windows 所有权测试任务失败", err))
        })
    }

    #[test]
    fn registered_task_remains_owned_after_windows_normalizes_uri_and_principal() {
        let job = ownership_round_trip_job();
        register_test_task(&job, true).expect("failed to register Windows ownership test task");
        let _cleanup = RegisteredTaskCleanup { id: job.id.clone() };
        assert_eq!(status(&job).unwrap(), JobStatus::Disabled);

        let id = job.id.clone();
        let (connected_identity, registered_principal) = with_root(move |root, identity| unsafe {
            let task = root
                .GetTask(&BSTR::from(task_name(&id)?.as_str()))
                .map_err(|err| format_windows_error("读取 Windows 所有权测试任务失败", err))?;
            let definition = task
                .Definition()
                .map_err(|err| format_windows_error("读取 Windows 所有权测试定义失败", err))?;
            let principal = definition
                .Principal()
                .map_err(|err| format_windows_error("读取 Windows 所有权测试账户失败", err))?;
            let mut user_id = BSTR::new();
            principal
                .UserId(&mut user_id)
                .map_err(|err| format_windows_error("读取 Windows 所有权测试账户 SID 失败", err))?;
            Ok((identity, user_id.to_string()))
        })
        .unwrap();
        assert!(!registered_principal.trim().is_empty());

        let connected_sid = resolve_account_sid(&connected_identity).unwrap();
        let registered_sid = resolve_account_sid(&registered_principal).unwrap();
        assert!(unsafe { EqualSid(connected_sid.as_psid(), registered_sid.as_psid()).is_ok() });
        assert!(principal_matches(&registered_principal).unwrap());

        enable(&job).expect("failed to enable owned Windows task");
        assert_eq!(status(&job).unwrap(), JobStatus::Enabled);
        disable(&job).expect("failed to disable owned Windows task");
        assert_eq!(status(&job).unwrap(), JobStatus::Disabled);

        save(&job).expect("failed to migrate owned Windows task to canonical XML");
        let xml = read_definition(&job).expect("failed to read registered task definition");
        assert!(xml.contains(&format!("<URI>{}</URI>", task_uri(&job.id).unwrap())));
        delete(&job).expect("failed to delete owned Windows task");
        assert_eq!(status(&job).unwrap(), JobStatus::Missing);
    }

    #[test]
    fn ownership_checks_accept_only_known_uris_and_equivalent_principals() {
        let id = "job-1234567890";
        assert!(ownership_uri_matches(r"\Tick.job-1234567890", id).unwrap());
        assert!(ownership_uri_matches("tick://com.gavin.tick/job-1234567890", id).unwrap());
        assert!(!ownership_uri_matches(r"\Other.job-1234567890", id).unwrap());

        let connected_identity = with_root(|_, identity| Ok(identity)).unwrap();
        assert!(principal_matches(&connected_identity).unwrap());
        assert!(!principal_matches("S-1-0-0").unwrap());
        assert!(!principal_matches("not-a-valid-account").unwrap());
        assert!(!principal_matches("").unwrap());
    }
}

use super::models::{JobStatus, ScheduleMode, ScheduledJob};
use super::paths::{task_name, task_uri, TASK_SOURCE};
use super::task_xml::build_task_xml;
use chrono::{Duration, Local};
use windows::core::{Error as WindowsError, Interface, BSTR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::TaskScheduler::{
    IExecAction, IRegisteredTask, ITaskFolder, ITaskService, TaskScheduler, TASK_ACTION_EXEC,
    TASK_CREATE, TASK_LOGON_INTERACTIVE_TOKEN, TASK_RUNLEVEL_LUA, TASK_STATE_DISABLED, TASK_UPDATE,
};
use windows::Win32::System::Variant::VARIANT;

const HRESULT_FILE_NOT_FOUND: i32 = 0x8007_0002_u32 as i32;
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
                ensure_owned(&task, &id, &identity)?;
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
    with_root(move |root, identity| unsafe {
        let task = get_owned_task(&root, &id, &identity)?;
        task.SetEnabled(true.into())
            .map_err(|err| format_windows_error("启用 Windows 任务失败", err))
    })
}

pub fn disable(job: &ScheduledJob) -> Result<(), String> {
    let id = job.id.clone();
    with_root(move |root, identity| unsafe {
        let task = get_owned_task(&root, &id, &identity)?;
        disable_and_stop(&task)
    })
}

pub fn run_now(job: &ScheduledJob) -> Result<(), String> {
    let id = job.id.clone();
    with_root(move |root, identity| unsafe {
        let task = get_owned_task(&root, &id, &identity)?;
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
    with_root(move |root, identity| unsafe {
        let name = task_name(&id)?;
        let task = match root.GetTask(&BSTR::from(name.as_str())) {
            Ok(task) => task,
            Err(err) if is_task_missing(&err) => return Ok(()),
            Err(err) => return Err(format_windows_error("查询 Windows 任务失败", err)),
        };
        ensure_owned(&task, &id, &identity)?;
        disable_and_stop(&task)?;
        root.DeleteTask(&BSTR::from(name.as_str()), 0)
            .map_err(|err| format_windows_error("删除 Windows 任务失败", err))
    })
}

pub fn status(job: &ScheduledJob) -> Result<JobStatus, String> {
    let id = job.id.clone();
    with_root(move |root, identity| unsafe {
        let name = task_name(&id)?;
        let task = match root.GetTask(&BSTR::from(name.as_str())) {
            Ok(task) => task,
            Err(err) if is_task_missing(&err) => return Ok(JobStatus::Missing),
            Err(err) => return Err(format_windows_error("查询 Windows 任务失败", err)),
        };
        ensure_owned(&task, &id, &identity)?;
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
    with_root(move |root, identity| unsafe {
        let task = get_owned_task(&root, &id, &identity)?;
        task.Xml()
            .map(|value| value.to_string())
            .map_err(|err| format_windows_error("读取 Windows 任务定义失败", err))
    })
}

unsafe fn get_owned_task(
    root: &ITaskFolder,
    id: &str,
    identity: &str,
) -> Result<IRegisteredTask, String> {
    let name = task_name(id)?;
    let task = unsafe { root.GetTask(&BSTR::from(name.as_str())) }
        .map_err(|err| format_windows_error("找不到 Windows 任务", err))?;
    unsafe { ensure_owned(&task, id, identity)? };
    Ok(task)
}

unsafe fn ensure_owned(task: &IRegisteredTask, id: &str, identity: &str) -> Result<(), String> {
    let expected_uri = task_uri(id)?;
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

    let expected_arguments = format!("--run-scheduled-job {id}");
    let is_owned = source == TASK_SOURCE
        && uri == expected_uri
        && user_id.to_string().eq_ignore_ascii_case(identity)
        && logon_type == TASK_LOGON_INTERACTIVE_TOKEN
        && run_level == TASK_RUNLEVEL_LUA
        && path.to_string().eq_ignore_ascii_case(expected_executable)
        && arguments == expected_arguments
        && working_directory
            .to_string()
            .eq_ignore_ascii_case(expected_working_directory);
    if is_owned {
        Ok(())
    } else {
        Err("同名 Windows 任务不是 Tick 创建的安全任务，已拒绝修改".to_string())
    }
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

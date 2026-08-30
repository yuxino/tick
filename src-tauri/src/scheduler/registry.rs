use super::models::ScheduledJob;
use super::paths::{
    ensure_dirs, normalize_managed_paths, registry_lock_path, registry_path,
    scheduler_operation_lock_path, validate_job_id,
};
use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub fn read_registry() -> Result<Vec<ScheduledJob>, String> {
    let _lock = acquire_registry_lock(false)?;
    let path = registry_path()?;
    read_registry_unlocked(&path)
}

pub fn upsert_job(mut job: ScheduledJob) -> Result<ScheduledJob, String> {
    normalize_managed_paths(&mut job)?;
    let _lock = acquire_registry_lock(true)?;
    let path = registry_path()?;
    let mut jobs = read_registry_unlocked(&path)?;
    if let Some(existing) = jobs.iter_mut().find(|item| item.id == job.id) {
        *existing = job.clone();
    } else {
        jobs.push(job.clone());
    }
    write_registry_unlocked(&path, &jobs)?;
    Ok(job)
}

pub fn find_job(id: &str) -> Result<ScheduledJob, String> {
    validate_job_id(id)?;
    read_registry()?
        .into_iter()
        .find(|job| job.id == id)
        .ok_or_else(|| "找不到任务".to_string())
}

pub fn delete_job(id: &str) -> Result<(), String> {
    validate_job_id(id)?;
    let _lock = acquire_registry_lock(true)?;
    let path = registry_path()?;
    let jobs = read_registry_unlocked(&path)?
        .into_iter()
        .filter(|job| job.id != id)
        .collect::<Vec<_>>();
    write_registry_unlocked(&path, &jobs)
}

fn acquire_registry_lock(exclusive: bool) -> Result<File, String> {
    acquire_file_lock(registry_lock_path()?, exclusive, "Tick 任务索引")
}

pub fn acquire_scheduler_operation_lock(exclusive: bool) -> Result<File, String> {
    acquire_file_lock(scheduler_operation_lock_path()?, exclusive, "Tick 调度操作")
}

fn acquire_file_lock(path: PathBuf, exclusive: bool, label: &str) -> Result<File, String> {
    ensure_dirs()?;
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let lock = options
        .open(path)
        .map_err(|err| format!("无法打开{label}锁：{err}"))?;
    let result = if exclusive {
        FileExt::lock_exclusive(&lock)
    } else {
        FileExt::lock_shared(&lock)
    };
    result.map_err(|err| format!("无法锁定{label}：{err}"))?;
    Ok(lock)
}

fn read_registry_unlocked(path: &Path) -> Result<Vec<ScheduledJob>, String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(err) => return Err(err.to_string()),
    };
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|err| err.to_string())?;
    if content.trim().is_empty() {
        return Ok(vec![]);
    }
    let mut jobs = serde_json::from_str::<Vec<ScheduledJob>>(&content)
        .map_err(|_| "Tick 任务索引格式损坏".to_string())?;
    for job in &mut jobs {
        normalize_managed_paths(job)?;
    }
    Ok(jobs)
}

fn write_registry_unlocked(path: &Path, jobs: &[ScheduledJob]) -> Result<(), String> {
    let mut normalized = jobs.to_vec();
    for job in &mut normalized {
        normalize_managed_paths(job)?;
    }
    let content = serde_json::to_vec_pretty(&normalized).map_err(|err| err.to_string())?;
    let temporary_path = temporary_registry_path(path)?;
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temporary_path)
        .map_err(|err| format!("无法创建 Tick 任务索引临时文件：{err}"))?;
    if let Err(err) = file.write_all(&content).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = std::fs::remove_file(&temporary_path);
        return Err(format!("无法写入 Tick 任务索引：{err}"));
    }
    drop(file);

    if let Err(err) = replace_registry_file(&temporary_path, path) {
        let _ = std::fs::remove_file(&temporary_path);
        return Err(err);
    }
    Ok(())
}

fn temporary_registry_path(path: &Path) -> Result<PathBuf, String> {
    let file_name = path
        .file_name()
        .ok_or_else(|| "Tick 任务索引路径无效".to_string())?;
    let mut temporary_name = file_name.to_os_string();
    temporary_name.push(".tmp");
    Ok(path.with_file_name(temporary_name))
}

#[cfg(target_os = "macos")]
fn replace_registry_file(temporary_path: &Path, path: &Path) -> Result<(), String> {
    std::fs::rename(temporary_path, path)
        .map_err(|err| format!("无法替换 Tick 任务索引：{err}"))?;
    let parent = path
        .parent()
        .ok_or_else(|| "Tick 任务索引路径无效".to_string())?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|err| format!("无法同步 Tick 任务索引目录：{err}"))
}

#[cfg(target_os = "windows")]
fn replace_registry_file(temporary_path: &Path, path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let temporary_wide = temporary_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let path_wide = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    unsafe {
        MoveFileExW(
            PCWSTR(temporary_wide.as_ptr()),
            PCWSTR(path_wide.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(|err| format!("无法替换 Tick 任务索引：{err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn atomically_replaces_registry_from_the_same_directory() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "tick-registry-replace-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("scheduled-jobs.json");
        let temporary_path = temporary_registry_path(&path).unwrap();
        assert_eq!(temporary_path.parent(), path.parent());
        std::fs::write(&path, b"old").unwrap();
        std::fs::write(&temporary_path, b"new").unwrap();

        replace_registry_file(&temporary_path, &path).unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        assert!(!temporary_path.exists());
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}

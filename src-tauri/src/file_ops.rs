#[cfg(target_os = "macos")]
use std::fs::File;
use std::path::Path;

pub fn remove_file_if_exists(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("无法删除 {}：{err}", path.display())),
    }
}

#[cfg(target_os = "macos")]
pub fn replace_file(temporary_path: &Path, path: &Path, label: &str) -> Result<(), String> {
    std::fs::rename(temporary_path, path).map_err(|err| format!("无法替换 {label}：{err}"))?;
    let directory = path.parent().ok_or_else(|| format!("{label}路径无效"))?;
    File::open(directory)
        .and_then(|directory| directory.sync_all())
        .map_err(|err| format!("无法同步 {label}目录：{err}"))
}

#[cfg(target_os = "windows")]
pub fn replace_file(temporary_path: &Path, path: &Path, label: &str) -> Result<(), String> {
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
    .map_err(|err| format!("无法替换 {label}：{err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn atomically_replaces_from_the_same_directory() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("tick-file-replace-{}-{unique}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("data.json");
        let temporary_path = directory.join("data.json.tmp");
        std::fs::write(&path, b"old").unwrap();
        std::fs::write(&temporary_path, b"new").unwrap();

        replace_file(&temporary_path, &path, "测试文件").unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        assert!(!temporary_path.exists());
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}

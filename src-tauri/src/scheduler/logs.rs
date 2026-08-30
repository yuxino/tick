use serde::Serialize;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobLog {
    pub kind: String,
    pub path: String,
    pub content: String,
    pub size: u64,
    pub truncated: bool,
}

pub fn read_log(kind: &str, path: &Path, max_bytes: u64) -> Result<JobLog, String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(empty_log(kind, path));
        }
        Err(err) => return Err(err.to_string()),
    };
    let size = file.seek(SeekFrom::End(0)).map_err(|err| err.to_string())?;
    let bytes_to_read = size.min(max_bytes).min(i64::MAX as u64);
    file.seek(SeekFrom::End(-(bytes_to_read as i64)))
        .map_err(|err| err.to_string())?;
    let mut bytes = Vec::with_capacity(bytes_to_read as usize);
    file.take(bytes_to_read)
        .read_to_end(&mut bytes)
        .map_err(|err| err.to_string())?;

    Ok(JobLog {
        kind: kind.to_string(),
        path: path.display().to_string(),
        content: String::from_utf8_lossy(&bytes).to_string(),
        size,
        truncated: size > bytes_to_read,
    })
}

fn empty_log(kind: &str, path: &Path) -> JobLog {
    JobLog {
        kind: kind.to_string(),
        path: path.display().to_string(),
        content: String::new(),
        size: 0,
        truncated: false,
    }
}

pub fn clear_log(path: &Path) -> Result<(), String> {
    std::fs::write(path, "").map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_log_path() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("tick-log-tail-{}-{unique}.log", std::process::id()))
    }

    #[test]
    fn reads_only_the_requested_tail() {
        let path = temporary_log_path();
        let mut file = File::create(&path).unwrap();
        file.write_all(b"0123456789").unwrap();
        drop(file);

        let log = read_log("stdout", &path, 4).unwrap();
        assert_eq!(log.content, "6789");
        assert_eq!(log.size, 10);
        assert!(log.truncated);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn missing_log_is_empty() {
        let path = temporary_log_path();
        let log = read_log("stderr", &path, 16).unwrap();
        assert!(log.content.is_empty());
        assert_eq!(log.size, 0);
        assert!(!log.truncated);
    }
}

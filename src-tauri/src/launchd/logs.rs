use serde::Serialize;
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
    if !path.exists() {
        return Ok(JobLog {
            kind: kind.to_string(),
            path: path.display().to_string(),
            content: String::new(),
            size: 0,
            truncated: false,
        });
    }

    let bytes = std::fs::read(path).map_err(|err| err.to_string())?;
    let size = bytes.len() as u64;
    let truncated = size > max_bytes;
    let start = if truncated {
        (size - max_bytes) as usize
    } else {
        0
    };
    let content = String::from_utf8_lossy(&bytes[start..]).to_string();

    Ok(JobLog {
        kind: kind.to_string(),
        path: path.display().to_string(),
        content,
        size,
        truncated,
    })
}

pub fn clear_log(path: &Path) -> Result<(), String> {
    std::fs::write(path, "").map_err(|err| err.to_string())
}

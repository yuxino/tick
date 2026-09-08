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
    // Anchor the tail to the measured size even if the process appends more output.
    file.seek(SeekFrom::Start(size - bytes_to_read))
        .map_err(|err| err.to_string())?;
    let mut bytes = Vec::with_capacity(bytes_to_read as usize);
    file.take(bytes_to_read)
        .read_to_end(&mut bytes)
        .map_err(|err| err.to_string())?;
    let truncated = size > bytes_to_read;
    // The byte limit can split a UTF-8 character. Omit only its leading fragment;
    // invalid bytes elsewhere still use the usual replacement-character decoding.
    let start = if truncated {
        bytes
            .iter()
            .take(3)
            .take_while(|byte| **byte & 0xc0 == 0x80)
            .count()
    } else {
        0
    };

    Ok(JobLog {
        kind: kind.to_string(),
        path: path.display().to_string(),
        content: String::from_utf8_lossy(&bytes[start..]).to_string(),
        size,
        truncated,
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
    fn truncated_tail_starts_at_a_complete_utf8_character() {
        for character in ['é', '中', '😀'] {
            let path = temporary_log_path();
            std::fs::write(&path, format!("prefix{character} tail\n")).unwrap();
            let tails = (1..character.len_utf8())
                .map(|partial_bytes| read_log("stdout", &path, 6 + partial_bytes as u64).unwrap())
                .collect::<Vec<_>>();
            let complete = read_log("stdout", &path, 6 + character.len_utf8() as u64).unwrap();
            std::fs::remove_file(path).unwrap();

            for log in tails {
                assert_eq!(log.content, " tail\n", "cut inside {character}");
                assert!(log.truncated);
            }
            assert_eq!(complete.content, format!("{character} tail\n"));
        }
    }

    #[test]
    fn tail_smaller_than_the_last_character_is_empty() {
        let path = temporary_log_path();
        std::fs::write(&path, "prefix😀").unwrap();
        let log = read_log("stdout", &path, 2).unwrap();
        std::fs::remove_file(path).unwrap();

        assert!(log.content.is_empty());
        assert_eq!(log.size, 10);
        assert!(log.truncated);
    }

    #[test]
    fn keeps_lossy_decoding_for_invalid_bytes_inside_the_log() {
        let path = temporary_log_path();
        std::fs::write(&path, b"prefix\xff tail").unwrap();
        let log = read_log("stderr", &path, 6).unwrap();
        std::fs::remove_file(path).unwrap();

        assert_eq!(log.content, "\u{fffd} tail");
        assert!(log.truncated);
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

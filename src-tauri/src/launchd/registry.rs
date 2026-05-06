use super::models::LaunchdJob;
use super::paths::{ensure_dirs, registry_path};

pub fn read_registry() -> Result<Vec<LaunchdJob>, String> {
    ensure_dirs()?;
    let path = registry_path()?;
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
    if content.trim().is_empty() {
        return Ok(vec![]);
    }
    serde_json::from_str(&content).map_err(|err| err.to_string())
}

pub fn write_registry(jobs: &[LaunchdJob]) -> Result<(), String> {
    ensure_dirs()?;
    let content = serde_json::to_string_pretty(jobs).map_err(|err| err.to_string())?;
    std::fs::write(registry_path()?, content).map_err(|err| err.to_string())
}

pub fn upsert_job(job: LaunchdJob) -> Result<LaunchdJob, String> {
    let mut jobs = read_registry()?;
    if let Some(existing) = jobs.iter_mut().find(|item| item.id == job.id) {
        *existing = job.clone();
    } else {
        jobs.push(job.clone());
    }
    write_registry(&jobs)?;
    Ok(job)
}

pub fn find_job(id: &str) -> Result<LaunchdJob, String> {
    read_registry()?
        .into_iter()
        .find(|job| job.id == id)
        .ok_or_else(|| "Job not found".to_string())
}

pub fn delete_job(id: &str) -> Result<(), String> {
    let jobs = read_registry()?
        .into_iter()
        .filter(|job| job.id != id)
        .collect::<Vec<_>>();
    write_registry(&jobs)
}

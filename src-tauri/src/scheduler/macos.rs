use super::executor;
use super::launchctl;
use super::models::{JobStatus, ScheduledJob};
use super::paths::definition_path;
use super::plist_writer::{remove_job_files, write_job_files};

pub fn save(job: &ScheduledJob) -> Result<(), String> {
    write_job_files(job)?;
    if job.status == JobStatus::Enabled {
        let path = definition_path(&job.id, &job.label)?;
        if launchctl::is_loaded(&job.label)? {
            launchctl::bootout(&job.label, &path)?;
        }
        launchctl::bootstrap(&path)?;
    }
    Ok(())
}

pub fn enable(job: &ScheduledJob) -> Result<(), String> {
    write_job_files(job)?;
    let path = definition_path(&job.id, &job.label)?;
    if launchctl::is_loaded(&job.label)? {
        launchctl::bootout(&job.label, &path)?;
    }
    launchctl::bootstrap(&path)
}

pub fn disable(job: &ScheduledJob) -> Result<(), String> {
    let path = definition_path(&job.id, &job.label)?;
    if launchctl::is_loaded(&job.label)? {
        launchctl::bootout(&job.label, &path)?;
    }
    Ok(())
}

pub fn run_now(job: &ScheduledJob) -> Result<(), String> {
    executor::spawn_detached(job)
}

pub fn delete(job: &ScheduledJob) -> Result<(), String> {
    disable(job)?;
    remove_job_files(job)
}

pub fn status(job: &ScheduledJob) -> Result<JobStatus, String> {
    if !definition_path(&job.id, &job.label)?.exists() {
        Ok(JobStatus::Missing)
    } else if launchctl::is_loaded(&job.label)? {
        Ok(JobStatus::Enabled)
    } else {
        Ok(JobStatus::Disabled)
    }
}

pub fn read_definition(job: &ScheduledJob) -> Result<String, String> {
    std::fs::read_to_string(definition_path(&job.id, &job.label)?).map_err(|err| err.to_string())
}

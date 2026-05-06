use std::path::PathBuf;

pub const LABEL_PREFIX: &str = "com.gavin.tick.";

pub fn home_dir() -> Result<PathBuf, String> {
    dirs::home_dir().ok_or_else(|| "无法定位用户主目录".to_string())
}

pub fn launch_agents_dir() -> Result<PathBuf, String> {
    Ok(home_dir()?.join("Library").join("LaunchAgents"))
}

pub fn app_data_dir() -> Result<PathBuf, String> {
    dirs::data_dir()
        .map(|dir| dir.join("tick"))
        .ok_or_else(|| "无法定位应用数据目录".to_string())
}

pub fn registry_path() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("launchd-jobs.json"))
}

pub fn scripts_dir() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("scripts"))
}

pub fn logs_dir() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("logs"))
}

pub fn wrappers_dir() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("wrappers"))
}

pub fn ensure_dirs() -> Result<(), String> {
    for dir in [
        launch_agents_dir()?,
        app_data_dir()?,
        scripts_dir()?,
        logs_dir()?,
        wrappers_dir()?,
    ] {
        std::fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    }
    Ok(())
}

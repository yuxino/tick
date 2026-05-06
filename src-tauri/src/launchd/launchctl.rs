use std::path::Path;
use std::process::Command;

fn gui_target() -> String {
    let uid = unsafe { libc::getuid() };
    format!("gui/{uid}")
}

fn run_launchctl(args: &[String]) -> Result<String, String> {
    let output = Command::new("launchctl")
        .args(args)
        .output()
        .map_err(|err| err.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

pub fn bootstrap(plist_path: &Path) -> Result<(), String> {
    run_launchctl(&[
        "bootstrap".to_string(),
        gui_target(),
        plist_path.display().to_string(),
    ])
    .map(|_| ())
}

pub fn bootout(label: &str, plist_path: &Path) -> Result<(), String> {
    let target = format!("{}/{}", gui_target(), label);
    run_launchctl(&["bootout".to_string(), target]).or_else(|_| {
        run_launchctl(&[
            "bootout".to_string(),
            gui_target(),
            plist_path.display().to_string(),
        ])
    })?;
    Ok(())
}

pub fn print_job(label: &str) -> Result<String, String> {
    let target = format!("{}/{}", gui_target(), label);
    run_launchctl(&["print".to_string(), target])
}

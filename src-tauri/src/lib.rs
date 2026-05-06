mod launchd;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            launchd::commands::list_launchd_jobs,
            launchd::commands::save_launchd_job,
            launchd::commands::enable_launchd_job,
            launchd::commands::disable_launchd_job,
            launchd::commands::run_launchd_job_now,
            launchd::commands::delete_launchd_job,
            launchd::commands::read_launchd_log,
            launchd::commands::clear_launchd_log,
            launchd::commands::read_launchd_plist,
            launchd::commands::print_launchd_job,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

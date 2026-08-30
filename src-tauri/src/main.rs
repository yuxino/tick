// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let mut arguments = std::env::args().skip(1);
    if arguments.next().as_deref() == Some("--run-scheduled-job") {
        let id = arguments.next();
        let result = id
            .as_deref()
            .filter(|_| arguments.next().is_none())
            .ok_or_else(|| "计划任务 runner 需要且只接受一个任务标识".to_string())
            .and_then(tick_lib::run_scheduled_job_runner);
        let exit_code = match result {
            Ok(code) => code,
            Err(err) => {
                if let Some(id) = id.as_deref() {
                    tick_lib::record_scheduled_job_runner_error(id, &err);
                }
                1
            }
        };
        std::process::exit(exit_code);
    }
    tick_lib::run()
}

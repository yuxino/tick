mod ai;
mod file_ops;
mod scheduler;

use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder},
    AppHandle, Manager, Runtime, WebviewWindow, WindowEvent,
};

fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        show_window(&window);
    }
}

fn show_window<R: Runtime>(window: &WebviewWindow<R>) {
    let _ = window.show();
    if window.is_minimized().unwrap_or(false) {
        let _ = window.unminimize();
    }
    let _ = window.set_focus();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let show = MenuItemBuilder::with_id("show", "显示窗口").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;

            let menu = MenuBuilder::new(app)
                .items(&[&show, &PredefinedMenuItem::separator(app)?, &quit])
                .build()?;

            let icon_data = include_bytes!("../icons/tray-icon.png");
            let img = image::load_from_memory(icon_data).expect("无法加载托盘图标");
            let rgba = img.to_rgba8();
            let (w, h) = (rgba.width(), rgba.height());
            let icon = Image::new_owned(rgba.into_raw(), w, h);

            TrayIconBuilder::new()
                .icon(icon)
                .icon_as_template(cfg!(target_os = "macos"))
                .tooltip("Tick")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        show_main_window(app);
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let is_ready_to_hide = window.is_visible().unwrap_or(false)
                                && !window.is_minimized().unwrap_or(false)
                                && window.is_focused().unwrap_or(false);

                            if is_ready_to_hide {
                                let _ = window.hide();
                            } else {
                                show_window(&window);
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            ai::get_deepseek_config_status,
            ai::save_deepseek_api_key,
            ai::delete_deepseek_api_key,
            ai::test_deepseek_connection,
            ai::generate_automation,
            ai::run_node_script_debug,
            scheduler::commands::get_scheduler_capabilities,
            scheduler::commands::get_node_runtime_status,
            scheduler::commands::list_scheduled_jobs,
            scheduler::commands::save_scheduled_job,
            scheduler::commands::enable_scheduled_job,
            scheduler::commands::disable_scheduled_job,
            scheduler::commands::run_scheduled_job_now,
            scheduler::commands::delete_scheduled_job,
            scheduler::commands::read_scheduled_job_log,
            scheduler::commands::clear_scheduled_job_log,
            scheduler::commands::read_job_definition,
        ])
        .build(tauri::generate_context!())
        .expect("构建 Tauri 应用时出错");

    app.run(|_app, _event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen {
            has_visible_windows,
            ..
        } = _event
        {
            if !has_visible_windows {
                show_main_window(_app);
            }
        }
    });
}

pub fn run_scheduled_job_runner(id: &str) -> Result<i32, String> {
    scheduler::commands::run_scheduled_job_runner(id)
}

pub fn record_scheduled_job_runner_error(id: &str, message: &str) {
    let _ = scheduler::executor::append_runner_error_for_id(id, message);
}

#[cfg(target_os = "windows")]
pub fn show_windows_message(message: &str) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND,
    };

    let message = message.trim();
    if message.is_empty() {
        return Err("Windows 提示内容不能为空".to_string());
    }
    if message.chars().count() > 512 {
        return Err("Windows 提示内容不能超过 512 个字符".to_string());
    }
    let message = std::ffi::OsStr::new(message)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let title = std::ffi::OsStr::new("Tick")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        MessageBoxW(
            None,
            PCWSTR(message.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONINFORMATION | MB_SETFOREGROUND,
        )
    };
    if result.0 == 0 {
        Err(format!(
            "无法显示 Windows 提示：{}",
            windows::core::Error::from_win32()
        ))
    } else {
        Ok(())
    }
}

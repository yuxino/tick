mod ai;
mod launchd;

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
                .icon_as_template(true)
                .tooltip("Tick 定时任务")
                .menu(&menu)
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
            launchd::commands::list_launchd_jobs,
            launchd::commands::save_launchd_job,
            launchd::commands::enable_launchd_job,
            launchd::commands::disable_launchd_job,
            launchd::commands::run_launchd_job_now,
            launchd::commands::delete_launchd_job,
            launchd::commands::read_launchd_log,
            launchd::commands::clear_launchd_log,
            launchd::commands::read_launchd_plist,
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

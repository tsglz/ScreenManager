use chrono::Local;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::{TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use crate::autostart;
use crate::database::{Database, UsageRecord};

pub fn create_tray(app: &AppHandle) -> Result<TrayIcon, tauri::Error> {
    let menu = build_menu(app)?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("Screen Manager")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "today_stats" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                    let _ = window.eval("window.location.hash = '/'");
                }
            }
            "autostart" => {
                let current = autostart::is_autostart_enabled();
                if current {
                    let _ = autostart::disable_autostart();
                } else {
                    let _ = autostart::enable_autostart();
                }
                if let Some(tray) = app.tray_by_id("main") {
                    if let Ok(new_menu) = build_menu(app) {
                        let _ = tray.set_menu(Some(new_menu));
                    }
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                ..
            } = event
            {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)
}

pub fn build_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let open_item = MenuItem::with_id(app, "open", "打开主界面", true, None::<&str>)?;
    let today_item = MenuItem::with_id(app, "today_stats", "今日统计", true, None::<&str>)?;
    let autostart_enabled = autostart::is_autostart_enabled();
    let autostart_item = CheckMenuItem::with_id(
        app,
        "autostart",
        "开机启动",
        autostart_enabled,
        true,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(app, "quit", "退出程序", true, None::<&str>)?;

    Menu::with_items(app, &[&open_item, &today_item, &autostart_item, &quit_item])
}

pub fn hide_window_on_close(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

// 暂时注释掉这个函数，因为它需要访问 MonitorState
// pub fn save_record_and_cleanup(app: &AppHandle) {
// }

use tauri::{
    AppHandle, Manager,
    menu::{Menu, MenuItem},
    tray::{TrayIcon, TrayIconBuilder, TrayIconEvent},
};

pub fn create_tray(app: &AppHandle) -> Result<TrayIcon, tauri::Error> {
    let open_item = MenuItem::with_id(app, "open", "Open Dashboard", true, None::<&str>)?;
    let today_item = MenuItem::with_id(app, "today_stats", "Today Stats", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&open_item, &today_item, &quit_item])?;

    TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Screen Manager")
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "open" | "today_stats" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)
}
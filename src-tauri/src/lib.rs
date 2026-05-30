pub mod database;
pub mod window_monitor;
pub mod autostart;
pub mod tray;
pub mod scheduler;
pub mod update;

pub use database::{UsageRecord, _DailySummary, DateStats, WeekStats, CategoryStats, CURRENT_SCHEMA_VERSION};
pub use window_monitor::WindowInfo;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use tauri_plugin_updater::UpdaterExt;

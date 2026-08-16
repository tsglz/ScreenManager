pub mod database;
pub mod session_aggregator;
pub mod window_monitor;
pub mod autostart;
pub mod tray;
pub mod scheduler;
pub mod update;
pub mod ollama;
pub mod config;

pub use database::{UsageRecord, _DailySummary, DateStats, WeekStats, CategoryStats, CURRENT_SCHEMA_VERSION, Report, ReportListItem, ReportListResult};
pub use window_monitor::WindowInfo;
pub use ollama::{ReportData, GenerateReportParams, generate_and_save_report};
pub use config::{AppConfig, load_config, save_config, apply_proxy_env, set_proxy_at_runtime};

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use tauri_plugin_updater::UpdaterExt;

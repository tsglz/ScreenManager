#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Local;
use std::sync::{Arc, Mutex};
use tauri::{async_runtime, Manager};

//#[cfg(not(any(target_os = "android", target_os = "ios")))]
//use tauri_plugin_updater::UpdaterExt;

use screen_manager_lib::config::{self, AppConfig};
use screen_manager_lib::database::{
    CategoryStats, CleanupResult, Database, DateStats, HourlyHeatmapEntry, Report,
    ReportListResult, StorageStats, UsageRecord, WeekStats,
};
use screen_manager_lib::ollama;
use screen_manager_lib::session_aggregator::{WorkSession, aggregate_sessions};
use screen_manager_lib::scheduler::start_scheduler;
use screen_manager_lib::tray::{create_tray, hide_window_on_close};
use screen_manager_lib::update::{get_current_version};
use screen_manager_lib::window_monitor::{get_foreground_window_info, is_idle, WindowInfo};
use screen_manager_lib::autostart;

struct MonitorState {
    db: Arc<Mutex<Database>>,
    current_window: Option<WindowInfo>,
    current_start_time: Option<chrono::DateTime<Local>>,
}

#[tauri::command]
fn get_today_total_duration(state: tauri::State<'_, Arc<Mutex<MonitorState>>>) -> i64 {
    state.lock().unwrap().db.lock().unwrap().get_today_total_duration().unwrap_or(0)
}

#[tauri::command]
fn get_top_apps_today(state: tauri::State<'_, Arc<Mutex<MonitorState>>>, limit: usize) -> Vec<(String, i64)> {
    state.lock().unwrap().db.lock().unwrap().get_top_apps_today(limit).unwrap_or_default()
}

#[tauri::command]
fn get_hourly_distribution_today(state: tauri::State<'_, Arc<Mutex<MonitorState>>>) -> Vec<(i32, i64)> {
    state.lock().unwrap().db.lock().unwrap().get_hourly_distribution_today().unwrap_or_default()
}

#[tauri::command]
fn get_recent_records(state: tauri::State<'_, Arc<Mutex<MonitorState>>>, limit: usize) -> Vec<UsageRecord> {
    state.lock().unwrap().db.lock().unwrap().get_recent_records(limit).unwrap_or_default()
}

#[tauri::command]
fn is_autostart_enabled() -> bool {
    autostart::is_autostart_enabled()
}

#[tauri::command]
fn set_autostart(enabled: bool) -> bool {
    if enabled {
        autostart::enable_autostart()
    } else {
        autostart::disable_autostart()
    }
}

#[tauri::command]
fn get_daily_total(state: tauri::State<'_, Arc<Mutex<MonitorState>>>, date: String) -> i64 {
    state.lock().unwrap().db.lock().unwrap().get_daily_total(&date).unwrap_or(0)
}

#[tauri::command]
fn get_daily_top_apps(state: tauri::State<'_, Arc<Mutex<MonitorState>>>, date: String, limit: usize) -> Vec<(String, i64)> {
    state.lock().unwrap().db.lock().unwrap().get_daily_top_apps(&date, limit).unwrap_or_default()
}

#[tauri::command]
fn get_daily_all_apps(state: tauri::State<'_, Arc<Mutex<MonitorState>>>, date: String) -> Vec<(String, i64)> {
    state.lock().unwrap().db.lock().unwrap().get_daily_all_apps(&date).unwrap_or_default()
}

#[tauri::command]
fn get_current_week_stats(state: tauri::State<'_, Arc<Mutex<MonitorState>>>) -> WeekStats {
    state.lock().unwrap().db.lock().unwrap().get_current_week_stats().unwrap_or(WeekStats {
        week_start: String::new(),
        week_end: String::new(),
        total_duration: 0,
    })
}

#[tauri::command]
fn get_previous_week_stats(state: tauri::State<'_, Arc<Mutex<MonitorState>>>) -> WeekStats {
    state.lock().unwrap().db.lock().unwrap().get_previous_week_stats().unwrap_or(WeekStats {
        week_start: String::new(),
        week_end: String::new(),
        total_duration: 0,
    })
}

#[tauri::command]
fn get_week_top_apps(state: tauri::State<'_, Arc<Mutex<MonitorState>>>, week_start: String, week_end: String, limit: usize) -> Vec<(String, i64)> {
    state.lock().unwrap().db.lock().unwrap().get_week_top_apps(&week_start, &week_end, limit).unwrap_or_default()
}

#[tauri::command]
fn get_week_daily_stats(state: tauri::State<'_, Arc<Mutex<MonitorState>>>, week_start: String, week_end: String) -> Vec<DateStats> {
    state.lock().unwrap().db.lock().unwrap().get_week_daily_stats(&week_start, &week_end).unwrap_or_default()
}

#[tauri::command]
fn get_current_month_stats(state: tauri::State<'_, Arc<Mutex<MonitorState>>>) -> DateStats {
    state.lock().unwrap().db.lock().unwrap().get_current_month_stats().unwrap_or(DateStats {
        date: String::new(),
        total_duration: 0,
    })
}

#[tauri::command]
fn get_previous_month_stats(state: tauri::State<'_, Arc<Mutex<MonitorState>>>) -> DateStats {
    state.lock().unwrap().db.lock().unwrap().get_previous_month_stats().unwrap_or(DateStats {
        date: String::new(),
        total_duration: 0,
    })
}

#[tauri::command]
fn get_month_top_apps(state: tauri::State<'_, Arc<Mutex<MonitorState>>>, month_start: String, month_end: String, limit: usize) -> Vec<(String, i64)> {
    state.lock().unwrap().db.lock().unwrap().get_month_top_apps(&month_start, &month_end, limit).unwrap_or_default()
}

#[tauri::command]
fn get_month_daily_stats(state: tauri::State<'_, Arc<Mutex<MonitorState>>>, month_start: String, month_end: String) -> Vec<DateStats> {
    state.lock().unwrap().db.lock().unwrap().get_month_daily_stats(&month_start, &month_end).unwrap_or_default()
}

#[tauri::command]
fn get_date_range_stats(state: tauri::State<'_, Arc<Mutex<MonitorState>>>, start_date: String, end_date: String) -> Vec<DateStats> {
    state.lock().unwrap().db.lock().unwrap().get_date_range_stats(&start_date, &end_date).unwrap_or_default()
}

#[tauri::command]
fn get_all_categories(state: tauri::State<'_, Arc<Mutex<MonitorState>>>) -> Vec<(String, String)> {
    state.lock().unwrap().db.lock().unwrap().get_all_categories().unwrap_or_default()
}

#[tauri::command]
fn get_category_for_app(state: tauri::State<'_, Arc<Mutex<MonitorState>>>, process_name: String) -> String {
    state.lock().unwrap().db.lock().unwrap().get_category_for_app(&process_name).unwrap_or_else(|_| "其他".to_string())
}

#[tauri::command]
fn set_app_category(state: tauri::State<'_, Arc<Mutex<MonitorState>>>, process_name: String, category_name: String) -> bool {
    state.lock().unwrap().db.lock().unwrap().set_app_category(&process_name, &category_name).is_ok()
}

#[tauri::command]
fn get_today_category_stats(state: tauri::State<'_, Arc<Mutex<MonitorState>>>) -> Vec<CategoryStats> {
    state.lock().unwrap().db.lock().unwrap().get_today_category_stats().unwrap_or_default()
}

#[tauri::command]
fn get_daily_category_stats(state: tauri::State<'_, Arc<Mutex<MonitorState>>>, date: String) -> Vec<CategoryStats> {
    state.lock().unwrap().db.lock().unwrap().get_daily_category_stats(&date).unwrap_or_default()
}

#[tauri::command]
fn get_week_category_stats(state: tauri::State<'_, Arc<Mutex<MonitorState>>>, week_start: String, week_end: String) -> Vec<CategoryStats> {
    state.lock().unwrap().db.lock().unwrap().get_week_category_stats(&week_start, &week_end).unwrap_or_default()
}

#[tauri::command]
fn get_month_category_stats(state: tauri::State<'_, Arc<Mutex<MonitorState>>>, month_start: String, month_end: String) -> Vec<CategoryStats> {
    state.lock().unwrap().db.lock().unwrap().get_month_category_stats(&month_start, &month_end).unwrap_or_default()
}

#[tauri::command]
fn init_default_categories(state: tauri::State<'_, Arc<Mutex<MonitorState>>>) -> bool {
    state.lock().unwrap().db.lock().unwrap().init_default_categories().is_ok()
}

#[tauri::command]
fn get_app_version() -> String {
    get_current_version()
}

#[tauri::command]
fn get_data_path() -> String {
    screen_manager_lib::update::get_app_data_dir().to_string_lossy().to_string()
}

#[tauri::command]
fn get_weekly_hourly_heatmap(state: tauri::State<'_, Arc<Mutex<MonitorState>>>, days: i64) -> Vec<HourlyHeatmapEntry> {
    state.lock().unwrap().db.lock().unwrap().get_weekly_hourly_heatmap(days).unwrap_or_default()
}

#[tauri::command]
async fn create_and_save_report(
    state: tauri::State<'_, Arc<Mutex<MonitorState>>>,
    report_type: String,
    start_date: String,
    end_date: String,
    model: Option<String>,
) -> Result<(i64, String), String> {
    let params = ollama::GenerateReportParams { report_type, start_date, end_date, model };
    let db = state.lock().unwrap().db.clone();
    ollama::generate_and_save_report(&db, params).await
}

#[tauri::command]
async fn list_ollama_models() -> Result<Vec<ollama::OllamaModelInfo>, String> {
    ollama::list_ollama_models().await
}

#[tauri::command]
async fn probe_ollama_ready(model: Option<String>) -> Result<(), String> {
    // 快速健康检查：Ollama 是否启动 + 目标模型是否存在
    let resolved = ollama::resolve_model(model.as_deref());
    let _ = ollama::probe_ollama_before_generate(&resolved).await?;
    Ok(())
}

// ===== 应用记录 / 工作时间线 =====

#[tauri::command]
fn get_records_for_date_range(
    state: tauri::State<'_, Arc<Mutex<MonitorState>>>,
    start_date: String,
    end_date: String,
    limit: usize,
) -> Vec<UsageRecord> {
    state
        .lock()
        .unwrap()
        .db
        .lock()
        .unwrap()
        .get_records_for_date_range(&start_date, &end_date, limit)
        .unwrap_or_default()
}

#[tauri::command]
fn get_records_between_datetimes(
    state: tauri::State<'_, Arc<Mutex<MonitorState>>>,
    start_iso: String,
    end_iso: String,
    limit: usize,
) -> Vec<UsageRecord> {
    state
        .lock()
        .unwrap()
        .db
        .lock()
        .unwrap()
        .get_records_between_datetimes(&start_iso, &end_iso, limit)
        .unwrap_or_default()
}

#[tauri::command]
fn get_app_usage_between_datetimes(
    state: tauri::State<'_, Arc<Mutex<MonitorState>>>,
    start_iso: String,
    end_iso: String,
) -> Vec<(String, i64)> {
    state
        .lock()
        .unwrap()
        .db
        .lock()
        .unwrap()
        .get_app_usage_between_datetimes(&start_iso, &end_iso)
        .unwrap_or_default()
}

// ===== 存储占用统计 & 过期数据清理 =====

const DEFAULT_CLEANUP_DAYS: i64 = 10;

#[tauri::command]
fn get_storage_stats(
    state: tauri::State<'_, Arc<Mutex<MonitorState>>>,
) -> StorageStats {
    state
        .lock()
        .unwrap()
        .db
        .lock()
        .unwrap()
        .get_storage_stats(DEFAULT_CLEANUP_DAYS)
        .unwrap_or(StorageStats {
            db_file_bytes: 0,
            db_file_path: String::new(),
            table_rows: vec![],
            earliest_record_date: None,
            latest_record_date: None,
            cleanup_cutoff_days: DEFAULT_CLEANUP_DAYS,
            cleanup_cutoff_date: String::new(),
            cleanup_usage_rows: 0,
            cleanup_daily_rows: 0,
        })
}

#[tauri::command]
fn cleanup_expired_records(
    state: tauri::State<'_, Arc<Mutex<MonitorState>>>,
    older_than_days: Option<i64>,
) -> CleanupResult {
    let days = older_than_days
        .map(|d| std::cmp::max(1, d))
        .unwrap_or(DEFAULT_CLEANUP_DAYS);
    state
        .lock()
        .unwrap()
        .db
        .lock()
        .unwrap()
        .cleanup_expired_records(days)
        .unwrap_or(CleanupResult {
            cutoff_date: String::new(),
            cutoff_days: days,
            deleted_usage_rows: 0,
            deleted_daily_rows: 0,
            db_size_before_bytes: 0,
            db_size_after_bytes: 0,
            saved_bytes: 0,
        })
}

#[tauri::command]
fn list_reports(
    state: tauri::State<'_, Arc<Mutex<MonitorState>>>,
    keyword: String,
    filter_type: String,
    filter_period: String,
    page: i64,
    page_size: i64,
) -> ReportListResult {
    state
        .lock()
        .unwrap()
        .db
        .lock()
        .unwrap()
        .list_reports(&keyword, &filter_type, &filter_period, page, page_size)
        .unwrap_or(ReportListResult { items: Vec::new(), total: 0 })
}

#[tauri::command]
fn get_report(
    state: tauri::State<'_, Arc<Mutex<MonitorState>>>,
    id: i64,
) -> Option<Report> {
    state
        .lock()
        .unwrap()
        .db
        .lock()
        .unwrap()
        .get_report(id)
        .ok()
        .flatten()
}

#[tauri::command]
fn delete_report(
    state: tauri::State<'_, Arc<Mutex<MonitorState>>>,
    id: i64,
) -> bool {
    state
        .lock()
        .unwrap()
        .db
        .lock()
        .unwrap()
        .delete_report(id)
        .unwrap_or(false)
}

#[tauri::command]
fn export_report_to_file(
    state: tauri::State<'_, Arc<Mutex<MonitorState>>>,
    id: i64,
    file_path: String,
) -> bool {
    state
        .lock()
        .unwrap()
        .db
        .lock()
        .unwrap()
        .export_report_to_file(id, &file_path)
        .unwrap_or(false)
}

#[tauri::command]
fn get_work_sessions(
    state: tauri::State<'_, Arc<Mutex<MonitorState>>>,
    start_date: String,
    end_date: String,
) -> Vec<WorkSession> {
    let idle_threshold: i64 = state
        .lock()
        .unwrap()
        .db
        .lock()
        .unwrap()
        .get_config_value("session_idle_threshold")
        .and_then(|v| v.parse().ok())
        .unwrap_or(900);
    let switch_threshold: i64 = state
        .lock()
        .unwrap()
        .db
        .lock()
        .unwrap()
        .get_config_value("session_switch_threshold")
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);

    let db = state.lock().unwrap().db.clone();
    let (records, overrides, app_categories) = {
        let dbg = db.lock().unwrap();
        let recs = dbg
            .get_records_for_range_ordered(&start_date, &end_date)
            .unwrap_or_default();
        let ovs = dbg
            .get_overrides_for_range(&start_date, &end_date)
            .unwrap_or_default();
        // 构建进程名(小写)→分类名映射，传给聚合器完成分类→项目归属回退
        let mut cats = std::collections::HashMap::new();
        if let Ok(all) = dbg.get_all_categories() {
            for (p, c) in all {
                cats.insert(p.to_lowercase(), c);
            }
        }
        (recs, ovs, cats)
    };
    aggregate_sessions(records, overrides, app_categories, idle_threshold, switch_threshold)
}

#[tauri::command]
fn set_record_project(
    state: tauri::State<'_, Arc<Mutex<MonitorState>>>,
    record_id: i64,
    project: String,
) -> bool {
    state
        .lock()
        .unwrap()
        .db
        .lock()
        .unwrap()
        .set_record_override(record_id, &project, "user")
        .is_ok()
}

#[tauri::command]
fn clear_record_project(
    state: tauri::State<'_, Arc<Mutex<MonitorState>>>,
    record_id: i64,
) -> bool {
    state
        .lock()
        .unwrap()
        .db
        .lock()
        .unwrap()
        .clear_record_override(record_id)
        .is_ok()
}

#[tauri::command]
fn set_project_by_process_name(
    state: tauri::State<'_, Arc<Mutex<MonitorState>>>,
    process_name: String,
    project: String,
    start_date: String,
    end_date: String,
) -> usize {
    state
        .lock()
        .unwrap()
        .db
        .lock()
        .unwrap()
        .set_project_by_process_name(&process_name, &project, &start_date, &end_date)
        .unwrap_or(0)
}

#[tauri::command]
fn clear_project_by_process_name(
    state: tauri::State<'_, Arc<Mutex<MonitorState>>>,
    process_name: String,
    start_date: String,
    end_date: String,
) -> usize {
    state
        .lock()
        .unwrap()
        .db
        .lock()
        .unwrap()
        .clear_project_by_process_name(&process_name, &start_date, &end_date)
        .unwrap_or(0)
}

#[tauri::command]
fn get_hourly_heatmap_for_range(
    state: tauri::State<'_, Arc<Mutex<MonitorState>>>,
    start_date: String,
    end_date: String,
) -> Vec<HourlyHeatmapEntry> {
    state
        .lock()
        .unwrap()
        .db
        .lock()
        .unwrap()
        .get_hourly_heatmap_for_range(&start_date, &end_date)
        .unwrap_or_default()
}

#[tauri::command]
fn get_app_config() -> AppConfig {
    config::load_config()
}

#[tauri::command]
fn save_app_config(cfg: AppConfig) -> Result<(), String> {
    // 先保存到文件
    config::save_config(&cfg)?;
    // 再将代理实时应用到进程环境变量（对后续所有网络请求立即生效）
    config::set_proxy_at_runtime(cfg.http_proxy.clone());
    Ok(())
}

fn main() {
    // 启动最早期：读取配置并应用代理环境变量（先于任何网络请求）
    let startup_cfg = config::load_config();
    config::apply_proxy_env(&startup_cfg);

    let db = Database::new().expect("Failed to create database");
    db.repair_abnormal_records().ok();
    db.init_default_categories().ok();
    let db_arc = Arc::new(Mutex::new(db));

    let db_for_scheduler = Arc::clone(&db_arc);

    let monitor_state = Arc::new(Mutex::new(MonitorState {
        db: Arc::clone(&db_arc),
        current_window: None,
        current_start_time: None,
    }));

    let monitor_state_clone = Arc::clone(&monitor_state);

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            start_scheduler(db_for_scheduler).await;
        });
    });

    async_runtime::spawn(async move {
        loop {
            let window_info_result = get_foreground_window_info();
            let idle_result = is_idle();

            let should_save = {
                let mut state = monitor_state_clone.lock().unwrap();

                let idle = idle_result.unwrap_or(false);
                if idle {
                    if state.current_window.is_some() {
                        let _ = save_current_record_internal(&mut state);
                    }
                    true
                } else {
                    match window_info_result {
                        Ok(window_info) => {
                            let should_save = match &state.current_window {
                                Some(current) => {
                                    current.process_name != window_info.process_name ||
                                    current.window_title != window_info.window_title
                                }
                                None => true,
                            };

                            if should_save {
                                let _ = save_current_record_internal(&mut state);
                                state.current_window = Some(window_info);
                                state.current_start_time = Some(Local::now());
                            }
                            false
                        }
                        Err(_) => false,
                    }
                }
            };

            if should_save {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            } else {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(monitor_state)
        .setup(|app| {
            create_tray(app.handle())?;

            let state = app.state::<Arc<Mutex<MonitorState>>>();
            let db = &state.lock().unwrap().db;
            if let Err(e) = db.lock().unwrap().generate_missing_daily_summaries(7) {
                eprintln!("[Setup] Failed to generate missing daily summaries: {}", e);
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                hide_window_on_close(window.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_today_total_duration,
            get_top_apps_today,
            get_hourly_distribution_today,
            get_recent_records,
            is_autostart_enabled,
            set_autostart,
            get_daily_total,
            get_daily_top_apps,
            get_daily_all_apps,
            get_current_week_stats,
            get_previous_week_stats,
            get_week_top_apps,
            get_week_daily_stats,
            get_current_month_stats,
            get_previous_month_stats,
            get_month_top_apps,
            get_month_daily_stats,
            get_date_range_stats,
            get_all_categories,
            get_category_for_app,
            set_app_category,
            get_today_category_stats,
            get_daily_category_stats,
            get_week_category_stats,
            get_month_category_stats,
            init_default_categories,
            get_app_version,
            get_data_path,
            get_weekly_hourly_heatmap,
            get_work_sessions,
            set_record_project,
            clear_record_project,
            set_project_by_process_name,
            clear_project_by_process_name,
            get_hourly_heatmap_for_range,
            create_and_save_report,
            list_ollama_models,
            probe_ollama_ready,
            get_records_for_date_range,
            get_records_between_datetimes,
            get_app_usage_between_datetimes,
            get_storage_stats,
            cleanup_expired_records,
            list_reports,
            get_report,
            delete_report,
            export_report_to_file,
            get_app_config,
            save_app_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn save_current_record_internal(state: &mut MonitorState) {
    if let (Some(window), Some(start_time)) = (&state.current_window, state.current_start_time) {
        let end_time = Local::now();
        let duration = (end_time - start_time).num_seconds();

        if duration > 0 {
            let record = UsageRecord {
                id: 0,
                process_name: window.process_name.clone(),
                window_title: window.window_title.clone(),
                start_time: start_time.format("%Y-%m-%dT%H:%M:%S").to_string(),
                end_time: end_time.format("%Y-%m-%dT%H:%M:%S").to_string(),
                duration_seconds: duration,
            };

            let _ = state.db.lock().unwrap().insert_record(&record);
        }

        state.current_window = None;
        state.current_start_time = None;
    }
}

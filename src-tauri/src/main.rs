#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Local;
use std::sync::{Arc, Mutex};
use tauri::{async_runtime, Manager};

//#[cfg(not(any(target_os = "android", target_os = "ios")))]
//use tauri_plugin_updater::UpdaterExt;

mod ollama;

use screen_manager_lib::database::{Database, DateStats, WeekStats, UsageRecord, CategoryStats, HourlyHeatmapEntry};
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
async fn generate_report(state: tauri::State<'_, Arc<Mutex<MonitorState>>>, report_type: String, start_date: String, end_date: String) -> Result<String, String> {
    let params = ollama::GenerateReportParams {
        report_type,
        start_date,
        end_date,
    };
    let db = state.lock().unwrap().db.clone();
    ollama::generate_report(&db, params).await
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
    let (records, overrides) = {
        let dbg = db.lock().unwrap();
        let recs = dbg
            .get_records_for_range_ordered(&start_date, &end_date)
            .unwrap_or_default();
        let ovs = dbg
            .get_overrides_for_range(&start_date, &end_date)
            .unwrap_or_default();
        (recs, ovs)
    };
    aggregate_sessions(records, overrides, idle_threshold, switch_threshold)
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

fn main() {
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
            generate_report,
            get_work_sessions,
            set_record_project,
            clear_record_project,
            get_hourly_heatmap_for_range
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

import { invoke } from '@tauri-apps/api/core'

export interface UsageRecord {
  id: number
  process_name: string
  window_title: string
  start_time: string
  end_time: string
  duration_seconds: number
}

export interface DateStats {
  date: string
  total_duration: number
}

export interface WeekStats {
  week_start: string
  week_end: string
  total_duration: number
}

export interface CategoryStats {
  category_name: string
  duration_seconds: number
}

export interface HourlyHeatmapEntry {
  date: string
  hour: number
  duration_seconds: number
}

export interface ProjectSlice {
  name: string
  seconds: number
}

export interface WorkSession {
  start_time: string
  end_time: string
  total_seconds: number
  main_project: string
  projects: ProjectSlice[]
  record_count: number
  /** 快速切屏标记：会话<5分钟、非锁屏、后面有另一段会话切换 */
  quick_switch?: boolean
}

export interface ReportListItem {
  id: number
  report_type: string
  periodicity: 'daily' | 'weekly' | 'monthly'
  start_date: string
  end_date: string
  title: string
  created_at: string
  updated_at: string
}

export interface Report extends ReportListItem {
  content_md: string
}

export interface ReportListResult {
  items: ReportListItem[]
  total: number
}

export interface AppConfig {
  http_proxy?: string | null
  ollama_model?: string | null
}

export interface OllamaModelInfo {
  name: string
  model: string
  modified_at?: string
  size?: number
  digest?: string
  family?: string
  parameter_size?: string
  quantization_level?: string
  size_human?: string
}

export interface StorageStats {
  db_file_bytes: number
  db_file_path: string
  table_rows: [string, number][]
  earliest_record_date?: string | null
  latest_record_date?: string | null
  cleanup_cutoff_days: number
  cleanup_cutoff_date: string
  cleanup_usage_rows: number
  cleanup_daily_rows: number
}

export interface CleanupResult {
  cutoff_date: string
  cutoff_days: number
  deleted_usage_rows: number
  deleted_daily_rows: number
  db_size_before_bytes: number
  db_size_after_bytes: number
  saved_bytes: number
}

export const api = {
  getTodayTotalDuration: () => invoke<number>('get_today_total_duration'),

  getTopAppsToday: (limit: number) => invoke<[string, number][]>('get_top_apps_today', { limit }),

  getHourlyDistributionToday: () => invoke<[number, number][]>('get_hourly_distribution_today'),

  getRecentRecords: (limit: number) => invoke<UsageRecord[]>('get_recent_records', { limit }),

  isAutostartEnabled: () => invoke<boolean>('is_autostart_enabled'),

  setAutostart: (enabled: boolean) => invoke<boolean>('set_autostart', { enabled }),

  getDailyTotal: (date: string) => invoke<number>('get_daily_total', { date }),

  getDailyTopApps: (date: string, limit: number) => invoke<[string, number][]>('get_daily_top_apps', { date, limit }),

  getDailyAllApps: (date: string) => invoke<[string, number][]>('get_daily_all_apps', { date }),

  getCurrentWeekStats: () => invoke<WeekStats>('get_current_week_stats'),

  getPreviousWeekStats: () => invoke<WeekStats>('get_previous_week_stats'),

  getWeekTopApps: (weekStart: string, weekEnd: string, limit: number) =>
    invoke<[string, number][]>('get_week_top_apps', { weekStart, weekEnd, limit }),

  getWeekDailyStats: (weekStart: string, weekEnd: string) =>
    invoke<DateStats[]>('get_week_daily_stats', { weekStart, weekEnd }),

  getCurrentMonthStats: () => invoke<DateStats>('get_current_month_stats'),

  getPreviousMonthStats: () => invoke<DateStats>('get_previous_month_stats'),

  getMonthTopApps: (monthStart: string, monthEnd: string, limit: number) =>
    invoke<[string, number][]>('get_month_top_apps', { monthStart, monthEnd, limit }),

  getMonthDailyStats: (monthStart: string, monthEnd: string) =>
    invoke<DateStats[]>('get_month_daily_stats', { monthStart, monthEnd }),

  getDateRangeStats: (startDate: string, endDate: string) =>
    invoke<DateStats[]>('get_date_range_stats', { startDate, endDate }),

  getAllCategories: () => invoke<[string, string][]>('get_all_categories'),

  getCategoryForApp: (processName: string) => invoke<string>('get_category_for_app', { processName }),

  setAppCategory: (processName: string, categoryName: string) =>
    invoke<boolean>('set_app_category', { processName, categoryName }),

  getTodayCategoryStats: () => invoke<CategoryStats[]>('get_today_category_stats'),

  getDailyCategoryStats: (date: string) => invoke<CategoryStats[]>('get_daily_category_stats', { date }),

  getWeekCategoryStats: (weekStart: string, weekEnd: string) =>
    invoke<CategoryStats[]>('get_week_category_stats', { weekStart, weekEnd }),

  getMonthCategoryStats: (monthStart: string, monthEnd: string) =>
    invoke<CategoryStats[]>('get_month_category_stats', { monthStart, monthEnd }),

  initDefaultCategories: () => invoke<boolean>('init_default_categories'),

  getAppVersion: () => invoke<string>('get_app_version'),

  getDataPath: () => invoke<string>('get_data_path'),

  getWeeklyHourlyHeatmap: (days: number) => invoke<HourlyHeatmapEntry[]>('get_weekly_hourly_heatmap', { days }),

  getHourlyHeatmapForRange: (startDate: string, endDate: string) =>
    invoke<HourlyHeatmapEntry[]>('get_hourly_heatmap_for_range', { startDate, endDate }),

  getWorkSessions: (startDate: string, endDate: string) =>
    invoke<WorkSession[]>('get_work_sessions', { startDate, endDate }),

  setRecordProject: (recordId: number, project: string) =>
    invoke<boolean>('set_record_project', { recordId, project }),

  clearRecordProject: (recordId: number) =>
    invoke<boolean>('clear_record_project', { recordId }),

  setProjectByProcessName: (processName: string, project: string, startDate: string, endDate: string) =>
    invoke<number>('set_project_by_process_name', { processName, project, startDate, endDate }),

  clearProjectByProcessName: (processName: string, startDate: string, endDate: string) =>
    invoke<number>('clear_project_by_process_name', { processName, startDate, endDate }),

  getRecordsBetweenDatetimes: (startIso: string, endIso: string, limit: number) =>
    invoke<UsageRecord[]>('get_records_between_datetimes', { startIso, endIso, limit }),

  getAppUsageBetweenDatetimes: (startIso: string, endIso: string) =>
    invoke<[string, number][]>('get_app_usage_between_datetimes', { startIso, endIso }),

  createAndSaveReport: (reportType: string, startDate: string, endDate: string, model?: string) =>
    invoke<[number, string]>('create_and_save_report', { reportType, startDate, endDate, model: model ?? null }),

  listOllamaModels: () => invoke<OllamaModelInfo[]>('list_ollama_models'),

  probeOllamaReady: (model?: string) => invoke<void>('probe_ollama_ready', { model: model ?? null }),

  listReports: (keyword: string, filterType: string, filterPeriod: string, page: number, pageSize: number) =>
    invoke<ReportListResult>('list_reports', { keyword, filterType, filterPeriod, page, pageSize }),

  getReport: (id: number) =>
    invoke<Report | null>('get_report', { id }),

  deleteReport: (id: number) =>
    invoke<boolean>('delete_report', { id }),

  exportReportToFile: (id: number, filePath: string) =>
    invoke<boolean>('export_report_to_file', { id, filePath }),

  getAppConfig: () => invoke<AppConfig>('get_app_config'),

  saveAppConfig: (cfg: AppConfig) => invoke<void>('save_app_config', { cfg }),

  getStorageStats: () => invoke<StorageStats>('get_storage_stats'),

  cleanupExpiredRecords: (olderThanDays?: number) =>
    invoke<CleanupResult>('cleanup_expired_records', { olderThanDays: olderThanDays ?? null }),
}

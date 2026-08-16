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

  createAndSaveReport: (reportType: string, startDate: string, endDate: string) =>
    invoke<[number, string]>('create_and_save_report', { reportType, startDate, endDate }),

  listReports: (keyword: string, filterType: string, filterPeriod: string, page: number, pageSize: number) =>
    invoke<ReportListResult>('list_reports', { keyword, filterType, filterPeriod, page, pageSize }),

  getReport: (id: number) =>
    invoke<Report | null>('get_report', { id }),

  deleteReport: (id: number) =>
    invoke<boolean>('delete_report', { id }),

  exportReportToFile: (id: number, filePath: string) =>
    invoke<boolean>('export_report_to_file', { id, filePath }),
}

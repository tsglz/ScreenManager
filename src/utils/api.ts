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
}
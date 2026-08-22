export function formatDuration(seconds: number): string {
  if (seconds === 0) return '0秒'

  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const secs = seconds % 60

  const parts: string[] = []
  if (hours > 0) parts.push(`${hours}小时`)
  if (minutes > 0) parts.push(`${minutes}分钟`)
  if (secs > 0 || parts.length === 0) parts.push(`${secs}秒`)

  return parts.join('')
}

export function formatDurationLong(seconds: number): string {
  if (seconds === 0) return '0 秒'

  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const secs = seconds % 60

  const parts: string[] = []
  if (hours > 0) parts.push(`${hours} 小时`)
  if (minutes > 0) parts.push(`${minutes} 分钟`)
  if (secs > 0 || parts.length === 0) parts.push(`${secs} 秒`)

  return parts.join(' ')
}

export function formatDateTime(dateStr: string): string {
  const date = new Date(dateStr)
  return date.toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
}

export function formatDate(dateStr: string): string {
  const date = new Date(dateStr)
  return date.toLocaleDateString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
  })
}

export function formatFullDate(dateStr: string): string {
  const date = new Date(dateStr)
  return date.toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  })
}

export function formatWeekRange(weekStart: string, weekEnd: string): string {
  const start = new Date(weekStart)
  const end = new Date(weekEnd)
  return `${start.getMonth() + 1}/${start.getDate()} - ${end.getMonth() + 1}/${end.getDate()}`
}

export function toLocalDateString(date: Date): string {
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, '0')
  const day = String(date.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

export function getTodayDate(): string {
  return toLocalDateString(new Date())
}

export function getWeekStart(date: Date = new Date()): string {
  const d = new Date(date)
  const day = d.getDay()
  const diff = d.getDate() - day + (day === 0 ? -6 : 1)
  d.setDate(diff)
  return toLocalDateString(d)
}

export function getWeekEnd(date: Date = new Date()): string {
  const start = new Date(getWeekStart(date))
  start.setDate(start.getDate() + 6)
  return toLocalDateString(start)
}

export function getMonthStart(date: Date = new Date()): string {
  const d = new Date(date)
  d.setDate(1)
  return toLocalDateString(d)
}

export function getMonthEnd(date: Date = new Date()): string {
  const d = new Date(date)
  d.setMonth(d.getMonth() + 1)
  d.setDate(0)
  return toLocalDateString(d)
}

export function getPreviousWeekStart(): string {
  const now = new Date()
  const start = new Date(now)
  start.setDate(now.getDate() - now.getDay() + 1 - 7)
  return toLocalDateString(start)
}

export function getPreviousWeekEnd(): string {
  const now = new Date()
  const end = new Date(now)
  end.setDate(now.getDate() - now.getDay())
  return end.toISOString().split('T')[0]
}

export function getDayNames(): string[] {
  return ['周一', '周二', '周三', '周四', '周五', '周六', '周日']
}

/** 以 1024 为单位把字节数格式化成 B/KB/MB/GB/TB，保留 1~2 位小数。 */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let i = 0
  let n = bytes
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024
    i += 1
  }
  const digits = i === 0 ? 0 : n >= 100 ? 1 : 2
  return `${n.toFixed(digits)} ${units[i]}`
}
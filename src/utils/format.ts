export function formatDuration(seconds: number): string {
  if (seconds === 0) return '0m'

  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)

  if (hours > 0) {
    return `${hours}h ${minutes}m`
  }
  return `${minutes}m`
}

export function formatDurationLong(seconds: number): string {
  if (seconds === 0) return '0 分钟'

  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)

  const parts: string[] = []
  if (hours > 0) parts.push(`${hours} 小时`)
  if (minutes > 0) parts.push(`${minutes} 分钟`)

  return parts.join(' ')
}

export function formatDateTime(dateStr: string): string {
  const date = new Date(dateStr)
  return date.toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
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

export function getTodayDate(): string {
  const now = new Date()
  return now.toISOString().split('T')[0]
}

export function getWeekStart(date: Date = new Date()): string {
  const d = new Date(date)
  const day = d.getDay()
  const diff = d.getDate() - day + (day === 0 ? -6 : 1)
  d.setDate(diff)
  return d.toISOString().split('T')[0]
}

export function getWeekEnd(date: Date = new Date()): string {
  const start = new Date(getWeekStart(date))
  start.setDate(start.getDate() + 6)
  return start.toISOString().split('T')[0]
}

export function getMonthStart(date: Date = new Date()): string {
  const d = new Date(date)
  d.setDate(1)
  return d.toISOString().split('T')[0]
}

export function getMonthEnd(date: Date = new Date()): string {
  const d = new Date(date)
  d.setMonth(d.getMonth() + 1)
  d.setDate(0)
  return d.toISOString().split('T')[0]
}

export function getPreviousWeekStart(): string {
  const now = new Date()
  const start = new Date(now)
  start.setDate(now.getDate() - now.getDay() + 1 - 7)
  return start.toISOString().split('T')[0]
}

export function getPreviousWeekEnd(): string {
  const now = new Date()
  const end = new Date(now)
  end.setDate(now.getDate() - now.getDay() + 7 - 7)
  return end.toISOString().split('T')[0]
}

export function getPreviousMonthStart(): string {
  const now = new Date()
  return new Date(now.getFullYear(), now.getMonth() - 1, 1).toISOString().split('T')[0]
}

export function getPreviousMonthEnd(): string {
  const now = new Date()
  return new Date(now.getFullYear(), now.getMonth(), 0).toISOString().split('T')[0]
}

export function getDayNames(): string[] {
  return ['周一', '周二', '周三', '周四', '周五', '周六', '周日']
}
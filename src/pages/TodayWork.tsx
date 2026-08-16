import { useState, useEffect, useMemo, Fragment } from 'react'
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, Cell } from 'recharts'
import { api, HourlyHeatmapEntry, UsageRecord, CategoryStats } from '../utils/api'
import { formatDuration, formatDateTime } from '../utils/format'
import './TodayWork.css'

const WEEKDAY_LABELS = ['日', '一', '二', '三', '四', '五', '六']

function getHeatLevel(duration: number, max: number): number {
  if (max === 0 || duration === 0) return 0
  const ratio = duration / max
  if (ratio <= 0.15) return 1
  if (ratio <= 0.35) return 2
  if (ratio <= 0.55) return 3
  if (ratio <= 0.75) return 4
  return 5
}

function formatDateShort(dateStr: string): string {
  const d = new Date(dateStr)
  const m = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  return `${m}/${day}`
}

function getWeekdayLabel(dateStr: string): string {
  const d = new Date(dateStr)
  return `周${WEEKDAY_LABELS[d.getDay()]}`
}

const CATEGORY_COLORS: Record<string, string> = {
  '开发工具': 'var(--accent)',
  '浏览器': 'var(--accent-blue)',
  '社交通讯': 'var(--accent-orange)',
  '游戏娱乐': 'var(--accent-red)',
  '办公工具': 'var(--accent-purple)',
  '系统工具': 'var(--text-muted)',
  'ScreenManager': 'var(--accent-red)',
  '其他': 'var(--text-muted)',
}

function TodayWork() {
  const [totalDuration, setTotalDuration] = useState(0)
  const [recordCount, setRecordCount] = useState(0)
  const [topApp, setTopApp] = useState('—')
  const [topAppDuration, setTopAppDuration] = useState(0)
  const [heatmapData, setHeatmapData] = useState<HourlyHeatmapEntry[]>([])
  const [hourlyData, setHourlyData] = useState<[number, number][]>([])
  const [recentRecords, setRecentRecords] = useState<UsageRecord[]>([])
  const [categoryStats, setCategoryStats] = useState<CategoryStats[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    loadData()
    const interval = setInterval(loadData, 30000)
    return () => clearInterval(interval)
  }, [])

  const loadData = async () => {
    try {
      const [total, topApps, records, heatmap, hourly, categories] = await Promise.all([
        api.getTodayTotalDuration(),
        api.getTopAppsToday(1),
        api.getRecentRecords(20),
        api.getWeeklyHourlyHeatmap(7),
        api.getHourlyDistributionToday(),
        api.getTodayCategoryStats(),
      ])
      setTotalDuration(total)
      setTopApp(topApps.length > 0 ? topApps[0][0] : '—')
      setTopAppDuration(topApps.length > 0 ? topApps[0][1] : 0)
      setRecordCount(records.length)
      setHeatmapData(heatmap)
      setHourlyData(hourly)
      setRecentRecords(records)
      setCategoryStats(categories)
    } catch (error) {
      console.error('Failed to load data:', error)
    } finally {
      setLoading(false)
    }
  }

  const todayDateStr = new Date().toISOString().split('T')[0]

  const { sortedDates, grid, maxDuration } = useMemo(() => {
    const dateSet = new Set(heatmapData.map((e) => e.date))
    const dates = Array.from(dateSet).sort()
    const g: Record<string, Record<number, number>> = {}
    for (const date of dates) {
      g[date] = {}
    }
    for (const entry of heatmapData) {
      g[entry.date][entry.hour] = entry.duration_seconds
    }
    const max = Math.max(...heatmapData.map((e) => e.duration_seconds), 1)
    return { sortedDates: dates, grid: g, maxDuration: max }
  }, [heatmapData])

  const hourlyChartData = hourlyData.map(([hour, duration]) => ({
    hour: hour % 3 === 0 ? `${hour}:00` : '',
    duration,
  }))

  const barColors = hourlyData.map(([hour]) => {
    if (hour >= 9 && hour < 18) return 'var(--accent)'
    if (hour >= 18 && hour < 22) return 'var(--accent-purple)'
    if (hour >= 6 && hour < 9) return 'var(--accent-blue)'
    return 'var(--text-muted)'
  })

  const categoryTotal = categoryStats.reduce((sum, c) => sum + c.duration_seconds, 0)

  const today = new Date()
  const greeting = today.getHours() < 6 ? '凌晨好' : today.getHours() < 12 ? '早上好' : today.getHours() < 18 ? '下午好' : '晚上好'

  return (
    <div className="today-work">
      <div className="tw-header">
        <div className="tw-header-left">
          <h1>今日工作</h1>
          <span className="tw-greeting">{greeting}，今天是 {todayDateStr}</span>
        </div>
        <div className="tw-header-right">
          <div className="tw-live-dot" />
          <span className="tw-live-text">实时同步中</span>
        </div>
      </div>

      <div className="tw-content">
        <div className="tw-overview">
          <div className="tw-section-title">
            <span className="tw-section-bar" />
            <h2>工作概览</h2>
          </div>
          <div className="stats-grid">
            <div className="stat-card highlight">
              <div className="stat-icon-wrap">
                <span className="stat-icon">⏱</span>
              </div>
              <div className="stat-body">
                <div className="stat-value">{formatDuration(totalDuration)}</div>
                <div className="stat-label">工作时长</div>
              </div>
            </div>
            <div className="stat-card">
              <div className="stat-icon-wrap blue">
                <span className="stat-icon">📋</span>
              </div>
              <div className="stat-body">
                <div className="stat-value">{recordCount}</div>
                <div className="stat-label">记录条数</div>
              </div>
            </div>
            <div className="stat-card">
              <div className="stat-icon-wrap green">
                <span className="stat-icon">🎯</span>
              </div>
              <div className="stat-body">
                <div className="stat-value app-name">{topApp}</div>
                <div className="stat-label">
                  主要工作{topAppDuration > 0 ? ` · ${formatDuration(topAppDuration)}` : ''}
                </div>
              </div>
            </div>
          </div>
        </div>

        <div className="tw-heatmap-section">
          <div className="tw-section-title">
            <span className="tw-section-bar" />
            <h2>时段记录</h2>
            <span className="heatmap-subtitle">近7天 · 每小时活动分布</span>
          </div>
          {loading ? (
            <div className="empty-state">加载中...</div>
          ) : (
            <div className="heatmap-container">
              <div className="heatmap-scroll">
                <div className="heatmap-grid">
                  <div className="hm-corner" />
                  {Array.from({ length: 24 }, (_, h) => (
                    <div key={`hl-${h}`} className="hm-hour-label">
                      {h % 3 === 0 ? `${h}:00` : ''}
                    </div>
                  ))}
                  {sortedDates.map((date) => {
                    const isToday = date === todayDateStr
                    return (
                      <Fragment key={date}>
                        <div className={`hm-date-label ${isToday ? 'today' : ''}`}>
                          <span className="date-md">{formatDateShort(date)}</span>
                          <span className="date-wd">{getWeekdayLabel(date)}</span>
                        </div>
                        {Array.from({ length: 24 }, (_, h) => {
                          const dur = grid[date]?.[h] ?? 0
                          const level = getHeatLevel(dur, maxDuration)
                          const tooltip = `${date} ${h}:00 — ${dur > 0 ? formatDuration(dur) : '无活动'}`
                          return (
                            <div
                              key={`cell-${date}-${h}`}
                              className={`heat-cell level-${level} ${isToday ? 'today-row' : ''}`}
                              title={tooltip}
                            />
                          )
                        })}
                      </Fragment>
                    )
                  })}
                </div>
              </div>
              <div className="heatmap-legend">
                <span>少</span>
                <span className="legend-cell level-0" />
                <span className="legend-cell level-1" />
                <span className="legend-cell level-2" />
                <span className="legend-cell level-3" />
                <span className="legend-cell level-4" />
                <span className="legend-cell level-5" />
                <span>多</span>
              </div>
            </div>
          )}
        </div>

        <div className="tw-charts-row">
          <div className="tw-chart-card">
            <div className="tw-section-title">
              <span className="tw-section-bar" />
              <h2>今日小时分布</h2>
            </div>
            <ResponsiveContainer width="100%" height={200}>
              <BarChart data={hourlyChartData} barCategoryGap={1}>
                <XAxis dataKey="hour" tick={{ fontSize: 11, fill: 'var(--text-muted)' }} interval={0} />
                <YAxis tickFormatter={(v) => formatDuration(v as number)} tick={{ fontSize: 11, fill: 'var(--text-muted)' }} width={60} />
                <Tooltip
                  formatter={(value) => [formatDuration(value as number), '时长']}
                  contentStyle={{ borderRadius: '8px', border: '1px solid var(--border)', fontSize: '13px', background: 'var(--bg-card)' }}
                />
                <Bar dataKey="duration" radius={[3, 3, 0, 0]}>
                  {hourlyChartData.map((_, index) => (
                    <Cell key={index} fill={barColors[index]} />
                  ))}
                </Bar>
              </BarChart>
            </ResponsiveContainer>
          </div>

          <div className="tw-chart-card">
            <div className="tw-section-title">
              <span className="tw-section-bar" />
              <h2>分类占比</h2>
            </div>
            {categoryStats.length > 0 ? (
              <div className="category-list">
                {categoryStats.slice(0, 6).map((cat) => {
                  const pct = categoryTotal > 0 ? (cat.duration_seconds / categoryTotal) * 100 : 0
                  const color = CATEGORY_COLORS[cat.category_name] || 'var(--text-muted)'
                  return (
                    <div key={cat.category_name} className="category-item">
                      <div className="cat-header">
                        <span className="cat-dot" style={{ background: color }} />
                        <span className="cat-name">{cat.category_name}</span>
                        <span className="cat-pct">{pct.toFixed(1)}%</span>
                        <span className="cat-dur">{formatDuration(cat.duration_seconds)}</span>
                      </div>
                      <div className="cat-bar-track">
                        <div className="cat-bar-fill" style={{ width: `${pct}%`, background: color }} />
                      </div>
                    </div>
                  )
                })}
              </div>
            ) : (
              <div className="empty-state">暂无分类数据</div>
            )}
          </div>
        </div>

        <div className="tw-recent-section">
          <div className="tw-section-title">
            <span className="tw-section-bar" />
            <h2>最近活动</h2>
            <span className="heatmap-subtitle">最新 {recentRecords.length} 条记录</span>
          </div>
          {recentRecords.length > 0 ? (
            <div className="recent-timeline">
              {recentRecords.slice(0, 10).map((record, idx) => (
                <div key={record.id} className="timeline-item">
                  <div className="timeline-dot" style={{ opacity: 1 - idx * 0.07 }} />
                  <div className="timeline-content">
                    <div className="timeline-top">
                      <span className="timeline-app">{record.process_name}</span>
                      <span className="timeline-duration">{formatDuration(record.duration_seconds)}</span>
                    </div>
                    <div className="timeline-bottom">
                      <span className="timeline-title">{record.window_title || '—'}</span>
                      <span className="timeline-time">{formatDateTime(record.start_time)}</span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="empty-state">暂无活动记录</div>
          )}
        </div>
      </div>
    </div>
  )
}

export default TodayWork
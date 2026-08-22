import { useState, useEffect } from 'react'
import { api, CategoryStats } from '../utils/api'
import { formatDuration } from '../utils/format'
import './AppRecords.css'

const PALETTE = [
  '#667eea', '#36c5d0', '#22c55e', '#f6a609', '#ff7676',
  '#a78bfa', '#3b82f6', '#ec4899', '#14b8a6', '#f97316',
  '#8b5cf6', '#06b6d4', '#84cc16', '#eab308', '#f43f5e',
]

const KNOWN_COLORS: Record<string, string> = {
  '开发工具': '#667eea',
  '浏览器': '#36c5d0',
  '社交通讯': '#f6a609',
  '游戏娱乐': '#ff7676',
  '办公工具': '#a78bfa',
  '系统工具': '#6b7280',
  'ScreenManager': '#ef5350',
  '其他': '#94a3b8',
}

function getCategoryColor(name: string, index: number): string {
  return KNOWN_COLORS[name] || PALETTE[index % PALETTE.length]
}

const RANK_COLORS = [
  '#667eea', '#36c5d0', '#22c55e', '#f6a609', '#ff7676',
  '#a78bfa', '#6b7280', '#3b82f6', '#ec4899', '#14b8a6',
]

function todayStr(): string {
  const d = new Date()
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  return `${y}-${m}-${day}`
}

function AppRecords() {
  const [selectedDate, setSelectedDate] = useState<string>(todayStr())
  const [topApps, setTopApps] = useState<[string, number][]>([])
  const [categoryStats, setCategoryStats] = useState<CategoryStats[]>([])
  const [totalDuration, setTotalDuration] = useState(0)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    loadData()
    // 仅当日页面 30s 自动刷新一次，避免历史日期反复请求
    if (selectedDate === todayStr()) {
      const interval = setInterval(loadData, 30000)
      return () => clearInterval(interval)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedDate])

  const loadData = async () => {
    try {
      setLoading(true)
      const [apps, categories, total] = await Promise.all([
        api.getDailyTopApps(selectedDate, 10),
        api.getDailyCategoryStats(selectedDate),
        api.getDailyTotal(selectedDate),
      ])
      setTopApps(apps as [string, number][])
      setCategoryStats(categories)
      setTotalDuration(Number(total) || 0)
    } catch (error) {
      console.error('Failed to load data:', error)
    } finally {
      setLoading(false)
    }
  }

  const maxDuration = topApps.length > 0 ? topApps[0][1] : 1
  const categoryTotal = categoryStats.reduce((sum, c) => sum + c.duration_seconds, 0)

  return (
    <div className="app-records">
      <div className="ar-header">
        <div className="ar-header-left">
          <h1>应用记录</h1>
          <div className="ar-header-row">
            <span className="ar-subtitle">{selectedDate} · 全天使用详情（00:00 – 24:00）</span>
            <div className="ar-date-picker-wrap">
              <label className="ar-date-label">选择日期：</label>
              <input
                className="ar-date-input"
                type="date"
                value={selectedDate}
                max={todayStr()}
                onChange={(e) => setSelectedDate(e.target.value)}
              />
            </div>
          </div>
        </div>
        <div className="ar-summary">
          <div className="ar-summary-item">
            <span className="ar-summary-value">{topApps.length}</span>
            <span className="ar-summary-label">活跃应用</span>
          </div>
          <div className="ar-summary-divider" />
          <div className="ar-summary-item">
            <span className="ar-summary-value">{formatDuration(totalDuration)}</span>
            <span className="ar-summary-label">总时长</span>
          </div>
        </div>
      </div>

      <div className="ar-content">
        {loading ? (
          <div className="empty-state">加载中...</div>
        ) : topApps.length === 0 ? (
          <div className="empty-state">该日期暂无应用使用记录</div>
        ) : (
          <>
            <div className="ar-card">
              <div className="tw-section-title">
                <span className="tw-section-bar" />
                <h2>当日应用 Top 10</h2>
                <span className="heatmap-subtitle">按使用时长排序</span>
              </div>
              <div className="ar-app-list">
                {topApps.map(([name, duration], idx) => {
                  const pct = totalDuration > 0 ? (duration / totalDuration) * 100 : 0
                  const barPct = maxDuration > 0 ? (duration / maxDuration) * 100 : 0
                  const rank = idx + 1
                  return (
                    <div key={name} className="ar-app-item">
                      <div className="ar-rank" style={{ background: RANK_COLORS[idx] }}>
                        {rank}
                      </div>
                      <div className="ar-app-info">
                        <div className="ar-app-header">
                          <span className="ar-app-name">{name}</span>
                          <span className="ar-app-pct">{pct.toFixed(1)}%</span>
                          <span className="ar-app-dur">{formatDuration(duration)}</span>
                        </div>
                        <div className="ar-bar-track">
                          <div
                            className="ar-bar-fill"
                            style={{ width: `${barPct}%`, background: RANK_COLORS[idx] }}
                          />
                        </div>
                      </div>
                    </div>
                  )
                })}
              </div>
            </div>

            {categoryStats.length > 0 && (
              <div className="ar-card">
                <div className="tw-section-title">
                  <span className="tw-section-bar" />
                  <h2>分类统计</h2>
                  <span className="heatmap-subtitle">按应用分类汇总</span>
                </div>
                <div className="ar-cat-grid">
                  {categoryStats.map((cat, catIdx) => {
                    const pct = categoryTotal > 0 ? (cat.duration_seconds / categoryTotal) * 100 : 0
                    const color = getCategoryColor(cat.category_name, catIdx)
                    return (
                      <div key={cat.category_name} className="ar-cat-card">
                        <div className="ar-cat-top">
                          <span className="ar-cat-dot" style={{ background: color }} />
                          <span className="ar-cat-name">{cat.category_name}</span>
                        </div>
                        <div className="ar-cat-value">{formatDuration(cat.duration_seconds)}</div>
                        <div className="ar-cat-pct">{pct.toFixed(1)}%</div>
                        <div className="ar-cat-bar">
                          <div className="ar-cat-bar-fill" style={{ width: `${pct}%`, background: color }} />
                        </div>
                      </div>
                    )
                  })}
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  )
}

export default AppRecords

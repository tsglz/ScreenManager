import { useState, useEffect } from 'react'
import { LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer, Cell, PieChart, Pie } from 'recharts'
import Header from '../components/Header'
import { api, DateStats, CategoryStats } from '../utils/api'
import { formatDuration, getMonthStart, getMonthEnd, formatFullDate } from '../utils/format'
import './Monthly.css'

type MonthPeriod = 'current' | 'previous'

function Monthly() {
  const [period, setPeriod] = useState<MonthPeriod>('current')
  const [monthStats, setMonthStats] = useState<DateStats | null>(null)
  const [topApps, setTopApps] = useState<[string, number][]>([])
  const [dailyStats, setDailyStats] = useState<DateStats[]>([])
  const [categoryStats, setCategoryStats] = useState<CategoryStats[]>([])
  const [monthRange, setMonthRange] = useState({ start: '', end: '' })
  const [viewMode, setViewMode] = useState<'apps' | 'categories'>('apps')

  useEffect(() => {
    loadData()
  }, [period])

  const loadData = async () => {
    try {
      let stats: DateStats
      let start: string
      let end: string

      if (period === 'current') {
        stats = await api.getCurrentMonthStats()
        start = getMonthStart()
        end = getMonthEnd()
      } else {
        stats = await api.getPreviousMonthStats()
        const prevMonth = new Date()
        prevMonth.setMonth(prevMonth.getMonth() - 1)
        start = getMonthStart(prevMonth)
        end = getMonthEnd(prevMonth)
      }

      setMonthStats(stats)
      setMonthRange({ start, end })

      const [apps, daily, categories] = await Promise.all([
        api.getMonthTopApps(start, end, 10),
        api.getMonthDailyStats(start, end),
        api.getMonthCategoryStats(start, end),
      ])
      setTopApps(apps)
      setDailyStats(daily)
      setCategoryStats(categories)
    } catch (error) {
      console.error('Failed to load data:', error)
    }
  }

  const handlePeriodChange = (newPeriod: MonthPeriod) => {
    setPeriod(newPeriod)
  }

  const colors = ['#4fc3f7', '#81c784', '#fff176', '#ffb74d', '#ce93d8', '#90caf9', '#a5d6a7', '#fff59d', '#ffcc80', '#f48fb1']
  const categoryColors: Record<string, string> = {
    '开发工具': '#4fc3f7',
    '浏览器': '#81c784',
    '社交通讯': '#fff176',
    '游戏娱乐': '#ffb74d',
    '办公工具': '#ce93d8',
    '系统工具': '#90caf9',
    'ScreenManager': '#ef5350',
    '其他': '#b0bec5',
  }

  const chartData = dailyStats.map((stat) => {
    const date = new Date(stat.date)
    return {
      date: `${date.getMonth() + 1}/${date.getDate()}`,
      duration: stat.total_duration,
    }
  })

  const appPieData = topApps.map(([name, value], index) => ({
    name,
    value,
    fill: colors[index % colors.length],
  }))

  const categoryPieData = categoryStats.map((cat) => ({
    name: cat.category_name,
    value: cat.duration_seconds,
    fill: categoryColors[cat.category_name] || '#b0bec5',
  }))

  const totalApps = topApps.reduce((sum, [, duration]) => sum + duration, 0)
  const totalCategories = categoryStats.reduce((sum, cat) => sum + cat.duration_seconds, 0)

  const currentList = viewMode === 'apps' ? topApps : categoryStats
  const currentTotal = viewMode === 'apps' ? totalApps : totalCategories
  const currentPieData = viewMode === 'apps' ? appPieData : categoryPieData
  const maxDuration = viewMode === 'apps' ? (topApps[0]?.[1] || 1) : (categoryStats[0]?.duration_seconds || 1)

  return (
    <div className="monthly-page">
      <Header title="月统计" />

      <div className="monthly-content">
        <div className="period-tabs">
          <button
            className={`tab ${period === 'current' ? 'active' : ''}`}
            onClick={() => handlePeriodChange('current')}
          >
            本月
          </button>
          <button
            className={`tab ${period === 'previous' ? 'active' : ''}`}
            onClick={() => handlePeriodChange('previous')}
          >
            上月
          </button>
        </div>

        <div className="month-range">
          {formatFullDate(monthRange.start)} - {formatFullDate(monthRange.end)}
        </div>

        <div className="stats-grid">
          <div className="stat-card highlight">
            <div className="stat-value">{formatDuration(monthStats?.total_duration || 0)}</div>
            <div className="stat-label">本月总时长</div>
          </div>
          <div className="stat-card">
            <div className="stat-value">{dailyStats.length}</div>
            <div className="stat-label">记录天数</div>
          </div>
        </div>

        <div className="charts-row">
          <div className="chart-card">
            <h3>每日趋势</h3>
            {dailyStats.length > 0 ? (
              <ResponsiveContainer width="100%" height={200}>
                <LineChart data={chartData}>
                  <XAxis dataKey="date" tick={{ fontSize: 10 }} interval="preserveStartEnd" />
                  <YAxis tickFormatter={(v) => formatDuration(v as number)} tick={{ fontSize: 11 }} />
                  <Tooltip formatter={(value) => [formatDuration(value as number), '时长']} />
                  <Line
                    type="monotone"
                    dataKey="duration"
                    stroke="#4fc3f7"
                    strokeWidth={2}
                    dot={{ fill: '#4fc3f7', strokeWidth: 2, r: 3 }}
                  />
                </LineChart>
              </ResponsiveContainer>
            ) : (
              <div className="empty-state">暂无数据</div>
            )}
          </div>

          <div className="chart-card">
            <div className="card-header">
              <h3>{viewMode === 'apps' ? '应用占比' : '分类占比'}</h3>
              <div className="view-toggle">
                <button
                  className={`toggle-btn ${viewMode === 'apps' ? 'active' : ''}`}
                  onClick={() => setViewMode('apps')}
                >
                  应用
                </button>
                <button
                  className={`toggle-btn ${viewMode === 'categories' ? 'active' : ''}`}
                  onClick={() => setViewMode('categories')}
                >
                  分类
                </button>
              </div>
            </div>
            {currentPieData.length > 0 ? (
              <ResponsiveContainer width="100%" height={200}>
                <PieChart>
                  <Pie
                    data={currentPieData}
                    dataKey="value"
                    nameKey="name"
                    cx="50%"
                    cy="50%"
                    innerRadius={40}
                    outerRadius={80}
                    paddingAngle={2}
                  >
                    {currentPieData.map((entry, index) => (
                      <Cell key={index} fill={entry.fill} />
                    ))}
                  </Pie>
                  <Tooltip formatter={(value) => [formatDuration(value as number), '时长']} />
                </PieChart>
              </ResponsiveContainer>
            ) : (
              <div className="empty-state">暂无数据</div>
            )}
          </div>
        </div>

        <div className="chart-card full-width">
          <h3>Top 10 {viewMode === 'apps' ? '应用' : '分类'}</h3>
          {currentList.length > 0 ? (
            <div className="apps-list">
              {currentList.map((item, index) => {
                const name = viewMode === 'apps' ? (item as [string, number])[0] : (item as CategoryStats).category_name
                const duration = viewMode === 'apps' ? (item as [string, number])[1] : (item as CategoryStats).duration_seconds
                const fill = viewMode === 'apps'
                  ? colors[index % colors.length]
                  : categoryColors[(item as CategoryStats).category_name] || '#b0bec5'
                return (
                  <div key={index} className="app-item">
                    <div className="app-rank" style={{ backgroundColor: fill }}>
                      {index + 1}
                    </div>
                    <div className="app-info">
                      <span className="app-name">{name}</span>
                      <div className="app-bar-container">
                        <div
                          className="app-bar"
                          style={{
                            width: `${(duration / maxDuration) * 100}%`,
                            backgroundColor: fill,
                          }}
                        />
                      </div>
                    </div>
                    <div className="app-stats">
                      <span className="app-duration">{formatDuration(duration)}</span>
                      <span className="app-percent">
                        {currentTotal > 0 ? ((duration / currentTotal) * 100).toFixed(1) : 0}%
                      </span>
                    </div>
                  </div>
                )
              })}
            </div>
          ) : (
            <div className="empty-state">暂无数据</div>
          )}
        </div>
      </div>
    </div>
  )
}

export default Monthly
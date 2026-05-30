import { useState, useEffect } from 'react'
import { LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer, Cell, PieChart, Pie } from 'recharts'
import Header from '../components/Header'
import { api, DateStats } from '../utils/api'
import { formatDuration, getMonthStart, getMonthEnd, formatFullDate } from '../utils/format'
import './Monthly.css'

type MonthPeriod = 'current' | 'previous'

function Monthly() {
  const [period, setPeriod] = useState<MonthPeriod>('current')
  const [monthStats, setMonthStats] = useState<DateStats | null>(null)
  const [topApps, setTopApps] = useState<[string, number][]>([])
  const [dailyStats, setDailyStats] = useState<DateStats[]>([])
  const [monthRange, setMonthRange] = useState({ start: '', end: '' })

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

      const [apps, daily] = await Promise.all([
        api.getMonthTopApps(start, end, 10),
        api.getMonthDailyStats(start, end),
      ])
      setTopApps(apps)
      setDailyStats(daily)
    } catch (error) {
      console.error('Failed to load data:', error)
    }
  }

  const handlePeriodChange = (newPeriod: MonthPeriod) => {
    setPeriod(newPeriod)
  }

  const colors = ['#4fc3f7', '#81c784', '#fff176', '#ffb74d', '#ce93d8', '#90caf9', '#a5d6a7', '#fff59d', '#ffcc80', '#f48fb1']

  const chartData = dailyStats.map((stat) => {
    const date = new Date(stat.date)
    return {
      date: `${date.getMonth() + 1}/${date.getDate()}`,
      duration: stat.total_duration,
    }
  })

  const pieData = topApps.map(([name, value], index) => ({
    name,
    value,
    fill: colors[index % colors.length],
  }))

  const totalApps = topApps.reduce((sum, [, duration]) => sum + duration, 0)

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
            <h3>应用占比</h3>
            {pieData.length > 0 ? (
              <ResponsiveContainer width="100%" height={200}>
                <PieChart>
                  <Pie
                    data={pieData}
                    dataKey="value"
                    nameKey="name"
                    cx="50%"
                    cy="50%"
                    innerRadius={40}
                    outerRadius={80}
                    paddingAngle={2}
                  >
                    {pieData.map((entry, index) => (
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
          <h3>Top 10 应用</h3>
          {topApps.length > 0 ? (
            <div className="apps-list">
              {topApps.map(([app, duration], index) => (
                <div key={index} className="app-item">
                  <div className="app-rank" style={{ backgroundColor: colors[index % colors.length] }}>
                    {index + 1}
                  </div>
                  <div className="app-info">
                    <span className="app-name">{app}</span>
                    <div className="app-bar-container">
                      <div
                        className="app-bar"
                        style={{
                          width: `${(duration / topApps[0][1]) * 100}%`,
                          backgroundColor: colors[index % colors.length],
                        }}
                      />
                    </div>
                  </div>
                  <div className="app-stats">
                    <span className="app-duration">{formatDuration(duration)}</span>
                    <span className="app-percent">
                      {totalApps > 0 ? ((duration / totalApps) * 100).toFixed(1) : 0}%
                    </span>
                  </div>
                </div>
              ))}
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
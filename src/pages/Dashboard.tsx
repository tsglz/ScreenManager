import { useState, useEffect } from 'react'
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, Cell, PieChart, Pie } from 'recharts'
import Header from '../components/Header'
import { api, UsageRecord, CategoryStats } from '../utils/api'
import { formatDuration, formatDateTime } from '../utils/format'
import './Dashboard.css'

function Dashboard() {
  const [totalDuration, setTotalDuration] = useState(0)
  const [topApps, setTopApps] = useState<[string, number][]>([])
  const [hourlyData, setHourlyData] = useState<[number, number][]>([])
  const [recentRecords, setRecentRecords] = useState<UsageRecord[]>([])
  const [categoryStats, setCategoryStats] = useState<CategoryStats[]>([])
  const [viewMode, setViewMode] = useState<'apps' | 'categories'>('apps')

  const loadData = async () => {
    try {
      const [total, apps, hourly, records, categories] = await Promise.all([
        api.getTodayTotalDuration(),
        api.getTopAppsToday(10),
        api.getHourlyDistributionToday(),
        api.getRecentRecords(20),
        api.getTodayCategoryStats(),
      ])
      setTotalDuration(total)
      setTopApps(apps)
      setHourlyData(hourly)
      setRecentRecords(records)
      setCategoryStats(categories)
    } catch (error) {
      console.error('Failed to load data:', error)
    }
  }

  useEffect(() => {
    loadData()
    const interval = setInterval(loadData, 5000)
    return () => clearInterval(interval)
  }, [])

  const chartData = hourlyData.map(([hour, duration]) => ({
    hour: `${hour}`,
    duration,
  }))

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

  const pieData = viewMode === 'apps'
    ? topApps.map(([name, value], index) => ({
        name,
        value,
        fill: colors[index % colors.length],
      }))
    : categoryStats.map((cat) => ({
        name: cat.category_name,
        value: cat.duration_seconds,
        fill: categoryColors[cat.category_name] || '#b0bec5',
      }))

  const totalValue = viewMode === 'apps'
    ? topApps.reduce((sum, [, duration]) => sum + duration, 0)
    : categoryStats.reduce((sum, cat) => sum + cat.duration_seconds, 0)

  return (
    <div className="dashboard">
      <Header title="今日概况" />

      <div className="dashboard-content">
        <div className="stats-grid">
          <div className="stat-card highlight">
            <div className="stat-value">{formatDuration(totalDuration)}</div>
            <div className="stat-label">今日使用时长</div>
          </div>
          <div className="stat-card">
            <div className="stat-value">{topApps.length}</div>
            <div className="stat-label">活跃应用</div>
          </div>
          <div className="stat-card">
            <div className="stat-value">{recentRecords.length}</div>
            <div className="stat-label">活动记录</div>
          </div>
        </div>

        <div className="charts-row">
          <div className="chart-card">
            <div className="card-header">
              <h3>使用占比</h3>
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
            {pieData.length > 0 ? (
              <div className="donut-container">
                <ResponsiveContainer width="100%" height={220}>
                  <PieChart>
                    <Pie
                      data={pieData}
                      dataKey="value"
                      nameKey="name"
                      cx="50%"
                      cy="50%"
                      innerRadius={60}
                      outerRadius={90}
                      paddingAngle={2}
                    >
                      {pieData.map((entry, index) => (
                        <Cell key={index} fill={entry.fill} />
                      ))}
                    </Pie>
                    <Tooltip formatter={(value) => [formatDuration(value as number), '时长']} />
                  </PieChart>
                </ResponsiveContainer>
                <div className="donut-legend">
                  {pieData.slice(0, 6).map((item, index) => (
                    <div key={index} className="legend-item">
                      <span className="legend-color" style={{ backgroundColor: item.fill }} />
                      <span className="legend-name">{item.name}</span>
                      <span className="legend-percent">
                        {totalValue > 0 ? ((item.value / totalValue) * 100).toFixed(1) : 0}%
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            ) : (
              <div className="empty-state">暂无数据</div>
            )}
          </div>

          <div className="chart-card">
            <h3>小时分布</h3>
            <ResponsiveContainer width="100%" height={220}>
              <BarChart data={chartData}>
                <XAxis dataKey="hour" tick={{ fontSize: 11 }} />
                <YAxis tickFormatter={(v) => formatDuration(v as number)} tick={{ fontSize: 11 }} />
                <Tooltip formatter={(value) => [formatDuration(value as number), '时长']} />
                <Bar dataKey="duration" radius={[4, 4, 0, 0]}>
                  {chartData.map((_, index) => (
                    <Cell key={index} fill={colors[index % colors.length]} />
                  ))}
                </Bar>
              </BarChart>
            </ResponsiveContainer>
          </div>
        </div>

        <div className="recent-section">
          <h3>最近活动</h3>
          {recentRecords.length > 0 ? (
            <table className="records-table">
              <thead>
                <tr>
                  <th>应用</th>
                  <th>窗口标题</th>
                  <th>开始时间</th>
                  <th>时长</th>
                </tr>
              </thead>
              <tbody>
                {recentRecords.map((record) => (
                  <tr key={record.id}>
                    <td>{record.process_name}</td>
                    <td className="title-cell">{record.window_title || '-'}</td>
                    <td>{formatDateTime(record.start_time)}</td>
                    <td>{formatDuration(record.duration_seconds)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : (
            <div className="empty-state">暂无活动记录</div>
          )}
        </div>
      </div>
    </div>
  )
}

export default Dashboard
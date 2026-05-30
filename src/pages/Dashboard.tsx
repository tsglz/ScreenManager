import { useState, useEffect } from 'react'
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, Cell } from 'recharts'
import Header from '../components/Header'
import { api, UsageRecord } from '../utils/api'
import { formatDuration, formatDateTime } from '../utils/format'
import './Dashboard.css'

function Dashboard() {
  const [totalDuration, setTotalDuration] = useState(0)
  const [topApps, setTopApps] = useState<[string, number][]>([])
  const [hourlyData, setHourlyData] = useState<[number, number][]>([])
  const [recentRecords, setRecentRecords] = useState<UsageRecord[]>([])

  const loadData = async () => {
    try {
      const [total, apps, hourly, records] = await Promise.all([
        api.getTodayTotalDuration(),
        api.getTopAppsToday(10),
        api.getHourlyDistributionToday(),
        api.getRecentRecords(20),
      ])
      setTotalDuration(total)
      setTopApps(apps)
      setHourlyData(hourly)
      setRecentRecords(records)
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
              <h3>应用使用排行</h3>
              <button className="refresh-btn" onClick={loadData}>刷新</button>
            </div>
            {topApps.length > 0 ? (
              <div className="apps-list">
                {topApps.map(([app, duration], index) => (
                  <div key={index} className="app-item">
                    <div className="app-rank">{index + 1}</div>
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
                    <span className="app-duration">{formatDuration(duration)}</span>
                  </div>
                ))}
              </div>
            ) : (
              <div className="empty-state">暂无数据</div>
            )}
          </div>

          <div className="chart-card">
            <h3>小时分布</h3>
            <ResponsiveContainer width="100%" height={200}>
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
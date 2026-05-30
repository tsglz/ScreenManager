import { useState, useEffect } from 'react'
import { Tooltip, ResponsiveContainer, PieChart, Pie, Cell } from 'recharts'
import Header from '../components/Header'
import { api } from '../utils/api'
import { formatDuration, getTodayDate, formatFullDate } from '../utils/format'
import './Daily.css'

function Daily() {
  const [selectedDate, setSelectedDate] = useState(getTodayDate())
  const [totalDuration, setTotalDuration] = useState(0)
  const [topApps, setTopApps] = useState<[string, number][]>([])
  const [allApps, setAllApps] = useState<[string, number][]>([])

  useEffect(() => {
    loadData()
  }, [selectedDate])

  const loadData = async () => {
    try {
      const [total, top, all] = await Promise.all([
        api.getDailyTotal(selectedDate),
        api.getDailyTopApps(selectedDate, 10),
        api.getDailyAllApps(selectedDate),
      ])
      setTotalDuration(total)
      setTopApps(top)
      setAllApps(all)
    } catch (error) {
      console.error('Failed to load data:', error)
    }
  }

  const handleDateChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setSelectedDate(e.target.value)
  }

  const colors = ['#4fc3f7', '#81c784', '#fff176', '#ffb74d', '#ce93d8', '#90caf9', '#a5d6a7', '#fff59d', '#ffcc80', '#f48fb1', '#80deea', '#c5e1a5']

  const pieData = allApps.map(([name, value], index) => ({
    name,
    value,
    fill: colors[index % colors.length],
  }))

  const totalApps = allApps.reduce((sum, [, duration]) => sum + duration, 0)

  return (
    <div className="daily-page">
      <Header title="日统计" />

      <div className="daily-content">
        <div className="date-selector">
          <input
            type="date"
            value={selectedDate}
            onChange={handleDateChange}
            max={getTodayDate()}
          />
          <span className="date-label">{formatFullDate(selectedDate)}</span>
        </div>

        <div className="stats-grid">
          <div className="stat-card highlight">
            <div className="stat-value">{formatDuration(totalDuration)}</div>
            <div className="stat-label">当日总时长</div>
          </div>
          <div className="stat-card">
            <div className="stat-value">{allApps.length}</div>
            <div className="stat-label">应用数量</div>
          </div>
        </div>

        <div className="charts-row">
          <div className="chart-card">
            <h3>应用使用排行</h3>
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

          <div className="chart-card">
            <h3>占比分布</h3>
            {pieData.length > 0 ? (
              <>
                <ResponsiveContainer width="100%" height={200}>
                  <PieChart>
                    <Pie
                      data={pieData}
                      dataKey="value"
                      nameKey="name"
                      cx="50%"
                      cy="50%"
                      innerRadius={50}
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
                <div className="pie-legend">
                  {pieData.slice(0, 6).map((item, index) => (
                    <div key={index} className="legend-item">
                      <span className="legend-color" style={{ backgroundColor: item.fill }} />
                      <span className="legend-name">{item.name}</span>
                    </div>
                  ))}
                </div>
              </>
            ) : (
              <div className="empty-state">暂无数据</div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}

export default Daily
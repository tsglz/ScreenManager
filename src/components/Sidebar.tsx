import { NavLink } from 'react-router-dom'
import { useTheme } from '../hooks/useTheme'
import './Sidebar.css'

const navItems = [
  { path: '/', label: '今日工作', icon: '📋' },
  { path: '/generate-report', label: '生成报告', icon: '📝' },
  { path: '/work-timeline', label: '工作时间线', icon: '🕐' },
  { path: '/time-heatmap', label: '时段热力图', icon: '🔥' },
  { path: '/app-records', label: '应用记录', icon: '📱' },
  { path: '/history-reports', label: '历史报告', icon: '📚' },
  { path: '/privacy', label: '隐私保护', icon: '🔒' },
]

function Sidebar() {
  const { theme, toggleTheme } = useTheme()

  return (
    <nav className="sidebar">
      <div className="sidebar-logo">
        <span className="logo-icon">📊</span>
        <span className="logo-text">工作报告</span>
      </div>
      <ul className="nav-list">
        {navItems.map((item) => (
          <li key={item.path}>
            <NavLink
              to={item.path}
              end={item.path === '/'}
              className={({ isActive }) => `nav-item ${isActive ? 'active' : ''}`}
            >
              <span className="nav-icon">{item.icon}</span>
              <span className="nav-label">{item.label}</span>
            </NavLink>
          </li>
        ))}
      </ul>
      <ul className="nav-list bottom">
        <li>
          <button className="nav-item theme-toggle" onClick={toggleTheme}>
            <span className="nav-icon">{theme === 'light' ? '🌙' : '☀️'}</span>
            <span className="nav-label">{theme === 'light' ? '夜间模式' : '日间模式'}</span>
          </button>
        </li>
        <li>
          <NavLink
            to="/settings"
            className={({ isActive }) => `nav-item ${isActive ? 'active' : ''}`}
          >
            <span className="nav-icon">⚙️</span>
            <span className="nav-label">设置</span>
          </NavLink>
        </li>
      </ul>
    </nav>
  )
}

export default Sidebar
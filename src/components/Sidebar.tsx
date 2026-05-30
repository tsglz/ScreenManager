import { NavLink } from 'react-router-dom'
import './Sidebar.css'

const navItems = [
  { path: '/', label: '今日', icon: '📊' },
  { path: '/daily', label: '日', icon: '📅' },
  { path: '/weekly', label: '周', icon: '📆' },
  { path: '/monthly', label: '月', icon: '🗓️' },
]

const bottomNavItems = [
  { path: '/settings', label: '设置', icon: '⚙️' },
]

function Sidebar() {
  return (
    <nav className="sidebar">
      <div className="sidebar-logo">
        <span className="logo-icon">⏱️</span>
        <span className="logo-text">ScreenTime</span>
      </div>
      <ul className="nav-list">
        {navItems.map((item) => (
          <li key={item.path}>
            <NavLink
              to={item.path}
              className={({ isActive }) => `nav-item ${isActive ? 'active' : ''}`}
            >
              <span className="nav-icon">{item.icon}</span>
              <span className="nav-label">{item.label}</span>
            </NavLink>
          </li>
        ))}
      </ul>
      <ul className="nav-list bottom">
        {bottomNavItems.map((item) => (
          <li key={item.path}>
            <NavLink
              to={item.path}
              className={({ isActive }) => `nav-item ${isActive ? 'active' : ''}`}
            >
              <span className="nav-icon">{item.icon}</span>
              <span className="nav-label">{item.label}</span>
            </NavLink>
          </li>
        ))}
      </ul>
    </nav>
  )
}

export default Sidebar

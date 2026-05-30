import { useState, useEffect } from 'react'
import { api } from '../utils/api'
import './Header.css'

interface HeaderProps {
  title: string
}

function Header({ title }: HeaderProps) {
  const [autostartEnabled, setAutostartEnabled] = useState(false)

  useEffect(() => {
    api.isAutostartEnabled().then(setAutostartEnabled).catch(console.error)
  }, [])

  const handleAutostartToggle = async () => {
    try {
      const result = await api.setAutostart(!autostartEnabled)
      if (result) {
        setAutostartEnabled(!autostartEnabled)
      }
    } catch (error) {
      console.error('Failed to toggle autostart:', error)
    }
  }

  return (
    <header className="header">
      <h1 className="header-title">{title}</h1>
      <div className="autostart-toggle">
        <span className="autostart-label">开机自启</span>
        <div
          className={`toggle-switch ${autostartEnabled ? 'active' : ''}`}
          onClick={handleAutostartToggle}
        >
          <div className="toggle-knob" />
        </div>
      </div>
    </header>
  )
}

export default Header
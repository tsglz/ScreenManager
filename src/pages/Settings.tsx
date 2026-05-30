import { useState, useEffect } from 'react'
import { api } from '../utils/api'
import { useUpdater } from '../hooks/useUpdater'
import { UpdateModal } from '../components/UpdateModal'
import './Settings.css'

export default function Settings() {
  const [autostartEnabled, setAutostartEnabled] = useState(false)
  const [appVersion, setAppVersion] = useState('')
  const [dataPath, setDataPath] = useState('')
  const [loading, setLoading] = useState(true)
  const [showUpdateModal, setShowUpdateModal] = useState(false)
  const { status: updateStatus, checkForUpdates } = useUpdater(false)

  useEffect(() => {
    loadSettings()
  }, [])

  const loadSettings = async () => {
    try {
      setLoading(true)
      const [autostart, version, path] = await Promise.all([
        api.isAutostartEnabled(),
        api.getAppVersion(),
        api.getDataPath(),
      ])
      setAutostartEnabled(autostart)
      setAppVersion(version)
      setDataPath(path)
    } catch (error) {
      console.error('Failed to load settings:', error)
    } finally {
      setLoading(false)
    }
  }

  const handleAutostartToggle = async () => {
    try {
      const newValue = !autostartEnabled
      const success = await api.setAutostart(newValue)
      if (success) {
        setAutostartEnabled(newValue)
      }
    } catch (error) {
      console.error('Failed to toggle autostart:', error)
    }
  }

  const handleCheckUpdate = async () => {
    setShowUpdateModal(true)
  }

  if (loading) {
    return <div className="settings-page loading">加载中...</div>
  }

  return (
    <div className="settings-page">
      <h1>设置</h1>

      <section className="settings-section">
        <h2>常规</h2>

        <div className="setting-item">
          <div className="setting-info">
            <h3>开机自启</h3>
            <p>系统启动时自动运行 Screen Manager</p>
          </div>
          <label className="toggle-switch">
            <input
              type="checkbox"
              checked={autostartEnabled}
              onChange={handleAutostartToggle}
            />
            <span className="toggle-slider"></span>
          </label>
        </div>
      </section>

      <section className="settings-section">
        <h2>关于</h2>

        <div className="setting-item">
          <div className="setting-info">
            <h3>当前版本</h3>
            <p className="version-number">{appVersion || '未知'}</p>
          </div>
          <button className="btn-check-update" onClick={handleCheckUpdate}>
            检查更新
          </button>
        </div>

        <div className="setting-item">
          <div className="setting-info">
            <h3>数据存储位置</h3>
            <p className="data-path">{dataPath || '未知'}</p>
          </div>
        </div>
      </section>

      <section className="settings-section">
        <h2>更新状态</h2>
        <div className="update-status">
          {updateStatus === 'idle' && <p>点击"检查更新"按钮查看是否有新版本</p>}
          {updateStatus === 'checking' && <p>正在检查更新...</p>}
          {updateStatus === 'available' && <p className="update-available">发现新版本可用！</p>}
          {updateStatus === 'error' && <p className="update-error">检查更新失败</p>}
        </div>
      </section>

      <section className="settings-section">
        <h2>配置说明</h2>
        <div className="config-info">
          <p>配置文件位于数据存储目录中：</p>
          <ul>
            <li><code>screen_time.db</code> - 使用记录数据库</li>
            <li><code>config.json</code> - 应用配置</li>
            <li><code>categories.json</code> - 应用分类配置</li>
          </ul>
          <p className="note">更新程序时会自动保留以上所有数据文件</p>
        </div>
      </section>

      {showUpdateModal && <UpdateModal onClose={() => setShowUpdateModal(false)} />}
    </div>
  )
}

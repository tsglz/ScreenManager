import { useState, useEffect } from 'react'
import { api, type AppConfig } from '../utils/api'
import { UpdateModal } from '../components/UpdateModal'
import { useTheme } from '../hooks/useTheme'
import './Settings.css'

export default function Settings() {
  const [autostartEnabled, setAutostartEnabled] = useState(false)
  const [appVersion, setAppVersion] = useState('')
  const [dataPath, setDataPath] = useState('')
  const [proxyValue, setProxyValue] = useState('')
  const [proxySaving, setProxySaving] = useState(false)
  const [proxyMsg, setProxyMsg] = useState<{ type: 'ok' | 'err'; text: string } | null>(null)
  const [loading, setLoading] = useState(true)
  const [showUpdateModal, setShowUpdateModal] = useState(false)
  const { theme, toggleTheme } = useTheme()

  useEffect(() => {
    loadSettings()
  }, [])

  const loadSettings = async () => {
    try {
      setLoading(true)
      const [autostart, version, path, cfg] = await Promise.all([
        api.isAutostartEnabled(),
        api.getAppVersion(),
        api.getDataPath(),
        api.getAppConfig(),
      ])
      setAutostartEnabled(autostart)
      setAppVersion(version)
      setDataPath(path)
      setProxyValue(cfg?.http_proxy ?? '')
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

  const handleSaveProxy = async () => {
    try {
      setProxySaving(true)
      setProxyMsg(null)
      const trimmed = proxyValue.trim()
      const cfg: AppConfig = {
        http_proxy: trimmed.length === 0 ? null : trimmed,
      }
      await api.saveAppConfig(cfg)
      setProxyValue(cfg.http_proxy ?? '')
      setProxyMsg({
        type: 'ok',
        text: trimmed
          ? '代理已保存并立即生效，建议重新点击「检查更新」验证'
          : '已清除代理设置，立即生效',
      })
    } catch (error) {
      console.error('Failed to save proxy:', error)
      setProxyMsg({
        type: 'err',
        text: error instanceof Error ? error.message : '保存失败，请重试',
      })
    } finally {
      setProxySaving(false)
    }
  }

  const handleCheckUpdate = () => {
    setProxyMsg(null)
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

        <div className="setting-item">
          <div className="setting-info">
            <h3>显示模式</h3>
            <p>{theme === 'light' ? '当前为日间模式' : '当前为夜间模式'}</p>
          </div>
          <label className="toggle-switch">
            <input
              type="checkbox"
              checked={theme === 'dark'}
              onChange={toggleTheme}
            />
            <span className="toggle-slider"></span>
          </label>
        </div>
      </section>

      <section className="settings-section">
        <h2>网络与代理</h2>

        <div className="setting-item column">
          <div className="setting-info full">
            <h3>HTTP / HTTPS 代理</h3>
            <p>
              设置后立即生效（应用启动时也会自动加载）。留空则不使用自定义代理，
              会读取系统环境变量 <code>HTTPS_PROXY</code> 或直连网络。
            </p>
            <p className="setting-hint">
              例：<code>http://127.0.0.1:7890</code>（Clash / v2rayN 默认端口通常为 7890 / 10809）
            </p>
          </div>
          <div className="proxy-row">
            <input
              type="text"
              className="proxy-input"
              placeholder="http://127.0.0.1:7890"
              value={proxyValue}
              onChange={(e) => setProxyValue(e.target.value)}
              spellCheck={false}
            />
            <button
              className="btn-check-update"
              onClick={handleSaveProxy}
              disabled={proxySaving}
            >
              {proxySaving ? '保存中...' : '保存'}
            </button>
          </div>
          {proxyMsg && (
            <p className={`proxy-msg ${proxyMsg.type}`}>{proxyMsg.text}</p>
          )}
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

      {showUpdateModal && (
        <UpdateModal 
          onClose={() => setShowUpdateModal(false)} 
          currentVersion={appVersion}
        />
      )}
    </div>
  )
}

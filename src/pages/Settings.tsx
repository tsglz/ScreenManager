import { useState, useEffect } from 'react'
import { api, type AppConfig, type CleanupResult, type StorageStats } from '../utils/api'
import { formatBytes } from '../utils/format'
import { UpdateModal } from '../components/UpdateModal'
import { useTheme } from '../hooks/useTheme'
import './Settings.css'

type TableStatRow = [string, number]

const TABLE_NAME_CHINESE: Record<string, string> = {
  usage_records: '使用明细记录',
  daily_summary: '按日汇总记录',
  reports: 'AI 报告',
  app_categories: '应用分类映射',
  categories: '分类定义',
  record_project_overrides: '手动项目归属',
  config: '配置项',
}

function formatTableCount(_tableName: string, cnt: number): string {
  if (cnt < 10_000) return cnt.toLocaleString('zh-CN')
  return `${(cnt / 10000).toFixed(2)} 万`
}

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

  // ===== 存储占用 & 清理 =====
  const [storageStats, setStorageStats] = useState<StorageStats | null>(null)
  const [storageLoading, setStorageLoading] = useState(false)
  const [cleaning, setCleaning] = useState(false)
  const [confirmStage, setConfirmStage] = useState<0 | 1>(0)
  const [cleanupMsg, setCleanupMsg] = useState<{ type: 'ok' | 'err'; text: string } | null>(null)
  const [lastResult, setLastResult] = useState<CleanupResult | null>(null)

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
      // 设置加载完成后再拉一次存储统计，与前面无关
      loadStorageStats()
    }
  }

  const loadStorageStats = async () => {
    try {
      setStorageLoading(true)
      const s = await api.getStorageStats()
      setStorageStats(s)
    } catch (e) {
      console.error('Failed to load storage stats:', e)
    } finally {
      setStorageLoading(false)
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
      // 先读已有配置，避免覆盖其他配置项（如默认 Ollama 模型）
      const existing = await api.getAppConfig().catch(() => null)
      const cfg: AppConfig = {
        ...(existing || {}),
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

  const handleCleanupClick = () => {
    setCleanupMsg(null)
    if (confirmStage === 0) {
      setConfirmStage(1)
      return
    }
    // 二次确认后真正执行
    void (async () => {
      try {
        setCleaning(true)
        const res = await api.cleanupExpiredRecords()
        setLastResult(res)
        setConfirmStage(0)
        setCleanupMsg({
          type: 'ok',
          text: `清理完成：共删除 ${(res.deleted_usage_rows + res.deleted_daily_rows).toLocaleString('zh-CN')} 条记录，释放 ${formatBytes(res.saved_bytes)} 磁盘空间。`,
        })
        await loadStorageStats()
      } catch (e) {
        console.error(e)
        setConfirmStage(0)
        setCleanupMsg({
          type: 'err',
          text: e instanceof Error ? `清理失败：${e.message}` : '清理失败，请稍后重试',
        })
      } finally {
        setCleaning(false)
      }
    })()
  }

  // 5 秒后自动把「OK 提示」从界面上拿掉，避免堆积
  useEffect(() => {
    if (!cleanupMsg || cleanupMsg.type !== 'ok') return
    const t = setTimeout(() => setCleanupMsg(null), 8000)
    return () => clearTimeout(t)
  }, [cleanupMsg])

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

      {/* ====== 存储占用 & 清理 ====== */}
      <section className="settings-section">
        <div className="storage-head">
          <h2>数据存储与清理</h2>
          <button
            className="btn-refresh"
            onClick={loadStorageStats}
            disabled={storageLoading || cleaning}
          >
            {storageLoading ? '刷新中...' : '刷新统计'}
          </button>
        </div>

        {storageLoading && <div className="storage-hint">正在读取存储信息...</div>}

        {!storageLoading && !storageStats && (
          <div className="storage-hint">暂时无法读取存储统计，请点击「刷新统计」重试。</div>
        )}

        {!storageLoading && storageStats && (
          <>
            <div className="storage-card">
              <div className="storage-overview">
                <div className="storage-metric">
                  <div className="storage-metric-label">数据库文件</div>
                  <div className="storage-metric-value-big">
                    {formatBytes(storageStats.db_file_bytes)}
                  </div>
                  <div className="storage-metric-sub" title={storageStats.db_file_path}>
                    {storageStats.db_file_path || '—'}
                  </div>
                </div>
                <div className="storage-metric">
                  <div className="storage-metric-label">记录时间范围</div>
                  <div className="storage-metric-value">
                    {storageStats.earliest_record_date && storageStats.latest_record_date ? (
                      <>
                        {storageStats.earliest_record_date} → {storageStats.latest_record_date}
                      </>
                    ) : (
                      '暂无记录'
                    )}
                  </div>
                  <div className="storage-metric-sub">最早 ~ 最近的使用明细日期</div>
                </div>
                <div className="storage-metric warn">
                  <div className="storage-metric-label">
                    可清理 {storageStats.cleanup_cutoff_days} 天前数据（{storageStats.cleanup_cutoff_date} 之前）
                  </div>
                  <div className="storage-metric-value">
                    使用明细 {storageStats.cleanup_usage_rows.toLocaleString('zh-CN')} 条
                    <span className="sep">·</span>
                    日汇总 {storageStats.cleanup_daily_rows.toLocaleString('zh-CN')} 条
                  </div>
                  <div className="storage-metric-sub">AI 报告、应用分类配置、手动项目归属不会被清理</div>
                </div>
              </div>

              <div className="storage-table-wrap">
                <div className="storage-table-title">各表记录数</div>
                <div className="storage-table">
                  {(storageStats.table_rows as TableStatRow[]).map(([table, cnt]) => (
                    <div key={table} className="storage-table-row">
                      <div className="storage-table-cell left">
                        <span className="storage-table-cell-name">
                          {TABLE_NAME_CHINESE[table] || table}
                        </span>
                        <span className="storage-table-cell-sys">{table}</span>
                      </div>
                      <div className="storage-table-cell right">
                        {formatTableCount(table, Number(cnt) || 0)}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>

            <div className="storage-actions">
              <div className="storage-actions-left">
                {cleanupMsg && (
                  <div className={`cleanup-msg ${cleanupMsg.type}`}>{cleanupMsg.text}</div>
                )}
                {!cleanupMsg && lastResult && lastResult.saved_bytes > 0 && (
                  <div className="cleanup-msg ok muted">
                    上次清理：{lastResult.cutoff_date} 之前，释放 {formatBytes(lastResult.saved_bytes)}
                  </div>
                )}
                {confirmStage === 1 && (
                  <div className="cleanup-confirm">
                    ⚠️ 你即将永久删除 <strong>{storageStats.cleanup_cutoff_date}</strong> 之前的
                    <strong>{storageStats.cleanup_usage_rows + storageStats.cleanup_daily_rows}</strong>
                    条旧记录（AI 报告与配置会保留）。该操作不可撤销。
                  </div>
                )}
              </div>
              <button
                className={confirmStage === 1 ? 'btn-danger' : 'btn-check-update'}
                onClick={handleCleanupClick}
                disabled={
                  cleaning ||
                  storageLoading ||
                  (storageStats.cleanup_usage_rows + storageStats.cleanup_daily_rows <= 0 && confirmStage === 0)
                }
                title={
                  storageStats.cleanup_usage_rows + storageStats.cleanup_daily_rows <= 0
                    ? '当前没有超过 10 天的旧记录可以清理'
                    : undefined
                }
              >
                {cleaning
                  ? '清理中，请稍候...'
                  : confirmStage === 1
                  ? `确认删除（${storageStats.cleanup_cutoff_date} 之前）`
                  : `一键清理 ${storageStats.cleanup_cutoff_days} 天前的过期数据`}
              </button>
            </div>
          </>
        )}
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

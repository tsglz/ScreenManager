import { useEffect } from 'react'
import { useUpdater } from '../hooks/useUpdater'
import './UpdateModal.css'

interface UpdateModalProps {
  onClose: () => void
  currentVersion: string
}

export function UpdateModal({ onClose, currentVersion }: UpdateModalProps) {
  const { status, updateInfo, error, progress, checkForUpdates, installUpdate, dismiss } = useUpdater(false)

  // 弹窗打开后立即触发检查
  useEffect(() => {
    checkForUpdates()
  }, [checkForUpdates])

  const handleUpdate = async () => {
    await installUpdate()
  }

  const handleLater = () => {
    dismiss()
    onClose()
  }

  const handleRetry = () => {
    checkForUpdates()
  }

  const handleClose = () => {
    dismiss()
    onClose()
  }

  const renderContent = () => {
    switch (status) {
      case 'idle':
      case 'checking':
        return (
          <div className="update-modal-content checking">
            <div className="spinner"></div>
            <p>正在检查更新...</p>
          </div>
        )

      case 'uptodate':
        return (
          <div className="update-modal-content uptodate">
            <div className="success-icon">✨</div>
            <h3>已是最新版本</h3>
            <p>当前版本 <strong>{currentVersion}</strong> 为最新版本</p>
            <div className="button-group">
              <button className="btn-primary" onClick={handleClose}>确定</button>
              <button className="btn-secondary" onClick={handleRetry}>重新检查</button>
            </div>
          </div>
        )

      case 'available':
        return (
          <div className="update-modal-content available">
            <div className="update-icon">🎉</div>
            <h3>发现新版本</h3>
            <div className="version-info">
              <p><strong>当前版本：</strong>{currentVersion}</p>
              <p><strong>最新版本：</strong>{updateInfo?.version}</p>
              {updateInfo?.date && (
                <p><strong>发布日期：</strong>{updateInfo.date}</p>
              )}
            </div>
            {updateInfo?.body && (
              <div className="release-notes">
                <h4>更新说明：</h4>
                <p>{updateInfo.body}</p>
              </div>
            )}
            <div className="button-group">
              <button className="btn-primary" onClick={handleUpdate}>立即更新</button>
              <button className="btn-secondary" onClick={handleLater}>稍后提醒</button>
            </div>
          </div>
        )

      case 'downloading':
        return (
          <div className="update-modal-content downloading">
            <div className="progress-container">
              <div className="progress-bar">
                <div className="progress-fill" style={{ width: `${progress}%` }}></div>
              </div>
              <p className="progress-text">正在下载更新... {progress}%</p>
            </div>
          </div>
        )

      case 'installing':
        return (
          <div className="update-modal-content installing">
            <div className="spinner"></div>
            <p>正在安装更新...</p>
          </div>
        )

      case 'ready':
        return (
          <div className="update-modal-content ready">
            <div className="success-icon">✅</div>
            <h3>更新完成</h3>
            <p>更新已安装完成，请重启应用</p>
            <div className="button-group">
              <button className="btn-primary" onClick={handleClose}>确定</button>
            </div>
          </div>
        )

      case 'error':
      default:
        return (
          <div className="update-modal-content error">
            <div className="error-icon">❌</div>
            <h3>检查失败</h3>
            <p>{error || '更新过程中出现错误'}</p>
            <div className="button-group">
              <button className="btn-primary" onClick={handleRetry}>重试</button>
              <button className="btn-secondary" onClick={handleClose}>关闭</button>
            </div>
          </div>
        )
    }
  }

  return (
    <div className="update-modal-overlay">
      <div className="update-modal">
        <button className="close-button" onClick={handleClose}>×</button>
        {renderContent()}
      </div>
    </div>
  )
}

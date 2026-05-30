import { useUpdater, UpdateStatus } from '../hooks/useUpdater'
import './UpdateModal.css'

interface UpdateModalProps {
  onClose: () => void
}

export function UpdateModal({ onClose }: UpdateModalProps) {
  const { status, updateInfo, error, progress, checkForUpdates, downloadAndInstall, dismiss } = useUpdater(false)

  const handleUpdate = async () => {
    await downloadAndInstall()
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
      case 'checking':
        return (
          <div className="update-modal-content checking">
            <div className="spinner"></div>
            <p>正在检查更新...</p>
          </div>
        )

      case 'available':
        return (
          <div className="update-modal-content available">
            <div className="update-icon">🎉</div>
            <h3>发现新版本</h3>
            <div className="version-info">
              <p><strong>当前版本：</strong>{updateInfo?.current_version}</p>
              <p><strong>最新版本：</strong>{updateInfo?.latest_version}</p>
            </div>
            {updateInfo?.release_notes && (
              <div className="release-notes">
                <h4>更新说明：</h4>
                <p>{updateInfo.release_notes}</p>
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

      case 'ready':
        return (
          <div className="update-modal-content ready">
            <div className="success-icon">✅</div>
            <h3>更新已下载</h3>
            <p>更新已下载完成，应用即将重启...</p>
          </div>
        )

      case 'error':
        return (
          <div className="update-modal-content error">
            <div className="error-icon">❌</div>
            <h3>更新失败</h3>
            <p>{error || '更新过程中出现错误'}</p>
            <div className="button-group">
              <button className="btn-primary" onClick={handleRetry}>重试</button>
              <button className="btn-secondary" onClick={handleClose}>关闭</button>
            </div>
          </div>
        )

      case 'idle':
      default:
        return (
          <div className="update-modal-content idle">
            <p>检查更新时发生错误</p>
            <button className="btn-primary" onClick={handleRetry}>重试</button>
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

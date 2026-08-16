import './Placeholder.css'

function Privacy() {
  return (
    <div className="placeholder-page">
      <div className="ph-header">
        <h1>隐私保护</h1>
      </div>
      <div className="ph-content">
        <div className="ph-card">
          <div className="ph-icon">🔒</div>
          <h2>隐私与数据保护</h2>
          <p>所有数据均存储在本地，不会上传到任何服务器</p>
          <div className="ph-features">
            <div className="ph-feature-item">
              <span className="ph-check">✓</span> 数据仅存储于本地
            </div>
            <div className="ph-feature-item">
              <span className="ph-check">✓</span> 无网络传输
            </div>
            <div className="ph-feature-item">
              <span className="ph-check">✓</span> 可随时清除数据
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

export default Privacy
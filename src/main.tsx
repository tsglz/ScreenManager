import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './index.css'

import { check } from '@tauri-apps/plugin-updater'
import { api } from './utils/api'

// 异步检查更新
async function autoUpdate() {
  try {
    const cfg = await api.getAppConfig().catch(() => null)
    const proxy = cfg?.http_proxy && cfg.http_proxy.trim() !== '' ? cfg.http_proxy.trim() : undefined
    if (proxy) {
      console.log('[autoUpdate] 使用配置的代理地址:', proxy)
    }
    const update = await check({ proxy })
    if (update) {
      await update.downloadAndInstall()
    }
  } catch (e) {
    console.error('update failed:', e)
  }
}

// 启动延迟执行
setTimeout(autoUpdate, 2000)

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
)
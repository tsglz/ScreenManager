import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './index.css'

import { check } from '@tauri-apps/plugin-updater'

// 异步检查更新
async function autoUpdate() {
  try {
    // appUpdater 内部会自动调用 check 和 install
    const update = await check()
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
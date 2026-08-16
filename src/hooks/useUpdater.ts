import { useState, useCallback, useEffect } from 'react'
import { check } from '@tauri-apps/plugin-updater'
import { api } from '../utils/api'

export type UpdateStatus = 'idle' | 'checking' | 'available' | 'downloading' | 'installing' | 'ready' | 'error' | 'uptodate'

export interface UpdateInfo {
  version: string
  body: string
  date: string
}

interface UseUpdaterReturn {
  status: UpdateStatus
  updateInfo: UpdateInfo | null
  error: string | null
  progress: number
  checkForUpdates: () => Promise<void>
  installUpdate: () => Promise<void>
  dismiss: () => void
}

export function useUpdater(autoCheck = true): UseUpdaterReturn {
  const [status, setStatus] = useState<UpdateStatus>('idle')
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [progress, setProgress] = useState(0)
  const [currentUpdate, setCurrentUpdate] = useState<Awaited<ReturnType<typeof check>> | null>(null)

  const checkForUpdates = useCallback(async () => {
    setStatus('checking')
    setError(null)

    try {
      const cfg = await api.getAppConfig().catch(() => null)
      const proxy = cfg?.http_proxy && cfg.http_proxy.trim() !== '' ? cfg.http_proxy.trim() : undefined
      if (proxy) {
        // eslint-disable-next-line no-console
        console.log('[Updater] 使用配置的代理地址进行更新检查:', proxy)
      }

      const update = await check({ proxy })

      if (update) {
        setUpdateInfo({
          version: update.version,
          body: update.body || '',
          date: update.date || '',
        })
        setCurrentUpdate(update)
        setStatus('available')
      } else {
        setStatus('uptodate')
      }
    } catch (err) {
      const raw = err instanceof Error ? err.message : String(err ?? '未知错误')
      const hint =
        '（国内用户提示：请先在「设置 → 网络与代理」中填入代理地址；应用内置 ghproxy 镜像作为候选节点，' +
        '若仍失败请手动访问 GitHub Releases 下载。）'
      setError(`${raw}${hint}`)
      setStatus('error')
    }
  }, [])

  const installUpdate = useCallback(async () => {
    if (!currentUpdate) {
      setError('没有可用的更新')
      setStatus('error')
      return
    }

    setStatus('downloading')
    setProgress(0)
    setError(null)

    try {
      await currentUpdate.downloadAndInstall((event: unknown) => {
        const e = event as { event?: string; progress?: { current: number; total: number } }
        if (e.progress && e.progress.total !== undefined) {
          const { current, total } = e.progress
          if (total > 0 && current !== undefined) {
            const newProgress = Math.round((current / total) * 100)
            setProgress(newProgress)
          }
        }
      })

      setStatus('installing')
      setProgress(100)
      
      setTimeout(() => {
        setStatus('ready')
      }, 1000)
    } catch (err) {
      setError(err instanceof Error ? err.message : '下载或安装更新失败')
      setStatus('error')
    }
  }, [currentUpdate])

  const dismiss = useCallback(() => {
    setStatus('idle')
    setUpdateInfo(null)
    setError(null)
    setProgress(0)
    setCurrentUpdate(null)
  }, [])

  useEffect(() => {
    if (autoCheck) {
      const timer = setTimeout(() => {
        checkForUpdates()
      }, 3000)
      return () => clearTimeout(timer)
    }
  }, [autoCheck, checkForUpdates])

  return {
    status,
    updateInfo,
    error,
    progress,
    checkForUpdates,
    installUpdate,
    dismiss,
  }
}

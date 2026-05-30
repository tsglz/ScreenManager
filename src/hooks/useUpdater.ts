import { useState, useCallback, useEffect } from 'react'
import { check } from '@tauri-apps/plugin-updater'

export type UpdateStatus = 'idle' | 'checking' | 'available' | 'downloading' | 'installing' | 'ready' | 'error'

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
      const update = await check()

      if (update) {
        setUpdateInfo({
          version: update.version,
          body: update.body || '',
          date: update.date || '',
        })
        setCurrentUpdate(update)
        setStatus('available')
      } else {
        setStatus('idle')
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : '检查更新失败，请检查网络连接')
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

import { useState, useCallback, useEffect } from 'react'
import { api, UpdateInfo } from '../utils/api'

export type UpdateStatus = 'idle' | 'checking' | 'available' | 'downloading' | 'ready' | 'error'

interface UseUpdaterReturn {
  status: UpdateStatus
  updateInfo: UpdateInfo | null
  error: string | null
  progress: number
  checkForUpdates: () => Promise<void>
  downloadAndInstall: () => Promise<void>
  dismiss: () => void
}

export function useUpdater(autoCheck = true): UseUpdaterReturn {
  const [status, setStatus] = useState<UpdateStatus>('idle')
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [progress, setProgress] = useState(0)

  const checkForUpdates = useCallback(async () => {
    setStatus('checking')
    setError(null)

    try {
      const info = await api.checkForUpdate()

      if (info.has_update) {
        setUpdateInfo(info)
        setStatus('available')
      } else {
        setStatus('idle')
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : '检查更新失败，请检查网络连接')
      setStatus('error')
    }
  }, [])

  const downloadAndInstall = useCallback(async () => {
    setStatus('downloading')
    setProgress(0)

    try {
      setProgress(50)
      await api.performUpdate()
      setProgress(100)
      setStatus('ready')
    } catch (err) {
      setError(err instanceof Error ? err.message : '下载或安装更新失败')
      setStatus('error')
    }
  }, [])

  const dismiss = useCallback(() => {
    setStatus('idle')
    setUpdateInfo(null)
    setError(null)
    setProgress(0)
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
    downloadAndInstall,
    dismiss,
  }
}

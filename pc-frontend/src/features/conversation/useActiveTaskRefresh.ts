import { useEffect } from 'react'

const ACTIVE_TASK_REFRESH_INTERVAL_MS = 1800

interface UseActiveTaskRefreshOptions {
  activeProjectId?: string | null
  hasRunningTask: boolean
  refreshTaskSurface: () => Promise<void>
}

export function useActiveTaskRefresh({
  activeProjectId,
  hasRunningTask,
  refreshTaskSurface,
}: UseActiveTaskRefreshOptions): void {
  useEffect(() => {
    if (!hasRunningTask || !activeProjectId) return
    let canceled = false
    let inFlight = false

    async function refreshVisibleTask() {
      if (canceled || inFlight) return
      inFlight = true
      try {
        await refreshTaskSurface()
      } catch (err) {
        console.warn('[TaskSurface] active task refresh failed:', err)
      } finally {
        inFlight = false
      }
    }

    refreshVisibleTask()
    const timer = window.setInterval(refreshVisibleTask, ACTIVE_TASK_REFRESH_INTERVAL_MS)
    return () => {
      canceled = true
      window.clearInterval(timer)
    }
  }, [activeProjectId, hasRunningTask, refreshTaskSurface])
}

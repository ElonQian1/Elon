import { useEffect, useRef, useState } from 'react'
import { subscribeRealtimeEvents } from './realtimeBus'
import type { RealtimeResourceKey } from './resourceKeys'
import type { RealtimeEvent } from './realtimeEvents'

const DEFAULT_DEBOUNCE_MS = 160
const DEFAULT_RUNNING_POLL_MS = 1800
const DEFAULT_SETTLE_POLL_MS = 2600
const DEFAULT_SETTLE_WINDOW_MS = 12_000

interface Options {
  enabled?: boolean
  running?: boolean
  debounceMs?: number
  runningPollMs?: number
  settlePollMs?: number
  settleWindowMs?: number
  refreshOnFocus?: boolean
  resourceKeys?: RealtimeResourceKey[]
  refresh: () => void | Promise<void>
  shouldRefreshEvent?: (event: RealtimeEvent) => boolean
}

export function useRealtimeResourceRefresh({
  enabled = true,
  running = false,
  debounceMs = DEFAULT_DEBOUNCE_MS,
  runningPollMs = DEFAULT_RUNNING_POLL_MS,
  settlePollMs = DEFAULT_SETTLE_POLL_MS,
  settleWindowMs = DEFAULT_SETTLE_WINDOW_MS,
  refreshOnFocus = true,
  resourceKeys,
  refresh,
  shouldRefreshEvent,
}: Options) {
  const refreshRef = useRef(refresh)
  const shouldRefreshEventRef = useRef(shouldRefreshEvent)
  const resourceKeysRef = useRef(resourceKeys)
  const refreshTimerRef = useRef<number | undefined>()
  const [settleUntil, setSettleUntil] = useState(0)

  useEffect(() => {
    refreshRef.current = refresh
  }, [refresh])

  useEffect(() => {
    shouldRefreshEventRef.current = shouldRefreshEvent
  }, [shouldRefreshEvent])

  useEffect(() => {
    resourceKeysRef.current = resourceKeys
  }, [resourceKeys])

  useEffect(() => {
    if (!enabled) return

    function clearRefreshTimer() {
      if (refreshTimerRef.current) {
        window.clearTimeout(refreshTimerRef.current)
        refreshTimerRef.current = undefined
      }
    }

    function scheduleRefresh(delay = debounceMs) {
      clearRefreshTimer()
      setSettleUntil(Date.now() + settleWindowMs)
      refreshTimerRef.current = window.setTimeout(() => {
        refreshTimerRef.current = undefined
        refreshRef.current()
      }, delay)
    }

    const unsubscribe = subscribeRealtimeEvents((event) => {
      const watched = resourceKeysRef.current ?? []
      const resourceMatches = watched.length === 0 || event.resources.some((key) => watched.includes(key))
      const shouldRefresh = shouldRefreshEventRef.current?.(event) ?? resourceMatches
      if (shouldRefresh) scheduleRefresh(event.type === 'project_task_done' ? 0 : debounceMs)
    })

    return () => {
      clearRefreshTimer()
      unsubscribe()
    }
  }, [debounceMs, enabled, settleWindowMs])

  useEffect(() => {
    if (!enabled) return
    if (!running && Date.now() >= settleUntil) return
    const intervalMs = running ? runningPollMs : settlePollMs
    const timer = window.setInterval(() => {
      if (!running && Date.now() >= settleUntil) {
        setSettleUntil(0)
        return
      }
      refreshRef.current()
    }, intervalMs)
    return () => window.clearInterval(timer)
  }, [enabled, running, runningPollMs, settlePollMs, settleUntil])

  useEffect(() => {
    if (!enabled || !refreshOnFocus) return
    function refreshVisiblePage() {
      if (document.visibilityState === 'hidden') return
      setSettleUntil(Date.now() + settleWindowMs)
      refreshRef.current()
    }
    window.addEventListener('focus', refreshVisiblePage)
    document.addEventListener('visibilitychange', refreshVisiblePage)
    return () => {
      window.removeEventListener('focus', refreshVisiblePage)
      document.removeEventListener('visibilitychange', refreshVisiblePage)
    }
  }, [enabled, refreshOnFocus, settleWindowMs])
}

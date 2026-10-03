import { useEffect } from 'react'
import { create } from 'zustand'
import { cloudConnectionProbeUrls } from '../../api/runtime'
import {
  CLOUD_PROBE_INTERVAL_MS, CLOUD_PROBE_TIMEOUT_MS, probeWorkbenchCloud, settleCloudProbe,
  type CloudConnectionSnapshot,
} from './workbenchConnectionModel'

const RETRY_EVENT = 'elon:retry-workbench-connection'

interface WorkbenchConnection extends CloudConnectionSnapshot {
  checking: boolean
  recoveryEpoch: number
  retry: () => void
}

export const useWorkbenchConnection = create<WorkbenchConnection>(() => ({
  cloudState: 'checking', failedProbes: 0, checking: true, recoveryEpoch: 0,
  retry: () => window.dispatchEvent(new Event(RETRY_EVENT)),
}))

/** Mounted once by Shell; navigation and notices share this bounded, read-only probe. */
export function useWorkbenchConnectionMonitor(enabled = true): void {
  useEffect(() => {
    if (!enabled) return
    let disposed = false
    let inFlight: AbortController | null = null
    let timer: number | undefined
    let epoch = 0
    let pendingRetry = false

    async function probe() {
      if (disposed) return
      if (inFlight) { pendingRetry = true; return }
      pendingRetry = false
      window.clearTimeout(timer)
      const controller = new AbortController()
      inFlight = controller
      const probeEpoch = epoch
      useWorkbenchConnection.setState({ checking: true })
      const timeout = window.setTimeout(() => controller.abort(), CLOUD_PROBE_TIMEOUT_MS)
      try {
        const readinessUrl = cloudConnectionProbeUrls()[0]
        const available = await probeWorkbenchCloud(readinessUrl, controller.signal)
        if (disposed || probeEpoch !== epoch) return
        useWorkbenchConnection.setState((state) => ({
          ...settleCloudProbe(state, available), checking: false,
          recoveryEpoch: state.recoveryEpoch + (available && state.cloudState === 'offline' ? 1 : 0),
        }))
      } finally {
        window.clearTimeout(timeout)
        inFlight = null
        if (!disposed) {
          if (pendingRetry) void probe()
          else timer = window.setTimeout(() => void probe(), CLOUD_PROBE_INTERVAL_MS)
        }
      }
    }

    function retry() { void probe() }
    function offline() {
      epoch += 1
      inFlight?.abort()
      useWorkbenchConnection.setState({ cloudState: 'offline', checking: false })
    }
    window.addEventListener('online', retry)
    window.addEventListener('offline', offline)
    window.addEventListener(RETRY_EVENT, retry)
    void probe()
    return () => {
      disposed = true
      inFlight?.abort()
      window.clearTimeout(timer)
      window.removeEventListener('online', retry)
      window.removeEventListener('offline', offline)
      window.removeEventListener(RETRY_EVENT, retry)
    }
  }, [enabled])
}

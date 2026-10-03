export type CloudState = 'checking' | 'online' | 'offline'
export interface CloudConnectionSnapshot {
  cloudState: CloudState
  failedProbes: number
}

export const CLOUD_FAILURE_THRESHOLD = 4
export const CLOUD_PROBE_INTERVAL_MS = 15_000
export const CLOUD_PROBE_TIMEOUT_MS = 8_000

export function settleCloudProbe(previous: CloudConnectionSnapshot, available: boolean): CloudConnectionSnapshot {
  if (available) return { cloudState: 'online', failedProbes: 0 }
  const failedProbes = previous.failedProbes + 1
  return {
    cloudState: previous.cloudState === 'checking' || failedProbes >= CLOUD_FAILURE_THRESHOLD
      ? 'offline' : previous.cloudState,
    failedProbes,
  }
}

/** Readiness, rather than process liveness, decides whether cloud features can recover. */
export async function probeWorkbenchCloud(url: string, signal: AbortSignal): Promise<boolean> {
  try {
    const response = await fetch(url, {
      cache: 'no-store', credentials: 'omit', signal,
      headers: { Accept: 'application/json' },
    })
    if (!response.ok || !response.headers.get('content-type')?.includes('application/json')) return false
    const readiness = await response.json() as { service?: string; status?: string }
    return readiness.service === 'elon-server' && readiness.status === 'ok'
  } catch {
    return false
  }
}

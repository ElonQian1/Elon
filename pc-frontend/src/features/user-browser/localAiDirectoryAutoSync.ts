export function localAiDirectoryAutoSyncKey({
  sessionIdentity,
  windowLabel,
  sessionOpen,
  navigationUpdatedAtMs,
  directoryComplete,
}: {
  sessionIdentity: string
  windowLabel?: string
  sessionOpen: boolean
  navigationUpdatedAtMs?: number
  directoryComplete?: boolean
}) {
  const identity = sessionIdentity.trim()
  if (!sessionOpen || !identity) return ''
  const freshness = Math.max(0, Number(navigationUpdatedAtMs) || 0)
  return `${identity}:${windowLabel?.trim() || 'session'}:${directoryComplete === true ? 'complete' : 'stale'}:${freshness}`
}

export const LOCAL_AI_DIRECTORY_FRESHNESS_MS = 2 * 60_000

export function localAiDirectoryNeedsAutoSync({
  navigationEvent,
  navigationUpdatedAtMs,
  nowMs = Date.now(),
}: {
  navigationEvent?: { collection?: { complete?: boolean } } | null
  navigationUpdatedAtMs?: number
  nowMs?: number
}) {
  if (navigationEvent?.collection?.complete !== true) return true
  const updatedAt = Number(navigationUpdatedAtMs) || 0
  if (updatedAt <= 0) return true
  return Math.max(0, nowMs - updatedAt) >= LOCAL_AI_DIRECTORY_FRESHNESS_MS
}

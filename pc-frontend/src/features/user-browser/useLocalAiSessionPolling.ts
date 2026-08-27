import { useCallback, useEffect, useRef } from 'react'
import {
  getLocalAiWebSessionState,
  isLocalAiConversationSnapshot,
  isLocalAiMessageSnapshot,
  type LocalAiWebSessionState,
} from './localAiBrowserApi'
import { localAiSnapshotIsStreaming } from './localAiPrivateStreamSignal'
import {
  LOCAL_AI_NATIVE_UPDATE_COALESCE_MS,
  listenLocalAiNativeSessionUpdates,
  localAiNativeSessionUpdateMatches,
} from './localAiNativeSessionUpdates'

const ACTIVE_POLL_DELAY_MS = 1_500
const PRIVATE_STREAM_WATCHDOG_POLL_DELAY_MS = 4_000
const OFFICIAL_WINDOW_POLL_DELAY_MS = 3_000
const IDLE_POLL_DELAY_MS = 6_000
const DEGRADED_POLL_DELAY_MS = 8_000
const BACKGROUND_POLL_DELAY_MS = 15_000

interface LocalAiSessionPollingOptions {
  enabled: boolean
  providerId?: string
  ownerKey?: string
  state: LocalAiWebSessionState | null
  onState: (state: LocalAiWebSessionState) => void
}

export function localAiSessionPollDelay(
  state: LocalAiWebSessionState | null,
  visibilityState: DocumentVisibilityState,
): number {
  if (visibilityState === 'hidden') return BACKGROUND_POLL_DELAY_MS
  if (!state
    || state.loading
    || state.windowStatus === 'opening'
    || state.windowStatus === 'loading'
    || state.rendererStatus === 'connecting'
    || state.contextReady === false) return ACTIVE_POLL_DELAY_MS

  const snapshot = isLocalAiMessageSnapshot(state.semanticEvent) ? state.semanticEvent : null
  if (localAiSnapshotIsStreaming(snapshot)) {
    return snapshot?.privateStreamObserved === true
      ? PRIVATE_STREAM_WATCHDOG_POLL_DELAY_MS
      : ACTIVE_POLL_DELAY_MS
  }

  const directory = isLocalAiConversationSnapshot(state.navigationEvent) ? state.navigationEvent : null
  if (directory?.collection && !directory.collection.complete) return ACTIVE_POLL_DELAY_MS

  if (state.windowStatus === 'error' || state.windowStatus === 'blocked') return DEGRADED_POLL_DELAY_MS
  if (state.windowStatus === 'closed') return BACKGROUND_POLL_DELAY_MS
  if (state.windowVisible) return OFFICIAL_WINDOW_POLL_DELAY_MS
  return IDLE_POLL_DELAY_MS
}

export default function useLocalAiSessionPolling({
  enabled,
  providerId,
  ownerKey,
  state,
  onState,
}: LocalAiSessionPollingOptions): () => void {
  const stateRef = useRef(state)
  const onStateRef = useRef(onState)
  const refreshRef = useRef<() => void>(() => {})
  stateRef.current = state
  onStateRef.current = onState

  useEffect(() => {
    if (!enabled || !providerId || !ownerKey) return
    let active = true
    let polling = false
    let refreshQueued = false
    let timer = 0
    let nativeUpdateTimer = 0
    let unlistenNativeUpdate = () => {}

    const schedule = (delay: number) => {
      window.clearTimeout(timer)
      timer = window.setTimeout(() => void poll(), delay)
    }
    const poll = async () => {
      if (!active) return
      if (polling) {
        refreshQueued = true
        return
      }
      polling = true
      window.clearTimeout(timer)
      try {
        const next = await getLocalAiWebSessionState(providerId, ownerKey)
        if (active) {
          stateRef.current = next
          onStateRef.current(next)
        }
      } catch {
        // Background state reads are best effort. User actions surface their own actionable errors.
      } finally {
        polling = false
        if (!active) return
        if (refreshQueued) {
          refreshQueued = false
          schedule(0)
        } else {
          schedule(localAiSessionPollDelay(stateRef.current, document.visibilityState))
        }
      }
    }
    const refreshNow = () => {
      window.clearTimeout(timer)
      if (polling) refreshQueued = true
      else void poll()
    }
    const refreshWhenVisible = () => {
      if (document.visibilityState === 'visible') refreshNow()
      else if (!polling) schedule(BACKGROUND_POLL_DELAY_MS)
    }

    refreshRef.current = refreshNow
    void listenLocalAiNativeSessionUpdates((update) => {
      if (!active || !localAiNativeSessionUpdateMatches(
        update,
        providerId,
        stateRef.current?.windowLabel,
      )) return
      if (nativeUpdateTimer) return
      nativeUpdateTimer = window.setTimeout(() => {
        nativeUpdateTimer = 0
        refreshNow()
      }, LOCAL_AI_NATIVE_UPDATE_COALESCE_MS)
    }).then((unlisten) => {
      if (active) unlistenNativeUpdate = unlisten
      else unlisten()
    }).catch(() => {
      // Native events are an optimization. Fixed polling remains the fail-open path.
    })
    window.addEventListener('focus', refreshWhenVisible)
    document.addEventListener('visibilitychange', refreshWhenVisible)
    void poll()
    return () => {
      active = false
      window.clearTimeout(timer)
      window.clearTimeout(nativeUpdateTimer)
      unlistenNativeUpdate()
      window.removeEventListener('focus', refreshWhenVisible)
      document.removeEventListener('visibilitychange', refreshWhenVisible)
      refreshRef.current = () => {}
    }
  }, [enabled, ownerKey, providerId])

  return useCallback(() => refreshRef.current(), [])
}

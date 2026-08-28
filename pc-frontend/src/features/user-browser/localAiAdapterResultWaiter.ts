import { findMatchingLocalAiCommandReceipt } from './localAiCommandReceipt'
import {
  LOCAL_AI_RESULT_POLL_INTERVAL_MS,
  localAiAdapterResultTimeoutMs,
} from './localAiAdapterTiming'
import {
  listenLocalAiNativeSessionUpdates,
  localAiNativeSessionUpdateMatches,
  type LocalAiNativeSessionUpdate,
} from './localAiNativeSessionUpdates'
import type { LocalAiWebSessionState } from './localAiBrowserApi'

export interface LocalAiAdapterResultRequest {
  action: string
  requestId: string
}

interface LocalAiAdapterResultWaiterOptions {
  providerId: string
  requests: ReadonlyArray<LocalAiAdapterResultRequest>
  readState: () => Promise<LocalAiWebSessionState>
  listen?: typeof listenLocalAiNativeSessionUpdates
  pollIntervalMs?: number
  timeoutMs?: number
}

function stateWithMatchingReceipt(
  state: LocalAiWebSessionState,
  requests: ReadonlyArray<LocalAiAdapterResultRequest>,
): LocalAiWebSessionState | null {
  for (const { action, requestId } of requests) {
    const receipt = findMatchingLocalAiCommandReceipt(
      state.commandResult,
      state.commandResults,
      action,
      requestId,
    )
    if (receipt) return state.commandResult === receipt ? state : { ...state, commandResult: receipt }
  }
  return null
}

/**
 * Resolve adapter receipts from the native update emitted by the WebView host.
 * A short fixed poll remains as a fail-open watchdog for event registration or
 * delivery failures, but the normal send path no longer waits for its next tick.
 */
export async function waitForLocalAiAdapterReceipts({
  providerId,
  requests,
  readState,
  listen = listenLocalAiNativeSessionUpdates,
  pollIntervalMs = LOCAL_AI_RESULT_POLL_INTERVAL_MS,
  timeoutMs = Math.max(...requests.map(({ action }) => localAiAdapterResultTimeoutMs(action))),
}: LocalAiAdapterResultWaiterOptions): Promise<LocalAiWebSessionState | null> {
  let active = true
  let targetWindowLabel: string | undefined
  let requestRead = () => {}
  const unlistenPromise = Promise.resolve()
    .then(() => listen((update: LocalAiNativeSessionUpdate) => {
      if (!active || update.kind !== 'command_result') return
      if (targetWindowLabel) {
        if (!localAiNativeSessionUpdateMatches(update, providerId, targetWindowLabel)) return
      } else if (update.providerId !== providerId) {
        return
      }
      requestRead()
    }))
    .catch(() => () => {})

  try {
    return await new Promise((resolve) => {
      let settled = false
      let reading = false
      let readQueued = false
      let pollTimer: ReturnType<typeof setTimeout> | undefined
      const deadlineTimer = setTimeout(() => finish(null), timeoutMs)

      function finish(state: LocalAiWebSessionState | null) {
        if (settled) return
        settled = true
        if (pollTimer !== undefined) clearTimeout(pollTimer)
        clearTimeout(deadlineTimer)
        resolve(state)
      }

      function scheduleWatchdog() {
        if (settled) return
        if (pollTimer !== undefined) clearTimeout(pollTimer)
        pollTimer = setTimeout(() => {
          pollTimer = undefined
          requestRead()
        }, pollIntervalMs)
      }

      async function readNow() {
        if (settled) return
        if (reading) {
          readQueued = true
          return
        }
        reading = true
        if (pollTimer !== undefined) {
          clearTimeout(pollTimer)
          pollTimer = undefined
        }
        try {
          const state = await readState()
          targetWindowLabel = state.windowLabel || targetWindowLabel
          const matched = stateWithMatchingReceipt(state, requests)
          if (matched) finish(matched)
        } catch {
          // State reads remain best effort until the action-specific deadline.
        } finally {
          reading = false
          if (settled) return
          if (readQueued) {
            readQueued = false
            void readNow()
          } else {
            scheduleWatchdog()
          }
        }
      }

      requestRead = () => void readNow()
      requestRead()
    })
  } finally {
    active = false
    void unlistenPromise.then((unlisten) => unlisten())
  }
}

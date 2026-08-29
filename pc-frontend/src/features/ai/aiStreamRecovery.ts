type StreamRecoveryCallbacks = {
  consumeStream: () => Promise<void>
  resetAssistantForReplay: () => void
  onBackgroundStarted: () => void
  onBackgroundFinished: () => void
  onBackgroundExpired: () => void | Promise<void>
}

// Keep a short network blip invisible, then let the background recovery own
// delivery so the composer is never held hostage by an HTTP connection.
const STREAM_RETRY_DELAYS_MS = [1000, 2000, 4000, 8000]
const BACKGROUND_STREAM_RETRY_DELAYS_MS = [
  10000, 15000, 30000, 45000, 60000, 60000, 60000, 60000, 60000, 60000,
]

export function isRecoverableStreamError(error: unknown) {
  const candidate = error as { status?: number; message?: string }
  const message = candidate?.message ?? ''
  const status = candidate?.status
  if (status === 0) return true
  if (status !== undefined && ![502, 503, 504].includes(status)) return false
  return /(连接|网络|中断|回答完成前|fetch|load failed|timeout|超时)/i.test(message)
}

function waitForStreamRetry(delayMs: number) {
  return new Promise<void>((resolve) => window.setTimeout(resolve, delayMs))
}

async function resumeInBackground(callbacks: StreamRecoveryCallbacks) {
  for (const delayMs of BACKGROUND_STREAM_RETRY_DELAYS_MS) {
    await waitForStreamRetry(delayMs)
    callbacks.resetAssistantForReplay()
    try {
      await callbacks.consumeStream()
      callbacks.onBackgroundFinished()
      return
    } catch {
      // A transport failure is not a user-facing chat error. Keep trying while
      // the server-side task is retained and let the next connection replay it.
    }
  }
  callbacks.onBackgroundExpired()
}

export async function consumeStreamWithRecovery(callbacks: StreamRecoveryCallbacks) {
  let retryAttempt = 0
  while (true) {
    try {
      await callbacks.consumeStream()
      return
    } catch (streamError) {
      if (!isRecoverableStreamError(streamError)) throw streamError
      if (retryAttempt >= STREAM_RETRY_DELAYS_MS.length) {
        callbacks.onBackgroundStarted()
        void resumeInBackground(callbacks)
        return
      }
      const delayMs = STREAM_RETRY_DELAYS_MS[retryAttempt]
      retryAttempt += 1
      await waitForStreamRetry(delayMs)
      callbacks.resetAssistantForReplay()
    }
  }
}

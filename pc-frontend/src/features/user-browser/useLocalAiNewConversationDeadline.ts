import { useEffect, useState } from 'react'
import { NEW_CONVERSATION_RECOVERY_TIMEOUT_MS } from './localAiWebChatControllerConfig'

export function localAiNewConversationDeadlineDelay(
  startedAtMs: number,
  observedAtMs = Date.now(),
): number | null {
  if (!startedAtMs) return null
  return Math.max(0, NEW_CONVERSATION_RECOVERY_TIMEOUT_MS - (observedAtMs - startedAtMs))
}

/** Forces settlement even when a stalled official WebView emits no more state changes. */
export default function useLocalAiNewConversationDeadline(startedAtMs: number): boolean {
  const [expiredGeneration, setExpiredGeneration] = useState(0)

  useEffect(() => {
    setExpiredGeneration(0)
    const delay = localAiNewConversationDeadlineDelay(startedAtMs)
    if (delay === null) return
    const timer = window.setTimeout(() => setExpiredGeneration(startedAtMs), delay)
    return () => window.clearTimeout(timer)
  }, [startedAtMs])

  return Boolean(startedAtMs && expiredGeneration === startedAtMs)
}

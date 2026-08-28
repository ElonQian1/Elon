import { useEffect, useState } from 'react'
import type { LocalAiMessageSnapshot, LocalAiWebSessionState } from './localAiBrowserApi'
import { localAiNewConversationSettleDelayMs } from './localAiNewConversation'

interface LocalAiNewConversationSettleOptions {
  session: LocalAiWebSessionState | null
  snapshot: LocalAiMessageSnapshot | null
  startedAtMs: number
  baselineConversationId: string
}

/** Re-evaluates an otherwise idle empty new-chat surface after its stability window. */
export default function useLocalAiNewConversationSettle({
  session,
  snapshot,
  startedAtMs,
  baselineConversationId,
}: LocalAiNewConversationSettleOptions): number {
  const [settledGeneration, setSettledGeneration] = useState(0)

  useEffect(() => {
    const delay = localAiNewConversationSettleDelayMs(
      session,
      snapshot,
      startedAtMs,
      baselineConversationId,
    )
    if (delay === null) return
    const timer = window.setTimeout(() => {
      setSettledGeneration((generation) => generation + 1)
    }, delay)
    return () => window.clearTimeout(timer)
  }, [baselineConversationId, session, snapshot, startedAtMs])

  return settledGeneration
}

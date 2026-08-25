import { useEffect, useRef, useState } from 'react'
import {
  PENDING_LOCAL_AI_RESPONSE_SLOW_MS,
  pendingLocalAiResponseSyncSlow,
  type PendingLocalAiResponse,
} from './localAiOptimisticSend'

export default function useLocalAiPendingResponseWatchdog(
  identity: string,
  pendingResponses: PendingLocalAiResponse[],
) {
  const generation = useRef(0)
  const [slowResponseId, setSlowResponseId] = useState('')
  const latest = pendingResponses[pendingResponses.length - 1]

  useEffect(() => {
    generation.current += 1
    const activeGeneration = generation.current
    setSlowResponseId('')
    if (!identity || !latest) return
    if (pendingLocalAiResponseSyncSlow(latest)) {
      setSlowResponseId(latest.id)
      return
    }
    const elapsedMs = Date.now() - latest.startedAtMs
    const timer = window.setTimeout(() => {
      if (activeGeneration !== generation.current) return
      setSlowResponseId(latest.id)
    }, Math.max(0, PENDING_LOCAL_AI_RESPONSE_SLOW_MS - elapsedMs))
    return () => window.clearTimeout(timer)
  }, [identity, latest])

  return Boolean(latest && slowResponseId === latest.id)
}

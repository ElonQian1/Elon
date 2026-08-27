import { useCallback, useRef, useState } from 'react'

export default function useLocalAiNewConversationLifecycle<T>() {
  const [queuedSend, setQueuedSendValue] = useState<T | null>(null)
  const [recoveryStartedAtMs, setRecoveryStartedAtMs] = useState(0)
  const baselineId = useRef('')
  const queuedSendRef = useRef<T | null>(null)
  const queuedSendDispatching = useRef(false)

  const setQueuedSend = useCallback((value: T | null) => {
    queuedSendRef.current = value
    setQueuedSendValue(value)
  }, [])
  const finish = useCallback(() => {
    baselineId.current = ''
    setRecoveryStartedAtMs(0)
  }, [])
  const clearQueuedSend = useCallback(() => {
    setQueuedSend(null)
    queuedSendDispatching.current = false
  }, [setQueuedSend])
  const reset = useCallback(() => {
    finish()
    clearQueuedSend()
  }, [clearQueuedSend, finish])
  const begin = useCallback((baselineConversationId: string) => {
    baselineId.current = baselineConversationId
    setRecoveryStartedAtMs(Date.now())
  }, [])
  const cancel = useCallback((restore?: (queued: T) => void) => {
    const queued = queuedSendRef.current
    reset()
    if (queued && restore) restore(queued)
  }, [reset])

  return {
    baselineId,
    begin,
    cancel,
    clearQueuedSend,
    finish,
    queuedSend,
    queuedSendDispatching,
    queuedSendRef,
    recoveryStartedAtMs,
    reset,
    setQueuedSend,
  }
}

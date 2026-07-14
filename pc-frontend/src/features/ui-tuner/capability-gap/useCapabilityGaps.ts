import { useCallback, useEffect, useRef, useState } from 'react'
import { listCapabilityGaps } from './capabilityGapApi'
import type { CapabilityGapDocument } from './types'

const ACTIVE_STATUSES = new Set(['APPROVED', 'UPGRADING', 'PUBLISHED'])

export function useCapabilityGaps(sessionId?: string) {
  const [gaps, setGaps] = useState<CapabilityGapDocument[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const generationRef = useRef(0)

  const refresh = useCallback(async () => {
    if (!sessionId) return
    const generation = generationRef.current
    setLoading(true)
    try {
      const next = await listCapabilityGaps(sessionId)
      if (generation !== generationRef.current) return
      setGaps(next)
      setError('')
    } catch (refreshError) {
      if (generation !== generationRef.current) return
      setError(refreshError instanceof Error ? refreshError.message : '无法读取平台能力状态')
    } finally {
      if (generation === generationRef.current) setLoading(false)
    }
  }, [sessionId])

  useEffect(() => {
    generationRef.current += 1
    setGaps([])
    setError('')
    if (!sessionId) return
    void refresh()
  }, [refresh, sessionId])

  const hasActiveGap = gaps.some((gap) => ACTIVE_STATUSES.has(gap.status))
  useEffect(() => {
    if (!sessionId || !hasActiveGap) return
    const timer = window.setInterval(() => { void refresh() }, 2_000)
    return () => window.clearInterval(timer)
  }, [hasActiveGap, refresh, sessionId])

  return { gaps, loading, error, refresh }
}

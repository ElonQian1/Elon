import { useCallback, useEffect, useRef, useState } from 'react'
import { getCapabilityReadiness, listCapabilityGaps } from './capabilityGapApi'
import type { CapabilityGapDocument, CapabilityReadiness } from './types'

const ACTIVE_STATUSES = new Set(['APPROVED', 'UPGRADING', 'PUBLISHED'])

export function useCapabilityGaps(sessionId?: string) {
  const [gaps, setGaps] = useState<CapabilityGapDocument[]>([])
  const [readiness, setReadiness] = useState<CapabilityReadiness | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const generationRef = useRef(0)

  const refresh = useCallback(async () => {
    if (!sessionId) return
    const generation = generationRef.current
    setLoading(true)
    try {
      const [next, nextReadiness] = await Promise.all([
        listCapabilityGaps(sessionId),
        getCapabilityReadiness(sessionId),
      ])
      if (generation !== generationRef.current) return
      setGaps(next)
      setReadiness(nextReadiness)
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
    setReadiness(null)
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

  return { gaps, readiness, loading, error, refresh }
}

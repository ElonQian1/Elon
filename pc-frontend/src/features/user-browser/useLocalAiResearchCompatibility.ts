import { useEffect, useState } from 'react'
import {
  getLocalAiWebResearchCaptureStatus,
  type LocalAiResearchCaptureStatus,
} from './localAiBrowserApi'

const RESEARCH_STATUS_SETTLEMENT_DELAYS_MS = [0, 1_400, 3_600] as const

interface LocalAiResearchCompatibilityOptions {
  enabled: boolean
  providerId?: string
  ownerKey: string
  semanticUpdatedAtMs: number
  streaming: boolean
}

/**
 * Raw-response analysis settles separately from the visible DOM snapshot.
 * Recheck around each completed semantic revision so upstream structure drift
 * is visible in native chat instead of remaining hidden in diagnostics.
 */
export default function useLocalAiResearchCompatibility({
  enabled,
  providerId,
  ownerKey,
  semanticUpdatedAtMs,
  streaming,
}: LocalAiResearchCompatibilityOptions) {
  const [status, setStatus] = useState<LocalAiResearchCaptureStatus>()

  useEffect(() => {
    setStatus(undefined)
  }, [ownerKey, providerId])

  useEffect(() => {
    if (!enabled || !providerId || !ownerKey || streaming) return
    let active = true
    const timers = RESEARCH_STATUS_SETTLEMENT_DELAYS_MS.map((delay) => window.setTimeout(() => {
      void getLocalAiWebResearchCaptureStatus(providerId, ownerKey)
        .then((next) => { if (active) setStatus(next) })
        .catch(() => {})
    }, delay))
    return () => {
      active = false
      timers.forEach((timer) => window.clearTimeout(timer))
    }
  }, [enabled, ownerKey, providerId, semanticUpdatedAtMs, streaming])

  return status
}

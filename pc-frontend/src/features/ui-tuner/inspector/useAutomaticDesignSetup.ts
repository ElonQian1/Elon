import { useEffect, useRef } from 'react'

interface AutomaticDesignSetupOptions {
  enabled: boolean
  setupKey: string
  runtimeReady: boolean
  runtimeBusy: boolean
  runtimeError: string
  onPrepareRuntime: () => void
  onUseDraft: () => void
}

const DRAFT_FALLBACK_DELAY_MS = 1800

/**
 * Pick the best editable path without asking a designer to understand the
 * Runtime, source binding, or renderer implementation.
 */
export function useAutomaticDesignSetup({
  enabled,
  setupKey,
  runtimeReady,
  runtimeBusy,
  runtimeError,
  onPrepareRuntime,
  onUseDraft,
}: AutomaticDesignSetupOptions) {
  const runtimeAttemptRef = useRef('')
  const draftFallbackRef = useRef('')

  useEffect(() => {
    if (!enabled || !setupKey || runtimeBusy) return undefined

    if (runtimeError) {
      if (draftFallbackRef.current !== setupKey) {
        draftFallbackRef.current = setupKey
        onUseDraft()
      }
      return undefined
    }

    if (runtimeReady) {
      if (runtimeAttemptRef.current !== setupKey) {
        runtimeAttemptRef.current = setupKey
        onPrepareRuntime()
      }
      return undefined
    }

    const fallbackTimer = window.setTimeout(() => {
      if (draftFallbackRef.current === setupKey) return
      draftFallbackRef.current = setupKey
      onUseDraft()
    }, DRAFT_FALLBACK_DELAY_MS)

    return () => window.clearTimeout(fallbackTimer)
  }, [
    enabled,
    onPrepareRuntime,
    onUseDraft,
    runtimeBusy,
    runtimeError,
    runtimeReady,
    setupKey,
  ])
}

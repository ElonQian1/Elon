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

export const DRAFT_FALLBACK_DELAY_MS = 1200
export const RUNTIME_BACKGROUND_FALLBACK_MS = 2200

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
  const prepareRuntimeRef = useRef(onPrepareRuntime)
  const useDraftRef = useRef(onUseDraft)
  prepareRuntimeRef.current = onPrepareRuntime
  useDraftRef.current = onUseDraft

  useEffect(() => {
    if (!enabled || !setupKey) return undefined

    const fallbackToDraft = () => {
      if (draftFallbackRef.current === setupKey) return
      draftFallbackRef.current = setupKey
      useDraftRef.current()
    }

    if (runtimeError) {
      fallbackToDraft()
      return undefined
    }

    if (runtimeAttemptRef.current !== setupKey) {
      runtimeAttemptRef.current = setupKey
      prepareRuntimeRef.current()
    }

    const waitingForRuntime = runtimeReady || runtimeBusy || runtimeAttemptRef.current === setupKey
    const fallbackTimer = window.setTimeout(() => {
      fallbackToDraft()
    }, waitingForRuntime ? RUNTIME_BACKGROUND_FALLBACK_MS : DRAFT_FALLBACK_DELAY_MS)

    return () => window.clearTimeout(fallbackTimer)
  }, [
    enabled,
    runtimeBusy,
    runtimeError,
    runtimeReady,
    setupKey,
  ])
}

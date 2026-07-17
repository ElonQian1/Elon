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
  const prepareRuntimeRef = useRef(onPrepareRuntime)
  const useDraftRef = useRef(onUseDraft)
  prepareRuntimeRef.current = onPrepareRuntime
  useDraftRef.current = onUseDraft

  useEffect(() => {
    if (!enabled || !setupKey || runtimeBusy) return undefined

    if (runtimeError) {
      if (draftFallbackRef.current !== setupKey) {
        draftFallbackRef.current = setupKey
        useDraftRef.current()
      }
      return undefined
    }

    if (runtimeReady) {
      if (runtimeAttemptRef.current !== setupKey) {
        runtimeAttemptRef.current = setupKey
        prepareRuntimeRef.current()
      }
      return undefined
    }

    const fallbackTimer = window.setTimeout(() => {
      if (draftFallbackRef.current === setupKey) return
      draftFallbackRef.current = setupKey
      useDraftRef.current()
    }, DRAFT_FALLBACK_DELAY_MS)

    return () => window.clearTimeout(fallbackTimer)
  }, [
    enabled,
    runtimeBusy,
    runtimeError,
    runtimeReady,
    setupKey,
  ])
}

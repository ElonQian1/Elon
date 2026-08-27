import { useEffect, useRef } from 'react'
import {
  controlLocalAiWebSession,
  getLocalAiWebSessionState,
  type LocalAiWebSessionState,
} from './localAiBrowserApi'

export const LOCAL_AI_BACKGROUND_STALL_TIMEOUT_MS = 12_000

export function localAiBackgroundNavigationStalled(
  state: Pick<
    LocalAiWebSessionState,
    'loading' | 'rendererStatus' | 'windowStatus' | 'windowVisible'
  > | null,
  blocked: boolean,
): boolean {
  return Boolean(
    !blocked
    && state?.loading
    && state.rendererStatus === 'connecting'
    && ['opening', 'loading', 'ready', 'minimized'].includes(state.windowStatus)
    && !state.windowVisible,
  )
}

interface BackgroundNavigationRecoveryOptions {
  providerId: string
  ownerKey: string
  state: LocalAiWebSessionState | null
  blocked: boolean
  onState: (state: LocalAiWebSessionState) => void
  onMessage: (message: string) => void
}

/** Preserves healthy in-flight navigation and reloads one stalled background generation once. */
export default function useLocalAiBackgroundNavigationRecovery({
  providerId,
  ownerKey,
  state,
  blocked,
  onState,
  onMessage,
}: BackgroundNavigationRecoveryOptions) {
  const previousLoading = useRef(false)
  const previousUrl = useRef('')
  const generation = useRef(0)
  const attemptedGeneration = useRef('')
  const callbacks = useRef({ onState, onMessage })
  callbacks.current = { onState, onMessage }
  const loading = localAiBackgroundNavigationStalled(state, blocked)
  const currentUrl = state?.currentUrl ?? ''
  if (loading && (!previousLoading.current || previousUrl.current !== currentUrl)) {
    generation.current += 1
  }
  previousLoading.current = loading
  previousUrl.current = currentUrl
  const generationKey = loading
    ? `${providerId}:${ownerKey}:${generation.current}:${currentUrl}`
    : ''

  useEffect(() => {
    if (!generationKey || attemptedGeneration.current === generationKey) return
    let active = true
    const timer = window.setTimeout(() => {
      void getLocalAiWebSessionState(providerId, ownerKey)
        .then(async (current) => {
          if (!active
            || current.currentUrl !== currentUrl
            || !localAiBackgroundNavigationStalled(current, false)) return
          attemptedGeneration.current = generationKey
          const next = await controlLocalAiWebSession(providerId, ownerKey, 'reload')
          if (!active) return
          callbacks.current.onState(next)
          callbacks.current.onMessage('官网后台加载长时间没有进展，已自动重载一次；缓存内容和草稿仍保留。')
        })
        .catch(() => {
          // The normal session poll and user-visible official-page fallback remain active.
        })
    }, LOCAL_AI_BACKGROUND_STALL_TIMEOUT_MS)
    return () => {
      active = false
      window.clearTimeout(timer)
    }
  }, [currentUrl, generationKey, ownerKey, providerId])
}

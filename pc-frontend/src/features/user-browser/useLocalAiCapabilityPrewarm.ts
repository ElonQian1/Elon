import { useEffect, useRef } from 'react'
import type {
  LocalAiMessageSnapshot,
  LocalAiWebProvider,
  LocalAiWebSessionState,
} from './localAiBrowserApi'
import {
  LOCAL_AI_CAPABILITY_PREWARM_DELAY_MS,
  localAiCapabilityPrewarmCooldown,
  localAiCapabilityPrewarmEligible,
} from './localAiCapabilityPrewarmPolicy'
import { syncLocalAiDeferredMenu } from './localAiDeferredMenuSync'

interface LocalAiCapabilityPrewarmOptions {
  provider: LocalAiWebProvider | undefined
  ownerKey: string
  sessionIdentity: string
  sessionState: LocalAiWebSessionState | null
  snapshot: LocalAiMessageSnapshot | null
  foregroundBlocked: boolean
  onState: (state: LocalAiWebSessionState) => void
}

export default function useLocalAiCapabilityPrewarm({
  provider,
  ownerKey,
  sessionIdentity,
  sessionState,
  snapshot,
  foregroundBlocked,
  onState,
}: LocalAiCapabilityPrewarmOptions) {
  const onStateRef = useRef(onState)
  onStateRef.current = onState
  const eligible = Boolean(provider && localAiCapabilityPrewarmEligible({
    providerId: provider.id,
    adapterActions: provider.adapterActions,
    sessionState,
    snapshot,
    foregroundBlocked,
  }))

  useEffect(() => {
    if (!eligible || !ownerKey || !sessionIdentity) return

    let active = true
    const timer = window.setTimeout(() => {
      if (!active || !localAiCapabilityPrewarmCooldown.claim(sessionIdentity)) return
      void syncLocalAiDeferredMenu({
        providerId: provider?.id ?? '',
        ownerKey,
        sessionIdentity,
        listAction: 'list_model_options',
        collectAction: 'collect_model_options',
      })
        .then((next) => { if (active && next) onStateRef.current(next) })
        .catch(() => {
          // Prewarming is opportunistic. The foreground menu keeps its normal retry and error UI.
        })
    }, LOCAL_AI_CAPABILITY_PREWARM_DELAY_MS)
    return () => {
      active = false
      window.clearTimeout(timer)
    }
  }, [eligible, ownerKey, provider?.id, sessionIdentity])
}

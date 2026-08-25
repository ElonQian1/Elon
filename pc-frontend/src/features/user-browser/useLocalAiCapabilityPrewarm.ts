import { useEffect, useRef } from 'react'
import type {
  LocalAiMessageSnapshot,
  LocalAiWebProvider,
  LocalAiWebSessionState,
} from './localAiBrowserApi'
import {
  runLocalAiWebAdapterCommand,
  waitForLocalAiAdapterResult,
} from './localAiBrowserApi'
import {
  LOCAL_AI_CAPABILITY_PREWARM_DELAY_MS,
  localAiCapabilityPrewarmCooldown,
  localAiCapabilityPrewarmEligible,
  localAiCapabilityPrewarmSupportsModel,
} from './localAiCapabilityPrewarmPolicy'
import { syncLocalAiDeferredMenu } from './localAiDeferredMenuSync'
import { isLocalAiUiManifestSnapshot } from './localAiBrowserProtocol'

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
  const sessionStateRef = useRef(sessionState)
  onStateRef.current = onState
  sessionStateRef.current = sessionState
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
      if (!active || !provider) return
      void prewarmCapabilities({
        provider,
        ownerKey,
        sessionIdentity,
        sessionState: sessionStateRef.current,
        onState: (next) => { if (active) onStateRef.current(next) },
        active: () => active,
      })
        .catch(() => {
          // Prewarming is opportunistic. Foreground actions retain their normal error UI.
        })
    }, LOCAL_AI_CAPABILITY_PREWARM_DELAY_MS)
    return () => {
      active = false
      window.clearTimeout(timer)
    }
  }, [eligible, ownerKey, provider?.id, sessionIdentity])
}

async function prewarmCapabilities({
  provider,
  ownerKey,
  sessionIdentity,
  sessionState,
  onState,
  active,
}: {
  provider: LocalAiWebProvider
  ownerKey: string
  sessionIdentity: string
  sessionState: LocalAiWebSessionState | null
  onState: (state: LocalAiWebSessionState) => void
  active: () => boolean
}) {
  const manifest = isLocalAiUiManifestSnapshot(sessionState?.uiManifestEvent)
    ? sessionState.uiManifestEvent
    : null
  const manifestCurrent = manifest?.compatibility === 'healthy' && !manifest.controlsTruncated
  if (
    !manifestCurrent
      && provider.adapterActions.includes('snapshot_ui_manifest')
      && localAiCapabilityPrewarmCooldown.claim(`${sessionIdentity}:ui_manifest`)
  ) {
    try {
      const requestId = await runLocalAiWebAdapterCommand(
        provider.id,
        ownerKey,
        'snapshot_ui_manifest',
      )
      const next = await waitForLocalAiAdapterResult(
        provider.id,
        ownerKey,
        'snapshot_ui_manifest',
        requestId,
      )
      if (active() && next) onState(next)
    } catch {
      // Manifest and model prewarming are independent; retain the model warm-up on transient drift.
    }
  }
  if (
    active()
      && localAiCapabilityPrewarmSupportsModel(provider.adapterActions)
      && localAiCapabilityPrewarmCooldown.claim(`${sessionIdentity}:model`)
  ) {
    const next = await syncLocalAiDeferredMenu({
      providerId: provider.id,
      ownerKey,
      sessionIdentity,
      listAction: 'list_model_options',
      collectAction: 'collect_model_options',
    })
    if (active() && next) onState(next)
  }
}

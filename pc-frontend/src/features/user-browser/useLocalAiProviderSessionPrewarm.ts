import { useEffect } from 'react'
import {
  getCachedLocalAiWebSessionState,
  getLocalAiWebSessionState,
  openLocalAiWebSession,
  type LocalAiWebProvider,
  type LocalAiWebSessionState,
} from './localAiBrowserApi'
import { localAiWarmSessionReusable } from './localAiWarmSessionPolicy'
import {
  LOCAL_AI_PROVIDER_SESSION_PREWARM_DELAY_MS,
  localAiProviderSessionPrewarmEligible,
  localAiProviderSessionPrewarmGate,
} from './localAiProviderSessionPrewarmPolicy'

interface LocalAiProviderSessionPrewarmOptions {
  enabled: boolean
  providers: LocalAiWebProvider[]
  selectedProviderId?: string
  ownerKey: string
  selectedState: LocalAiWebSessionState | null
}

/**
 * APK keeps both provider WebViews alive and resumes the selected surface. On
 * Win, create the inactive provider only after the active one is stable, then
 * retain its isolated WebView2 profile in the hidden native host. This removes
 * cold provider switches without competing with the first visible chat load.
 */
export default function useLocalAiProviderSessionPrewarm({
  enabled,
  providers,
  selectedProviderId,
  ownerKey,
  selectedState,
}: LocalAiProviderSessionPrewarmOptions) {
  const providerKey = providers.map((provider) => provider.id).join('|')

  useEffect(() => {
    if (!enabled || !ownerKey || !selectedProviderId || providers.length < 2) return
    let active = true
    let timer = 0
    const schedule = () => {
      window.clearTimeout(timer)
      if (!active || document.visibilityState !== 'visible') return
      timer = window.setTimeout(() => {
        if (!active || document.visibilityState !== 'visible') return
        const candidates = providers.filter((provider) => localAiProviderSessionPrewarmEligible({
          enabled,
          ownerKey,
          selectedProviderId,
          candidateProviderId: provider.id,
          selectedState,
          documentVisible: document.visibilityState === 'visible',
        }))
        void prewarmCandidates(candidates, ownerKey, () => active)
      }, LOCAL_AI_PROVIDER_SESSION_PREWARM_DELAY_MS)
    }
    const handleVisibility = () => schedule()
    document.addEventListener('visibilitychange', handleVisibility)
    schedule()
    return () => {
      active = false
      window.clearTimeout(timer)
      document.removeEventListener('visibilitychange', handleVisibility)
    }
  }, [
    enabled,
    ownerKey,
    providerKey,
    providers,
    selectedProviderId,
    selectedState?.lastError,
    selectedState?.loading,
    selectedState?.providerId,
    selectedState?.windowStatus,
  ])
}

async function prewarmCandidates(
  providers: LocalAiWebProvider[],
  ownerKey: string,
  active: () => boolean,
) {
  for (const provider of providers) {
    if (!active() || document.visibilityState !== 'visible') return
    const key = `${ownerKey}:${provider.id}`
    if (!localAiProviderSessionPrewarmGate.claim(key)) continue
    let succeeded = false
    try {
      let state = getCachedLocalAiWebSessionState(provider.id, ownerKey)
      if (!localAiWarmSessionReusable(state, provider.id)) {
        try {
          state = await getLocalAiWebSessionState(provider.id, ownerKey)
        } catch {
          state = null
        }
      }
      if (!active()) return
      if (!localAiWarmSessionReusable(state, provider.id)) {
        await openLocalAiWebSession(provider.id, ownerKey, { showWindow: false })
      }
      succeeded = true
    } catch {
      // Opportunistic only: selecting this provider retains the normal resume path.
    } finally {
      localAiProviderSessionPrewarmGate.release(key, succeeded)
    }
  }
}

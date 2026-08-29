import type { LocalAiWebSessionState } from './localAiBrowserApi'

export const LOCAL_AI_PROVIDER_SESSION_PREWARM_DELAY_MS = 2_400
export const LOCAL_AI_PROVIDER_SESSION_PREWARM_SUCCESS_COOLDOWN_MS = 5 * 60_000
export const LOCAL_AI_PROVIDER_SESSION_PREWARM_FAILURE_COOLDOWN_MS = 30_000

interface LocalAiProviderSessionPrewarmEligibility {
  enabled: boolean
  ownerKey: string
  selectedProviderId?: string
  candidateProviderId: string
  selectedState: LocalAiWebSessionState | null
  documentVisible: boolean
}

export function localAiProviderSessionPrewarmEligible({
  enabled,
  ownerKey,
  selectedProviderId,
  candidateProviderId,
  selectedState,
  documentVisible,
}: LocalAiProviderSessionPrewarmEligibility): boolean {
  return Boolean(
    enabled
      && ownerKey
      && selectedProviderId
      && candidateProviderId
      && candidateProviderId !== selectedProviderId
      && documentVisible
      && selectedState?.providerId === selectedProviderId
      && ['ready', 'minimized'].includes(selectedState.windowStatus)
      && !selectedState.loading
      && !selectedState.lastError,
  )
}

export class LocalAiProviderSessionPrewarmGate {
  private readonly active = new Set<string>()
  private readonly nextAttemptAt = new Map<string, number>()

  constructor(private readonly maxEntries = 32) {}

  claim(key: string, nowMs = Date.now()): boolean {
    if (!key || this.active.has(key) || (this.nextAttemptAt.get(key) ?? 0) > nowMs) return false
    this.active.add(key)
    return true
  }

  release(key: string, succeeded: boolean, nowMs = Date.now()) {
    this.active.delete(key)
    this.nextAttemptAt.delete(key)
    this.nextAttemptAt.set(
      key,
      nowMs + (succeeded
        ? LOCAL_AI_PROVIDER_SESSION_PREWARM_SUCCESS_COOLDOWN_MS
        : LOCAL_AI_PROVIDER_SESSION_PREWARM_FAILURE_COOLDOWN_MS),
    )
    while (this.nextAttemptAt.size > this.maxEntries) {
      const oldest = this.nextAttemptAt.keys().next().value
      if (typeof oldest !== 'string') break
      this.nextAttemptAt.delete(oldest)
    }
  }
}

export const localAiProviderSessionPrewarmGate = new LocalAiProviderSessionPrewarmGate()

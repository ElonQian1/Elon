import type {
  LocalAiAdapterAction,
  LocalAiMessageSnapshot,
  LocalAiWebSessionState,
} from './localAiBrowserApi'
import { localAiSnapshotIsStreaming } from './localAiPrivateStreamSignal'

export const LOCAL_AI_CAPABILITY_PREWARM_DELAY_MS = 900
export const LOCAL_AI_CAPABILITY_PREWARM_COOLDOWN_MS = 60_000

const REQUIRED_MODEL_ACTIONS: LocalAiAdapterAction[] = [
  'list_model_options',
  'collect_model_options',
]

interface LocalAiCapabilityPrewarmEligibility {
  providerId: string
  adapterActions: readonly LocalAiAdapterAction[]
  sessionState: LocalAiWebSessionState | null
  snapshot: LocalAiMessageSnapshot | null
  foregroundBlocked: boolean
}

export function localAiCapabilityPrewarmEligible({
  providerId,
  adapterActions,
  sessionState,
  snapshot,
  foregroundBlocked,
}: LocalAiCapabilityPrewarmEligibility): boolean {
  const supportsManifest = adapterActions.includes('snapshot_ui_manifest')
  const supportsModel = REQUIRED_MODEL_ACTIONS.every((action) => adapterActions.includes(action))
  return Boolean(
    providerId
      && sessionState?.providerId === providerId
      && (sessionState.windowStatus === 'ready' || sessionState.windowStatus === 'minimized')
      && sessionState.rendererStatus === 'active'
      && !sessionState.loading
      && !sessionState.lastError
      && snapshot?.composerReady
      && !localAiSnapshotIsStreaming(snapshot)
      && !foregroundBlocked
      && (supportsManifest || supportsModel),
  )
}

export function localAiCapabilityPrewarmSupportsModel(
  adapterActions: readonly LocalAiAdapterAction[],
): boolean {
  return REQUIRED_MODEL_ACTIONS.every((action) => adapterActions.includes(action))
}

export class LocalAiCapabilityPrewarmCooldown {
  private readonly attempts = new Map<string, number>()

  constructor(
    private readonly cooldownMs = LOCAL_AI_CAPABILITY_PREWARM_COOLDOWN_MS,
    private readonly maxEntries = 32,
  ) {}

  claim(key: string, nowMs = Date.now()): boolean {
    const previous = this.attempts.get(key)
    if (previous !== undefined && nowMs - previous < this.cooldownMs) return false
    this.attempts.delete(key)
    this.attempts.set(key, nowMs)
    while (this.attempts.size > this.maxEntries) {
      const oldest = this.attempts.keys().next().value
      if (typeof oldest !== 'string') break
      this.attempts.delete(oldest)
    }
    return true
  }
}

export const localAiCapabilityPrewarmCooldown = new LocalAiCapabilityPrewarmCooldown()

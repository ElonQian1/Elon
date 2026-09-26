const DEFAULT_MAX_ENTRIES = 12
const RESTART_KEY = 'elon.pc.webAiRestartDrafts.v1'
const MAX_RESTART_AGE_MS = 24 * 60 * 60 * 1000
type RestartStorage = Pick<Storage, 'getItem' | 'setItem' | 'removeItem'>

export const LOCAL_AI_PROVIDER_DRAFT_MAX_LENGTH = 12_000

// Normally memory-only. An explicit update checkpoints drafts locally once;
// neither draft text nor account identities enter MCP diagnostics or receipts.
export class LocalAiProviderDraftCache {
  private readonly entries = new Map<string, string>()
  private restartPending = false

  constructor(private readonly maxEntries = DEFAULT_MAX_ENTRIES, private readonly restartStorage?: RestartStorage) {
    if (!Number.isInteger(maxEntries) || maxEntries < 1) {
      throw new Error('Local AI draft cache capacity must be a positive integer.')
    }
    try {
      const raw = restartStorage?.getItem(RESTART_KEY)
      if (!raw || raw.length > 1_000_000) return
      const saved = JSON.parse(raw)
      if (!Number.isSafeInteger(saved.at) || saved.at > Date.now() || Date.now() - saved.at > MAX_RESTART_AGE_MS || !Array.isArray(saved.entries)) {
        restartStorage?.removeItem(RESTART_KEY)
        return
      }
      for (const entry of saved.entries.slice(0, maxEntries)) {
        if (Array.isArray(entry) && typeof entry[0] === 'string' && entry[0].length <= 2048 &&
            !entry[0].endsWith(':pending-owner') && typeof entry[1] === 'string') this.remember(entry[0], entry[1])
      }
      restartStorage?.removeItem(RESTART_KEY)
    } catch { /* Corrupt or unavailable local recovery must not prevent opening chat. */ }
  }

  read(identity: string): string {
    if (!identity) return ''
    const value = this.entries.get(identity) ?? ''
    if (!value) return ''
    this.entries.delete(identity)
    this.entries.set(identity, value)
    return value
  }

  remember(identity: string, value: string): void {
    if (!identity) return
    const draft = value.slice(0, LOCAL_AI_PROVIDER_DRAFT_MAX_LENGTH)
    this.entries.delete(identity)
    if (draft.trim()) this.entries.set(identity, draft)
    while (this.entries.size > this.maxEntries) {
      const oldest = this.entries.keys().next().value
      if (typeof oldest !== 'string') break
      this.entries.delete(oldest)
    }
    if (this.restartPending) this.checkpointForRestart()
  }

  checkpointForRestart(): void {
    if ([...this.entries.keys()].some(identity => identity.endsWith(':pending-owner'))) throw new Error('restart_draft_owner_unresolved')
    if (!this.restartStorage) {
      if (this.entries.size) throw new Error('restart_draft_storage_unavailable')
      return
    }
    const value = JSON.stringify({ at: Date.now(), entries: [...this.entries] })
    this.restartStorage.setItem(RESTART_KEY, value)
    if (this.restartStorage.getItem(RESTART_KEY) !== value) throw new Error('restart_draft_save_failed')
    this.restartPending = true
  }

  cancelRestartCheckpoint(): void {
    this.restartPending = false
    try { this.restartStorage?.removeItem(RESTART_KEY) } catch { /* No draft content in diagnostics. */ }
  }

  claimPending(providerId: string, ownerKey: string): string {
    const target = localAiProviderDraftIdentity(providerId, ownerKey)
    const pending = localAiProviderDraftIdentity(providerId, '')
    const value = this.read(pending)
    if (value) {
      this.entries.delete(pending)
      this.remember(target, value)
      return value
    }
    return this.read(target)
  }
}

function restartStorage(): RestartStorage | undefined {
  try { return typeof window === 'undefined' ? undefined : window.localStorage } catch { return undefined }
}

export const localAiProviderDraftCache = new LocalAiProviderDraftCache(DEFAULT_MAX_ENTRIES, restartStorage())

export function localAiProviderDraftIdentity(providerId: string, ownerKey: string): string {
  const provider = providerId.trim()
  if (!provider) return ''
  const owner = ownerKey.trim()
  return owner
    ? `${provider.length}:${provider}:${owner.length}:${owner}`
    : `${provider.length}:${provider}:pending-owner`
}

export function mergeLocalAiRecoveredDraft(prompt: string, currentDraft: string): string {
  const recovered = prompt.trim()
  const current = currentDraft.trim()
  if (!recovered) return currentDraft.slice(0, LOCAL_AI_PROVIDER_DRAFT_MAX_LENGTH)
  if (!current) return prompt.slice(0, LOCAL_AI_PROVIDER_DRAFT_MAX_LENGTH)
  if (current === recovered) return currentDraft.slice(0, LOCAL_AI_PROVIDER_DRAFT_MAX_LENGTH)
  return `${prompt}\n\n${currentDraft}`.slice(0, LOCAL_AI_PROVIDER_DRAFT_MAX_LENGTH)
}

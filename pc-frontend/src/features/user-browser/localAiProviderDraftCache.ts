const DEFAULT_MAX_ENTRIES = 12

export const LOCAL_AI_PROVIDER_DRAFT_MAX_LENGTH = 12_000

// Deliberately memory-only: Win keeps drafts responsive across provider/session
// rebinding without adding them to the persistent semantic snapshot cache.
export class LocalAiProviderDraftCache {
  private readonly entries = new Map<string, string>()

  constructor(private readonly maxEntries = DEFAULT_MAX_ENTRIES) {
    if (!Number.isInteger(maxEntries) || maxEntries < 1) {
      throw new Error('Local AI draft cache capacity must be a positive integer.')
    }
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

export const localAiProviderDraftCache = new LocalAiProviderDraftCache()

export function localAiProviderDraftIdentity(providerId: string, ownerKey: string): string {
  const provider = providerId.trim()
  if (!provider) return ''
  const owner = ownerKey.trim()
  return owner
    ? `${provider.length}:${provider}:${owner.length}:${owner}`
    : `${provider.length}:${provider}:pending-owner`
}

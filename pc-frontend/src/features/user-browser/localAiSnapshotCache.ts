import type { LocalAiWebSessionState } from './localAiBrowserApi'

const DEFAULT_MAX_ENTRIES = 8

export class LocalAiSnapshotCache {
  private readonly entries = new Map<string, LocalAiWebSessionState>()

  constructor(private readonly maxEntries = DEFAULT_MAX_ENTRIES) {
    if (!Number.isInteger(maxEntries) || maxEntries < 1) {
      throw new Error('Local AI snapshot cache capacity must be a positive integer.')
    }
  }

  get size() {
    return this.entries.size
  }

  read(providerId: string, ownerKey: string): LocalAiWebSessionState | null {
    const key = cacheKey(providerId, ownerKey)
    const state = this.entries.get(key)
    if (!state) return null
    this.entries.delete(key)
    this.entries.set(key, state)
    return state
  }

  remember(providerId: string, ownerKey: string, state: LocalAiWebSessionState): void {
    if (state.providerId !== providerId) {
      throw new Error('Local AI snapshot provider identity mismatch.')
    }
    const key = cacheKey(providerId, ownerKey)
    this.entries.delete(key)
    this.entries.set(key, state)
    while (this.entries.size > this.maxEntries) {
      const oldest = this.entries.keys().next().value
      if (typeof oldest !== 'string') break
      this.entries.delete(oldest)
    }
  }

  forget(providerId: string, ownerKey: string): void {
    this.entries.delete(cacheKey(providerId, ownerKey))
  }

  clear(): void {
    this.entries.clear()
  }
}

export const localAiSnapshotCache = new LocalAiSnapshotCache()

function cacheKey(providerId: string, ownerKey: string): string {
  const provider = providerId.trim()
  const owner = ownerKey.trim()
  if (!provider || !owner) throw new Error('Local AI snapshot cache identity is incomplete.')
  return `${provider.length}:${provider}:${owner.length}:${owner}`
}

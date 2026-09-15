import type { ActiveConversation, Friend, FriendGroup, SocialMessage } from './socialMessageTypes'

const PREFIX = 'elon_social_cache_v1:'
const MAX_SIZE = 1_500_000
const MAX_AGE = 7 * 86400_000
export interface SocialSnapshot {
  friends?: Friend[]
  groups?: FriendGroup[]
  active: ActiveConversation | null
  messages: Record<string, SocialMessage[]>
  drafts: Record<string, string>
}
export const emptySocialSnapshot = (): SocialSnapshot => ({ active: null, messages: {}, drafts: {} })
export const conversationId = (item: ActiveConversation) => `${item.kind}:${item.id}`
export const socialCacheKey = (server: string, userId: string) => `${PREFIX}${encodeURIComponent(server)}:${encodeURIComponent(userId)}`

export function readSocialCache(key: string): SocialSnapshot {
  try {
    const raw = localStorage.getItem(key)
    if (!raw || raw.length > MAX_SIZE) return emptySocialSnapshot()
    const data = JSON.parse(raw)
    if (data.version !== 1 || !Number.isFinite(data.savedAt) || Date.now() - data.savedAt > MAX_AGE) return emptySocialSnapshot()
    const listsValid = (list: unknown, field: string) => list === undefined || (Array.isArray(list) && list.every(item => item && typeof item.id === 'string' && typeof item[field] === 'string'))
    if (!listsValid(data.friends, 'account') || !listsValid(data.groups, 'name')) return emptySocialSnapshot()
    const result: SocialSnapshot = { friends: data.friends, groups: data.groups, active: null, messages: {}, drafts: {} }
    if (data.active && ['friend', 'group'].includes(data.active.kind) && typeof data.active.id === 'string') result.active = data.active
    for (const [id, rows] of Object.entries(data.messages ?? {})) {
      if (/^(friend|group):/.test(id) && Array.isArray(rows) && rows.every(m => m && typeof m.id === 'string' && typeof m.content === 'string' && typeof m.created_at === 'string')) result.messages[id] = rows.slice(-120)
    }
    for (const [id, draft] of Object.entries(data.drafts ?? {})) {
      if (/^(friend|group):/.test(id) && typeof draft === 'string') result.drafts[id] = draft.slice(0, 16000)
    }
    return result
  } catch { return emptySocialSnapshot() }
}

export function writeSocialCache(key: string, snapshot: SocialSnapshot): boolean {
  // Keep active messages first; evict least recently visited conversations on quota pressure.
  const activeKey = snapshot.active && conversationId(snapshot.active)
  const keys = Object.keys(snapshot.messages).reverse().sort((a, b) => a === activeKey ? -1 : b === activeKey ? 1 : 0).slice(0, 20)
  const messages = Object.fromEntries(keys.map(id => [id, snapshot.messages[id].filter(m => !m.id.startsWith('tmp-')).slice(-120)]))
  const drafts = Object.fromEntries(Object.entries(snapshot.drafts).filter(([, text]) => text).slice(-40))
  const data = { version: 1, savedAt: Date.now(), friends: snapshot.friends?.slice(0, 500), groups: snapshot.groups?.slice(0, 500), active: snapshot.active, messages, drafts }
  try {
    while (true) {
      const raw = JSON.stringify(data)
      if (raw.length <= MAX_SIZE) {
        try { localStorage.setItem(key, raw); return true } catch { /* Evict and retry within a fixed bound. */ }
      }
      const oldest = keys.pop()
      if (!oldest) break
      delete messages[oldest]
    }
  } catch { /* Disabled storage must not stop the live conversation. */ }
  return false
}

export function clearSocialCaches() {
  try {
    const keys = Array.from({ length: localStorage.length }, (_, i) => localStorage.key(i))
    keys.forEach(key => { if (key?.startsWith(PREFIX)) localStorage.removeItem(key) })
  } catch { /* Logout remains available with disabled storage. */ }
}

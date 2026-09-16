import '../../../../android/app/src/main/assets/social_link_read_adapter.js'
import type { LinkPreview } from './socialLinks'

export interface ReadPreview { schema: 1; original: string; url: string; article: true; title: string; author: string; image: string | null }
declare global {
  var ElonSocialReadAdapter: { identity(value: string): string | null; readingUrl(value: string): string; validate(original: string, value: unknown): ReadPreview | null }
}
const storageKey = 'elon-social-read-preview-v1', ttl = 24 * 3600000
interface Entry { scope: string; original: string; saved: number; value: ReadPreview }
function entries(): Entry[] {
  try {
    const raw = localStorage.getItem(storageKey) || '[]'; if (raw.length > 2 * 1024 * 1024) return []
    const list: unknown = JSON.parse(raw); if (!Array.isArray(list)) return []
    return list.slice(-128).filter(e => e && typeof e.scope === 'string' && typeof e.original === 'string' && Number.isFinite(e.saved) && e.saved <= Date.now() && Date.now() - e.saved < ttl)
  } catch { return [] }
}
export function fromRead(original: LinkPreview, value: unknown): LinkPreview | null {
  const read = ElonSocialReadAdapter.validate(original.url, value)
  return read ? { ...original, title: read.title, author: read.author, image: read.image, status: 'ready' } : null
}
export function cachedRead(scope: string, original: LinkPreview): { preview: LinkPreview; expires: number } | null {
  const entry = entries().find(e => e.scope === scope && e.original === original.url)
  const preview = entry ? fromRead(original, entry.value) : null
  return preview && entry ? { preview, expires: entry.saved + ttl } : null
}
export function rememberRead(scope: string, original: LinkPreview, value: unknown): LinkPreview | null {
  const read = ElonSocialReadAdapter.validate(original.url, value)
  const preview = fromRead(original, read); if (!read || !preview) return null
  const list = entries().filter(e => e.scope !== scope || e.original !== original.url)
  list.push({ scope, original: original.url, saved: Date.now(), value: read })
  try { localStorage.setItem(storageKey, JSON.stringify(list.slice(-128))) } catch { /* memory cache still works */ }
  return preview
}

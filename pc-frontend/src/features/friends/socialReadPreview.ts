import '../../../../android/app/src/main/assets/social_link_read_adapter.js'
import type { LinkPreview } from './socialLinks'

export interface ReadPreview { schema: 1; original: string; url: string; article: true; title: string; author: string; image: string | null }
declare global {
  var ElonSocialReadAdapter: { identity(value: string): string | null; readingUrl(value: string): string; readSource(value: string, player?: string): string; cacheAlias(original: string, source: string): boolean; validate(original: string, value: unknown): ReadPreview | null }
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
  const read = ElonSocialReadAdapter.validate(readSource(original), value)
  return read ? { ...original, title: read.title, author: read.author, image: read.image, status: 'ready' } : null
}
export function readSource(preview: LinkPreview) { return ElonSocialReadAdapter.readSource(preview.url, preview.embed?.url) }
export function readBackComplete(original: LinkPreview, value: unknown): boolean {
  const read = ElonSocialReadAdapter.validate(readSource(original), value)
  const video = /^(bilibili|douyin|xiaohongshu):/.test(ElonSocialReadAdapter.identity(readSource(original)) || '')
  return !!read && (!video || !!read.image)
}
export function forgetRead(scope: string, original: LinkPreview) {
  try { localStorage.setItem(storageKey, JSON.stringify(entries().filter(e => e.scope !== scope || e.original !== original.url))) } catch { /* optional cache */ }
}
export function cachedRead(scope: string, original: LinkPreview): { preview: LinkPreview; expires: number } | null {
  const entry = entries().find(e => e.scope === scope && e.original === original.url)
  const source = entry?.value?.original
  const read = typeof source === 'string' && ElonSocialReadAdapter.cacheAlias(original.url, source) ? ElonSocialReadAdapter.validate(source, entry?.value) : null
  const embed = original.embed || (read && ElonSocialReadAdapter.identity(read.original)?.startsWith('douyin:') ? ElonSocialLinks.embed(read.original) : null)
  const preview = read ? { ...original, embed, title: read.title, author: read.author, image: read.image, status: 'ready' as const } : null
  return preview && entry ? { preview, expires: entry.saved + ttl } : null
}
export function rememberRead(scope: string, original: LinkPreview, value: unknown): LinkPreview | null {
  const read = ElonSocialReadAdapter.validate(readSource(original), value)
  const preview = fromRead(original, read); if (!read || !preview) return null
  const previous = cachedRead(scope, original)
  if (!read.image && previous?.preview.image) return previous.preview
  const list = entries().filter(e => e.scope !== scope || e.original !== original.url)
  list.push({ scope, original: original.url, saved: Date.now(), value: read })
  try { localStorage.setItem(storageKey, JSON.stringify(list.slice(-128))) } catch { /* memory cache still works */ }
  return preview
}

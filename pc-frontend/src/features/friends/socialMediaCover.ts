import { getDesktopInvoke } from '../shell/desktopShell'
import { readSource, cachedRead, rememberRead } from './socialReadPreview'
import type { LinkPreview } from './socialLinks'

type Job = { key: string; preview: LinkPreview; current: () => boolean; resolve: (value: unknown) => void }
const queue: Job[] = []
const pending = new Map<string, Promise<unknown>>()
const failures = new Map<string, number>()
let running = false

async function drain() {
  if (running) return
  running = true
  try {
    while (queue.length) {
      const job = queue.shift()!
      let value: unknown = null
      try {
        if (job.current()) value = await getDesktopInvoke()?.('get_social_media_read_preview', { url: readSource(job.preview) })
      } catch { /* Older shells and site challenges retain the original-page action. */ }
      if (!value && job.current()) {
        if (failures.size >= 128) failures.delete(failures.keys().next().value!)
        failures.set(job.key, Date.now() + 30000)
      }
      pending.delete(job.key)
      job.resolve(job.current() ? value : null)
    }
  } finally { running = false }
}

export function readMediaCover(scope: string, preview: LinkPreview, current: () => boolean): Promise<unknown> {
  const source = readSource(preview)
  if (!getDesktopInvoke() || !['douyin', 'xiaohongshu'].includes(ElonSocialReadAdapter.identity(source)?.split(':')[0] || '') || !current()) return Promise.resolve(null)
  const key = scope + '\n' + source
  const existing = pending.get(key)
  if (existing) return existing
  if (queue.length >= 8 || (failures.get(key) || 0) > Date.now()) return Promise.resolve(null)
  const result = new Promise<unknown>(resolve => queue.push({ key, preview, current, resolve }))
  pending.set(key, result)
  void drain()
  return result
}

/** Called only after the shared card's visibility-triggered load, never for all chat history. */
export async function enrichMediaCover(scope: string, preview: LinkPreview, current: () => boolean) {
  if (preview.image || !current()) return
  const cached = cachedRead(scope, preview)
  if (cached?.preview.image) { ElonSocialLinks.remember(scope, cached.preview, cached.expires); return }
  const value = await readMediaCover(scope, preview, current)
  if (!value || !current()) return
  const updated = rememberRead(scope, preview, value)
  if (updated?.image) ElonSocialLinks.remember(scope, updated)
}

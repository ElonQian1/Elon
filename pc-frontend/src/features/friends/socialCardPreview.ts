import { getDesktopInvoke } from '../shell/desktopShell'
import { previewApi } from './socialReadBack'
import type { LinkPreview } from './socialLinks'

// The native endpoint accepts only a BV id; never send cloud auth or an arbitrary URL.
export async function cardPreviewApi(path: string, init: RequestInit): Promise<unknown> {
  const invoke = getDesktopInvoke()
  let item: LinkPreview | undefined
  const native = async (preview: LinkPreview) => {
    if (!invoke || preview.embed?.kind !== 'bilibili' || !/^BV[A-Za-z0-9]{10}$/.test(preview.embed.id)) return null
    try {
      const value = await invoke<{ title: string; author: string; image: string } | null>('get_bilibili_public_preview', { bvid: preview.embed.id })
      if (init.signal?.aborted) return null
      return value ? { ...preview, ...value, status: 'ready' } : null
    } catch { return null }
  }
  if (invoke && path === '/api/me/link-preview' && typeof init.body === 'string') {
    try {
      item = ElonSocialLinks.links(JSON.parse(init.body).url)[0]
      const value = item ? await native(item) : null
      if (value) return value
    } catch { /* malformed input is left to the normal endpoint */ }
  }
  if (init.signal?.aborted) throw new DOMException('Aborted', 'AbortError')
  const value = await previewApi(path, init)
  // Short links only have a BV id after the existing server resolver has followed them.
  if (item?.site === '哔哩哔哩' && !item.embed && value?.url === item.url && !value.image && !value.cover_data_url) return await native({ ...item, embed: value.embed }) || value
  return value
}

/** Read-only MCP evidence from the same WebView image decoder used by cards. */
export async function inspectCardPreviews(links: string[]) {
  const results = []
  for (const url of links.slice(0, 2)) {
    try {
      const value = await cardPreviewApi('/api/me/link-preview', { method: 'POST', body: JSON.stringify({ url }), signal: AbortSignal.timeout(15000) }) as LinkPreview & { cover_data_url?: string }
      const image = value.cover_data_url || value.image
      const decoded = image ? await new Promise<boolean>(resolve => {
        const img = new Image(); const timer = setTimeout(() => finish(false), 6000)
        function finish(ok: boolean) { clearTimeout(timer); img.onload = null; img.onerror = null; img.removeAttribute('src'); resolve(ok) }
        img.referrerPolicy = 'no-referrer'; img.onload = () => finish(img.naturalWidth > 0); img.onerror = () => finish(false); img.src = image
      }) : false
      results.push({ status: value.status, site: value.site, has_image: !!image, image_decoded: decoded })
    } catch { results.push({ status: 'unavailable', has_image: false, image_decoded: false }) }
  }
  return results
}

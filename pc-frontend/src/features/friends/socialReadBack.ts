import { getAuthToken } from '../../api/client'
import { resolveApiUrl } from '../../api/runtime'
import type { LinkPreview } from './socialLinks'
import { rememberRead } from './socialReadPreview'

export async function previewApi(path: string, init: RequestInit) {
  const token = getAuthToken()
  const response = await fetch(resolveApiUrl(path), { ...init, headers: { 'Content-Type': 'application/json', ...(token ? { Authorization: `Bearer ${token}` } : {}) } })
  if (!response.ok) throw new Error('预览暂不可用')
  return response.json()
}

// Share what this member actually saw so readers whose server fetch was blocked get a real card.
function reportRead(url: string, value: unknown) {
  const read = ElonSocialReadAdapter.validate(url, value); if (!read) return
  previewApi('/api/me/link-preview/report', { method: 'POST', body: JSON.stringify({ url, read }) }).catch(() => undefined)
}

/** Read-back from the original page: update local cache, live cards and the server. */
export function applyReadBack(scope: string, preview: LinkPreview, value: unknown) {
  const updated = rememberRead(scope, preview, value)
  if (!updated) return false
  ElonSocialLinks.remember(scope, updated)
  reportRead(preview.url, value)
  return true
}

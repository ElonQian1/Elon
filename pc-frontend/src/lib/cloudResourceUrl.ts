import { resolveApiUrl } from '../api/runtime'

/** Resolve server-owned media and downloads against the API host in local Win mode. */
export function cloudResourceUrl(value?: string | null): string {
  const raw = value?.trim() || ''
  if (!raw || raw.startsWith('//') || /[\\\u0000-\u001f]/.test(raw)) return ''
  if (!raw.startsWith('/') && !/^https?:\/\//i.test(raw)) return ''
  try {
    const base = new URL(resolveApiUrl('/api/runtime'), location.href)
    const url = new URL(raw, base)
    if (!/^https?:$/.test(url.protocol) || url.username || url.password) return ''
    return url.toString()
  } catch {
    return ''
  }
}

export interface SourceLink { version: 1; url: string; method: 'qr' | 'share' }
export function webSourceUrl(value: unknown): string | null {
  if (typeof value !== 'string' || !/^https?:\/\//i.test(value) || new TextEncoder().encode(value).length > 4096 || /[\s\u0000-\u001f\u007f\\]/.test(value)) return null
  try { const url = new URL(value); return ['https:', 'http:'].includes(url.protocol) && url.host && !url.username && !url.password ? value : null } catch { return null }
}
export function sourceLink(value: unknown): SourceLink | null {
  if (!value || typeof value !== 'object') return null
  const item = value as SourceLink
  return item.version === 1 && ['qr', 'share'].includes(item.method) && webSourceUrl(item.url) ? item : null
}
export function sourceLabel(url: string): string {
  try { const u = new URL(url); return u.hostname === 'mp.weixin.qq.com' && (u.pathname === '/s' || u.pathname.startsWith('/s/')) ? '阅读原文' : '打开链接' } catch { return '打开链接' }
}

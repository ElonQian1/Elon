export type GrantExpiryPreset = '7' | '30' | '90' | '365' | 'none'

export const grantExpiryOptions: Array<{ value: GrantExpiryPreset; label: string }> = [
  { value: '7', label: '7 天' },
  { value: '30', label: '30 天' },
  { value: '90', label: '90 天' },
  { value: '365', label: '1 年' },
  { value: 'none', label: '长期有效' },
]

export function grantExpiresAt(
  preset: GrantExpiryPreset,
  now = new Date(),
): string | undefined {
  if (preset === 'none') return undefined
  const expiresAt = new Date(now.getTime())
  expiresAt.setUTCDate(expiresAt.getUTCDate() + Number(preset))
  return expiresAt.toISOString()
}

export function isGrantExpired(expiresAt?: string, now = Date.now()): boolean {
  if (!expiresAt) return false
  const timestamp = Date.parse(expiresAt)
  return !Number.isFinite(timestamp) || timestamp <= now
}

export function grantExpiryLabel(expiresAt?: string, now = Date.now()): string {
  if (!expiresAt) return '长期有效'
  const timestamp = Date.parse(expiresAt)
  if (!Number.isFinite(timestamp)) return '有效期异常'
  const label = new Date(timestamp).toLocaleString('zh-CN')
  return timestamp <= now ? `已于 ${label} 过期` : `有效至 ${label}`
}

export function grantTermsLabel(terms: {
  expires_at?: string
  max_invocations?: number
  max_amount_micros?: number
}): string {
  const calls = terms.max_invocations == null ? '次数不限' : `最多 ${terms.max_invocations} 次`
  const amount = terms.max_amount_micros == null
    ? '金额不限'
    : `最多 ${(terms.max_amount_micros / 1_000_000).toFixed(2)} 元`
  return `${grantExpiryLabel(terms.expires_at)} · ${calls} · ${amount}`
}

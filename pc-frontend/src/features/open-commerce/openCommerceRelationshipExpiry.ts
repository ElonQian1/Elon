export type RelationshipExpiryPreset = '30' | '90' | '365'
export type RelationshipExpiryState = 'healthy' | 'expiring' | 'expired'

export const relationshipRenewalWindowDays = 14

export const relationshipExpiryOptions: Array<{
  value: RelationshipExpiryPreset
  label: string
}> = [
  { value: '30', label: '30 天' },
  { value: '90', label: '90 天' },
  { value: '365', label: '1 年' },
]

export function relationshipExpiresAt(
  preset: RelationshipExpiryPreset,
  now = new Date(),
): string {
  const expiresAt = new Date(now.getTime())
  expiresAt.setUTCDate(expiresAt.getUTCDate() + Number(preset))
  return expiresAt.toISOString()
}

export function relationshipExpiryLabel(expiresAt: string): string {
  const timestamp = Date.parse(expiresAt)
  if (!Number.isFinite(timestamp)) return '有效期异常'
  return `有效至 ${new Date(timestamp).toLocaleString('zh-CN')}`
}

export function relationshipExpiryState(
  expiresAt: string,
  now = new Date(),
): RelationshipExpiryState {
  const timestamp = Date.parse(expiresAt)
  if (!Number.isFinite(timestamp) || timestamp <= now.getTime()) return 'expired'
  const renewalWindow = relationshipRenewalWindowDays * 24 * 60 * 60 * 1000
  return timestamp <= now.getTime() + renewalWindow ? 'expiring' : 'healthy'
}

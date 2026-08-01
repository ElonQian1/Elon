export type RelationshipExpiryPreset = '30' | '90' | '365'

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

export function errorText(error: unknown) {
  if (error instanceof Error) return error.message
  if (error && typeof error === 'object' && 'message' in error) {
    return String(error.message)
  }
  return '操作失败，请稍后重试'
}

export function parseJsonObject(value: string): Record<string, unknown> {
  const parsed = JSON.parse(value) as unknown
  if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object') {
    throw new Error('请输入 JSON object')
  }
  return parsed as Record<string, unknown>
}

export function formatMicros(value: number, currency = 'CNY') {
  return `${(value / 1_000_000).toFixed(6)} ${currency}`
}

export function splitValues(value: string) {
  return value
    .split(/[,，]/)
    .map((item) => item.trim())
    .filter(Boolean)
}

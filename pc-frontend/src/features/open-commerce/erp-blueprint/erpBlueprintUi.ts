export function errorMessage(error: unknown) {
  if (error instanceof Error) return error.message
  return String(error)
}

export function shortDate(value: string) {
  return value ? new Date(value).toLocaleString() : '—'
}

export function isNewerVersion(candidate: string, current: string) {
  const tuple = (value: string) => value.split(/[+-]/, 1)[0].split('.').map((part) => Number(part) || 0)
  const left = tuple(candidate)
  const right = tuple(current)
  for (let index = 0; index < 3; index += 1) {
    if ((left[index] ?? 0) !== (right[index] ?? 0)) return (left[index] ?? 0) > (right[index] ?? 0)
  }
  return false
}

export const classificationLabels: Record<string, string> = {
  existing: '已有能力',
  composition: '组合实现',
  private_extension: '商户私有扩展',
  candidate_common: '通用候选',
}

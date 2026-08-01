import type { OpenCommerceGrant } from './openCommerceTypes'

export function optionalPositiveInteger(value: string, label: string): number | undefined {
  if (!value.trim()) return undefined
  const parsed = Number(value)
  if (!Number.isSafeInteger(parsed) || parsed <= 0) throw new Error(`${label}必须是正整数`)
  return parsed
}

export function optionalYuanMicros(value: string): number | undefined {
  if (!value.trim()) return undefined
  const parsed = Number(value)
  const micros = Math.round(parsed * 1_000_000)
  if (!Number.isFinite(parsed) || parsed <= 0 || !Number.isSafeInteger(micros)) {
    throw new Error('总预算必须大于 0')
  }
  return micros
}

export function grantBudgetLabel(grant: OpenCommerceGrant): string {
  const calls = grant.max_invocations == null
    ? '次数不限'
    : `${grant.used_invocations}/${grant.max_invocations} 次`
  const amount = grant.max_amount_micros == null
    ? '金额不限'
    : `${formatYuan(grant.used_amount_micros)}/${formatYuan(grant.max_amount_micros)} 元`
  return `${calls} · ${amount}`
}

function formatYuan(micros: number): string {
  return (micros / 1_000_000).toFixed(2)
}

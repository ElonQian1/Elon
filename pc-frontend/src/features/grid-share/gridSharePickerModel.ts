import type { GridFacts } from '../grid-chat/gridChatSnapshot'

export type GridSort = 'profit-desc' | 'profit-asc' | 'symbol' | 'ended-desc'
const decimal = /^-?(0|[1-9][0-9]{0,29})(\.[0-9]{1,20})?$/
export function knownDecimal(value: unknown): value is string { return typeof value === 'string' && decimal.test(value) }
function units(value: string): bigint {
  const [whole, fraction = ''] = value.replace(/^-/, '').split('.')
  return BigInt(whole + fraction.padEnd(20, '0')) * (value.startsWith('-') ? -1n : 1n)
}
export function profitTone(value: unknown): 'gain' | 'loss' | 'neutral' {
  if (!knownDecimal(value)) return 'neutral'
  const amount = units(value)
  return amount > 0n ? 'gain' : amount < 0n ? 'loss' : 'neutral'
}
export function displayProfit(value: unknown): string {
  if (!knownDecimal(value)) return '未读取'
  const tone = profitTone(value), [whole, fraction = ''] = value.replace(/^-/, '').split('.')
  const prefix = tone === 'gain' ? '+' : tone === 'loss' ? '−' : ''
  if (whole === '0' && tone !== 'neutral' && fraction.slice(0, 4).replace(/0/g, '') === '') return `${prefix}<0.0001`
  const digits = fraction.slice(0, 4).replace(/0+$/, '')
  return prefix + whole.replace(/\B(?=(\d{3})+(?!\d))/g, ',') + (digits ? `.${digits}` : '')
}
export function selectGridRows(rows: GridFacts[], query: string, sort: GridSort): GridFacts[] {
  const search = query.trim().toLocaleUpperCase()
  const result = rows.filter(row => !search || `${row.symbol} ${row.id}`.toLocaleUpperCase().includes(search))
  return result.sort((a, b) => {
    if (sort === 'ended-desc') return Number(b.end ?? 0) - Number(a.end ?? 0) || (a.id ?? '').localeCompare(b.id ?? '')
    if (sort !== 'symbol') {
      const ak = knownDecimal(a.profit), bk = knownDecimal(b.profit)
      if (ak !== bk) return ak ? -1 : 1
      if (ak && bk) {
        const difference = units(a.profit!) - units(b.profit!)
        if (difference !== 0n) return (difference > 0n ? 1 : -1) * (sort === 'profit-desc' ? -1 : 1)
      }
    }
    return (a.symbol ?? '').localeCompare(b.symbol ?? '') || (a.id ?? '').localeCompare(b.id ?? '')
  })
}
export function tokenSymbol(symbol: string) { return symbol.endsWith('USDT') ? symbol.slice(0, -4) : symbol }
export const directionName = (direction: string | null | undefined) => ({ LONG: '做多', SHORT: '做空', NEUTRAL: '中性' }[direction ?? ''] ?? '方向未读取')

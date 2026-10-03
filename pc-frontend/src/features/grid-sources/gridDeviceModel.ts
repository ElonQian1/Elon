import { sha256 } from '@noble/hashes/sha2.js'
import { bytesToHex, utf8ToBytes } from '@noble/hashes/utils.js'
import type { ExchangeWebObservation } from '../exchange-webview/exchangeWebviewApi'
import { object, gridRows } from '../grid-chat/gridChatSnapshot'

export const snapshotSchema = 'yilong.grid_device_snapshot.v1'
export type Platform = 'android' | 'windows'
export interface DeviceSnapshot {
  schema: string; platform: Platform; device_id: string; sequence: number
  account: string | null; account_kind: string; observed_at_ms: number; fresh_until_ms: number
  status: 'fresh' | 'unavailable'; rows: Record<string, unknown>[]
}
export interface DeviceSource { source_id: string; snapshot: DeviceSnapshot }
const rowFields = ['id', 'symbol', 'status', 'direction', 'spacing', 'lower', 'upper', 'count', 'leverage', 'profit', 'created']
const metrics = ['initialNotional', 'investment', 'matchedPnl', 'fundingFee', 'fee', 'adjustmentAmount', 'perGridQty',
  'perGridQuoteQty', 'triggerPrice', 'stopUpper', 'stopLower', 'stopTpPnl', 'stopSlPnl', 'trailingUpPrice',
  'trailingDownPrice', 'closeOnStop', 'autoAddMargin', 'trailingUp', 'trailingDown', 'matchedCount', 'ended', 'marginType', 'orderCurrency']
export function accountDigest(value: string) { return bytesToHex(sha256(utf8ToBytes(value))) }
export function deviceSnapshot(state: ExchangeWebObservation | null, device: string, sequence: number, now = Date.now()): DeviceSnapshot {
  const empty: DeviceSnapshot = { schema: snapshotSchema, platform: 'windows', device_id: device, sequence,
    account: null, account_kind: 'unknown', observed_at_ms: now, fresh_until_ms: now, status: 'unavailable', rows: [] }
  if (!state || !state.windowOpen || !state.adapterReady || !state.list) return empty
  const list = gridRows(state), age = now - list.observedAtMs
  if (!Number.isSafeInteger(list.observedAtMs) || age < -5000 || age >= 300000) return empty
  const rows = (object(state.list).rows as unknown[]).map(raw => {
    const row = object(raw), detail = object(state.details[String(row.id)]), candidate = object(detail.row)
    const detailed = detail.schema === 'yilong.binance_observation.v1' && detail.account === list.source.account
      && detail.account_kind === list.source.accountKind && candidate.id === row.id && candidate.symbol === row.symbol
    const merged = detailed ? { ...row, ...Object.fromEntries(Object.entries(candidate).filter(([, value]) => value !== null)) } : row
    const values = { ...object(row.metrics), ...(detailed ? object(candidate.metrics) : {}) }
    return { ...Object.fromEntries(rowFields.map(key => [key, merged[key] ?? null])), detail: detailed,
      metrics: Object.fromEntries(metrics.filter(key => key in values).map(key => [key, values[key]])) }
  })
  return { ...empty, account: accountDigest(list.source.account), account_kind: list.source.accountKind,
    observed_at_ms: list.observedAtMs, fresh_until_ms: list.observedAtMs + 300000, status: 'fresh', rows }
}
export function parseDeviceSources(raw: string): DeviceSource[] {
  if (new TextEncoder().encode(raw).length > 1048576 + 16384) throw Error('同步响应过大')
  const value = JSON.parse(raw)
  if (value.schema !== 'yilong.grid_device_sources.v1' || !Array.isArray(value.sources) || value.sources.length > 16) throw Error('同步协议不匹配')
  const sources: DeviceSource[] = value.sources
  const ids = new Set<string>()
  for (const source of sources) {
    const s = source.snapshot
    if (!/^[0-9a-f]{64}$/.test(source.source_id) || ids.has(source.source_id) || s?.schema !== snapshotSchema
      || !['android', 'windows'].includes(s.platform) || !/^[0-9a-f]{8}(-[0-9a-f]{4}){3}-[0-9a-f]{12}$/.test(s.device_id)
      || !Number.isSafeInteger(s.sequence) || s.sequence < 1 || !['primary', 'sub', 'unknown'].includes(s.account_kind)
      || !['fresh', 'unavailable'].includes(s.status) || !Number.isSafeInteger(s.observed_at_ms) || s.observed_at_ms < 1
      || s.observed_at_ms > Date.now() + 30000 || !Number.isSafeInteger(s.fresh_until_ms)
      || s.fresh_until_ms < s.observed_at_ms || s.fresh_until_ms - s.observed_at_ms > 300000
      || (s.status === 'fresh' ? !/^[0-9a-f]{64}$/.test(s.account ?? '') : s.account !== null)
      || !Array.isArray(s.rows) || s.rows.length > 500 || (s.status === 'unavailable' && s.rows.length > 0)) throw Error('来源数据无法验证')
    if (new Set(s.rows.map(row => row.id)).size !== s.rows.length) throw Error('策略数据重复')
    ids.add(source.source_id)
  }
  return sources
}
export function fresh(source: DeviceSnapshot, now: number) { return source.status === 'fresh' && now >= source.observed_at_ms && now < source.fresh_until_ms }

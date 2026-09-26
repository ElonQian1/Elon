import { GRID_FIELDS, projectGrid, type GridFacts } from '../grid-chat/gridChatSnapshot'
import type { ResearchCommand } from './types'

export const BINANCE_GRID_KINDS = ['binance_grid_list', 'binance_grid_detail'] as const
export type BinanceGridKind = typeof BINANCE_GRID_KINDS[number]
export const GRID_READ_TTL = 5 * 60_000
export const GRID_READ_ERRORS = ['invalid_request', 'request_conflict', 'request_expired', 'busy',
  'session_unavailable', 'identity_unavailable', 'context_changed', 'read_failed', 'strategy_not_found'] as const
export type GridReadError = typeof GRID_READ_ERRORS[number]
export const GRID_LIST_FIELDS = ['id', 'symbol', 'status', 'direction', 'leverage', 'lower', 'upper', 'count', 'spacing'] as const
export interface GridReadRequest { kind: BinanceGridKind; id: string; strategy?: string; offset: number; limit: number; start: boolean }
export type GridListRow = Pick<GridFacts, typeof GRID_LIST_FIELDS[number]>
export interface GridReadResult {
  schema: 'yilong.browser-research.result.v1'
  kind: BinanceGridKind
  reader: {
    schema: 'yilong.binance-grid-read.v1'; request_id: string; status: 'pending' | 'ready' | 'failed'
    error?: GridReadError; observed_at_ms?: number; expires_at_ms?: number
    items?: GridListRow[]; row?: GridFacts; total?: number; offset?: number; next_offset?: number | null
  }
}
const record = (v: unknown): v is Record<string, unknown> => v !== null && typeof v === 'object' && !Array.isArray(v)
const integer = (v: unknown): v is number => Number.isSafeInteger(v) && Number(v) >= 0
const exact = (v: Record<string, unknown>, keys: readonly string[]) => Object.keys(v).length === keys.length && keys.every(k => k in v)
export function isBinanceGridKind(kind: string): kind is BinanceGridKind {
  return (BINANCE_GRID_KINDS as readonly string[]).includes(kind)
}
export function parseGridReadRequest(command: ResearchCommand): GridReadRequest {
  const detail = command.kind === 'binance_grid_detail'
  const allowed = ['kind', 'instance_id', 'request_id', 'query', ...(detail ? ['resource_id'] : ['offset', 'limit'])]
  if (!isBinanceGridKind(command.kind) || Object.keys(command).some(k => !allowed.includes(k))
    || !/^[A-Za-z0-9_-]{8,80}$/.test(command.request_id ?? '')
    || command.query !== undefined && command.query !== 'start'
    || detail && !/^[1-9][0-9]{0,19}$/.test(command.resource_id ?? '')
    || !detail && (command.offset !== undefined && (!integer(command.offset) || command.offset > 500)
      || command.limit !== undefined && (!integer(command.limit) || command.limit < 1 || command.limit > 50))) {
    throw new Error('invalid_request')
  }
  return { kind: command.kind, id: command.request_id!, strategy: command.resource_id,
    offset: command.offset ?? 0, limit: command.limit ?? 25, start: command.query === 'start' }
}
export function gridReadResult(request: GridReadRequest, status: 'pending' | 'failed', error?: GridReadError): GridReadResult {
  return { schema: 'yilong.browser-research.result.v1', kind: request.kind,
    reader: { schema: 'yilong.binance-grid-read.v1', request_id: request.id, status, ...(error ? { error } : {}) } }
}
export function listGridRow(row: GridFacts): GridListRow {
  return Object.fromEntries(GRID_LIST_FIELDS.map(key => [key, row[key]])) as GridListRow
}
function validRow(row: unknown, detail: boolean): boolean {
  if (!record(row) || !exact(row, detail ? Object.keys(GRID_FIELDS) : GRID_LIST_FIELDS)) return false
  try {
    const facts = projectGrid({ ...row, metrics: row })
    return Object.entries(row).every(([key, value]) => value === facts[key as keyof GridFacts])
  } catch { return false }
}
export function validGridReadResult(value: unknown, command: ResearchCommand): value is GridReadResult {
  let request: GridReadRequest
  try { request = parseGridReadRequest(command) } catch { return false }
  if (!record(value) || !exact(value, ['schema', 'kind', 'reader'])
    || value.schema !== 'yilong.browser-research.result.v1' || value.kind !== request.kind || !record(value.reader)) return false
  const r = value.reader, base = ['schema', 'request_id', 'status']
  if (r.schema !== 'yilong.binance-grid-read.v1' || r.request_id !== request.id) return false
  if (r.status === 'pending') return exact(r, base)
  if (r.status === 'failed') return exact(r, [...base, 'error']) && (GRID_READ_ERRORS as readonly unknown[]).includes(r.error)
  if (r.status !== 'ready' || !integer(r.observed_at_ms) || r.observed_at_ms === 0 || !integer(r.expires_at_ms)
    || r.expires_at_ms < r.observed_at_ms || r.expires_at_ms - r.observed_at_ms > GRID_READ_TTL) return false
  const ready = [...base, 'observed_at_ms', 'expires_at_ms']
  if (request.kind === 'binance_grid_detail') return exact(r, [...ready, 'row']) && validRow(r.row, true)
    && record(r.row) && r.row.id === request.strategy
  return exact(r, [...ready, 'items', 'total', 'offset', 'next_offset']) && Array.isArray(r.items)
    && integer(r.total) && r.total <= 500 && r.offset === request.offset && request.offset <= r.total
    && r.items.length === Math.min(request.limit, r.total - request.offset) && r.items.every(row => validRow(row, false))
    && new Set(r.items.map(row => row.id)).size === r.items.length
    && r.next_offset === (request.offset + r.items.length < r.total ? request.offset + r.items.length : null)
}

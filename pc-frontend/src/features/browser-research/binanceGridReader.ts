import { gridReadPort, readGridAttachment, readGridSelection, type GridReadPort } from '../grid-chat/readGridChatSnapshot'
import { gridSource, sameGridSource, type GridFacts, type GridSource } from '../grid-chat/gridChatSnapshot'
import { openExchangeWebSession } from '../exchange-webview/exchangeWebviewApi'
import { GRID_READ_TTL, gridReadResult, listGridRow, parseGridReadRequest } from './binanceGridContract'
import type { GridReadError, GridReadRequest, GridReadResult } from './binanceGridContract'
import type { ResearchCommand } from './types'

interface Dependencies {
  owner: () => string
  port: (owner: string) => GridReadPort
  open: (owner: string) => Promise<unknown>
  now: () => number
}
interface ReadEntry {
  request: GridReadRequest; owner: string; expires: number; active: boolean
  source?: GridSource; observedAt?: number; rows?: GridFacts[]; error?: GridReadError
}
const fault = (code: GridReadError) => Object.assign(new Error(code), { gridCode: code })

/** Explicit MCP reads only. Pending jobs never hold the research heartbeat or replay a private request. */
export function createBinanceGridReader(deps: Dependencies) {
  const entries = new Map<string, ReadEntry>()
  let disposed = false
  const active = (entry: ReadEntry) => !disposed && entry.active && deps.owner() === entry.owner && deps.now() < entry.expires
  function sweep() {
    for (const [key, entry] of entries) if (entry.owner !== deps.owner() || deps.now() >= entry.expires) {
      entry.active = false; entries.delete(key)
    }
  }
  async function prepare(port: GridReadPort, entry: ReadEntry) {
    let state = await port.get()
    if (!active(entry)) throw fault('context_changed')
    if (!state.windowOpen) await deps.open(entry.owner)
    const deadline = deps.now() + 15_000
    // Only the explicit request opens/restores a background page; there is no DOM input.
    for (let attempt = 0; attempt < 45; attempt++) {
      if (!active(entry)) throw fault('context_changed')
      state = await port.get()
      if (state.windowOpen && state.adapterReady && state.documentToken && state.identity) return gridSource(state)
      if (deps.now() >= deadline) break
      await port.sleep()
    }
    throw fault(state.windowOpen ? 'identity_unavailable' : 'session_unavailable')
  }
  async function collect(entry: ReadEntry) {
    const port = deps.port(entry.owner)
    try {
      entry.source = await prepare(port, entry)
      const selection = await readGridSelection(port, () => active(entry))
      if (!sameGridSource(entry.source, selection.source)) throw fault('context_changed')
      if (entry.request.kind === 'binance_grid_list') {
        entry.rows = selection.rows
        entry.observedAt = selection.observedAtMs
      } else {
        if (!selection.rows.some(row => row.id === entry.request.strategy)) throw fault('strategy_not_found')
        const detail = await readGridAttachment(port, selection, entry.request.strategy!, 'mcp-read-only', () => active(entry))
        entry.rows = [detail.facts]; entry.observedAt = detail.observedAtMs
      }
    } catch (error) {
      entry.rows = undefined
      entry.error = error instanceof Error && 'gridCode' in error ? error.gridCode as GridReadError : 'read_failed'
    }
  }
  async function run(project: string, owner: string, command: ResearchCommand): Promise<GridReadResult> {
    const request = parseGridReadRequest(command)
    if (disposed || !owner || owner !== deps.owner() || !/^[a-f0-9]{64}$/.test(project)) return gridReadResult(request, 'failed', 'context_changed')
    // An expired request is a failed query, never an implicit new API request.
    const key = JSON.stringify([project, owner, request.id]), prior = entries.get(key)
    if (prior && deps.now() >= prior.expires) {
      prior.active = false
      return gridReadResult(request, 'failed', 'request_expired')
    }
    sweep()
    let entry = entries.get(key)
    if (!entry) {
      if (!request.start || request.offset !== 0) return gridReadResult(request, 'failed', 'request_expired')
      if ([...entries.values()].some(item => !item.rows && !item.error)) return gridReadResult(request, 'failed', 'busy')
      if (entries.size >= 16) return gridReadResult(request, 'failed', 'busy')
      entry = { request, owner, expires: deps.now() + GRID_READ_TTL, active: true }
      entries.set(key, entry)
      void collect(entry)
      return gridReadResult(request, 'pending')
    }
    if (request.kind !== entry.request.kind || request.strategy !== entry.request.strategy) return gridReadResult(request, 'failed', 'request_conflict')
    if (entry.error) return gridReadResult(request, 'failed', entry.error)
    if (!entry.rows || !entry.source || !entry.observedAt) return gridReadResult(request, 'pending')
    try {
      if (!sameGridSource(entry.source, gridSource(await deps.port(owner).get()))) throw fault('context_changed')
      if (!active(entry)) throw fault('context_changed')
    } catch { entry.rows = undefined; entry.error = 'context_changed'; return gridReadResult(request, 'failed', 'context_changed') }
    const result = gridReadResult(request, 'pending')
    result.reader = { ...result.reader, status: 'ready', observed_at_ms: entry.observedAt, expires_at_ms: entry.expires }
    if (request.kind === 'binance_grid_detail') result.reader.row = { ...entry.rows[0] }
    else {
      if (request.offset > entry.rows.length) return gridReadResult(request, 'failed', 'invalid_request')
      const items = entry.rows.slice(request.offset, request.offset + request.limit).map(listGridRow)
      Object.assign(result.reader, { items, total: entry.rows.length, offset: request.offset,
        next_offset: request.offset + items.length < entry.rows.length ? request.offset + items.length : null })
    }
    return result
  }
  return { run, dispose: () => { disposed = true; for (const entry of entries.values()) entry.active = false; entries.clear() } }
}
export function browserBinanceGridReader(owner: () => string) {
  return createBinanceGridReader({ owner, port: gridReadPort, now: Date.now,
    open: key => openExchangeWebSession('binance', key, false) })
}

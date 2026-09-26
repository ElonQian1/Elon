import { json, localUrl, result, rpc } from './local-rpc.mjs'

const properties = {
  request_id: { type: 'string', pattern: '^[A-Za-z0-9_-]{8,96}$', description: 'Unique snapshot request ID; reuse for polling and pages only.' },
  start: { type: 'boolean', default: false, description: 'Explicitly begin a read. Omit when polling or paging.' },
}
export const gridTools = [
  { name: 'binance_grid_list', description: 'Read this Android phone own Binance grids through the authenticated Yilong APK private-session adapter. Requires configured loopback ELON_APK_MCP_URL (ADB forward). start=true requests fresh data; poll the same request_id. No Win fallback, trades, or chat sends.',
    inputSchema: { type: 'object', additionalProperties: false, required: ['request_id'], properties: {
      ...properties, offset: { type: 'integer', minimum: 0, maximum: 500 }, limit: { type: 'integer', minimum: 1, maximum: 50 },
    } }, annotations: { readOnlyHint: true, destructiveHint: false, openWorldHint: true } },
  { name: 'binance_grid_detail', description: 'Read one exact owned strategy from this Android phone, refreshing its owned list first. Requires configured APK MCP. Never infer the target from symbol alone. Unknown fields remain null; amounts are decimal strings.',
    inputSchema: { type: 'object', additionalProperties: false, required: ['request_id', 'strategy_id'], properties: {
      ...properties, strategy_id: { type: 'string', pattern: '^[1-9][0-9]{0,19}$' },
    } }, annotations: { readOnlyHint: true, destructiveHint: false, openWorldHint: true } },
]
const fields = new Set(['id', 'symbol', 'status', 'direction', 'spacing', 'lower', 'upper', 'count', 'leverage', 'profit', 'created',
  'investment', 'initialNotional', 'perGridQty', 'perGridQuoteQty', 'matchedPnl', 'fundingFee', 'fee', 'matchedCount', 'marginType', 'orderCurrency', 'stopUpper', 'stopLower'])
function request(name, args) {
  const kind = name === 'binance_grid_list' ? 'list' : name === 'binance_grid_detail' ? 'detail' : null
  if (!kind || !args || typeof args !== 'object' || Array.isArray(args)) throw new Error('invalid_grid_request')
  const keys = kind === 'list' ? ['request_id', 'start', 'offset', 'limit'] : ['request_id', 'start', 'strategy_id']
  if (Object.keys(args).some(key => !keys.includes(key)) || typeof args.request_id !== 'string' || !/^[A-Za-z0-9_-]{8,96}$/.test(args.request_id)) throw new Error('invalid_grid_request')
  if ('start' in args && typeof args.start !== 'boolean') throw new Error('invalid_grid_request')
  if (kind === 'detail' && (typeof args.strategy_id !== 'string' || !/^[1-9][0-9]{0,19}$/.test(args.strategy_id))) throw new Error('invalid_grid_request')
  for (const [key, max] of [['offset', 500], ['limit', 50]]) {
    if (key in args && (!Number.isInteger(args[key]) || args[key] < (key === 'limit' ? 1 : 0) || args[key] > max)) throw new Error('invalid_grid_request')
  }
  if (args.start && (args.offset || 0) !== 0) throw new Error('invalid_grid_request')
  return { ...args, kind }
}
function projectRow(row) {
  if (!row || typeof row !== 'object' || Array.isArray(row) || Object.keys(row).length !== fields.size || Object.keys(row).some(key => !fields.has(key))) throw new Error('invalid_grid_receipt')
  if (Object.values(row).some(value => value !== null && (typeof value !== 'string' || value.length > 64))) throw new Error('invalid_grid_receipt')
  if (!/^[1-9][0-9]{0,19}$/.test(row.id || '') || !/^[A-Z0-9\u3400-\u4DBF\u4E00-\u9FFF]{1,24}USDT$/.test(row.symbol || '')) throw new Error('invalid_grid_receipt')
  return Object.fromEntries([...fields].map(key => [key, row[key]]))
}
export function gridReceipt(value, input) {
  if (value?.schema !== 'yilong.binance_grid_read.v1' || value.source !== 'android_webview' || value.kind !== input.kind || value.request_id !== input.request_id || value.trading_enabled !== false) throw new Error('invalid_grid_receipt')
  const base = { schema: value.schema, source: value.source, kind: value.kind, request_id: value.request_id, status: value.status, trading_enabled: false }
  if (value.status === 'pending') return base
  if (value.status === 'failed') return { ...base, error: /^[a-z_]{1,60}$/.test(value.error || '') ? value.error : 'grid_read_failed' }
  if (value.status !== 'ready' || !Number.isSafeInteger(value.observed_at_ms) || value.observed_at_ms <= 0 || value.observed_at_ms > Date.now() + 5000 || Date.now() - value.observed_at_ms > 300000 || !Number.isSafeInteger(value.valid_for_ms) || value.valid_for_ms < 1 || value.valid_for_ms > 300000) throw new Error('invalid_grid_receipt')
  Object.assign(base, { observed_at_ms: value.observed_at_ms, valid_for_ms: value.valid_for_ms, coverage: 'observed_response_only' })
  if (input.kind === 'detail') {
    const row = projectRow(value.row)
    if (row.id !== input.strategy_id) throw new Error('invalid_grid_receipt')
    return { ...base, row }
  }
  const { total, offset, next_offset, rows } = value
  if (!Number.isInteger(total) || total < 0 || total > 500 || offset !== (input.offset || 0) || offset > total || !Array.isArray(rows) || rows.length !== Math.min(input.limit || 25, total - offset)) throw new Error('invalid_grid_receipt')
  if (next_offset !== (offset + rows.length < total ? offset + rows.length : null)) throw new Error('invalid_grid_receipt')
  const projected = rows.map(projectRow)
  if (new Set(projected.map(row => row.id)).size !== rows.length) throw new Error('invalid_grid_receipt')
  return { ...base, total, offset, next_offset, rows: projected }
}
export function createGridService(env = process.env, transport = json) {
  let token
  return async (name, args) => {
    const input = request(name, args)
    if (!env.ELON_APK_MCP_URL) throw new Error('apk_endpoint_not_configured')
    const base = localUrl(env.ELON_APK_MCP_URL)
    const health = await transport(base + '/health')
    if (typeof health.auth_token !== 'string' || !health.auth_token) throw new Error('apk_unavailable')
    if (token && token !== health.auth_token) { token = health.auth_token; throw new Error('apk_session_changed') }
    token = health.auth_token
    const value = result(await transport(base + '/mcp', rpc('binance_grid_read', { ...input, auth_token: token })))
    return gridReceipt(value, input)
  }
}

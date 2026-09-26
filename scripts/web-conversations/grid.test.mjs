import test from 'node:test'
import assert from 'node:assert/strict'
import { createGridService, gridReceipt, gridTools } from './grid.mjs'

const input = { kind: 'list', request_id: 'grid_test_001' }
const ready = () => ({ schema: 'yilong.binance_grid_read.v1', source: 'android_webview', ...input, trading_enabled: false,
  status: 'ready', observed_at_ms: Date.now(), valid_for_ms: 299000, total: 0, offset: 0, next_offset: null, rows: [] })
test('empty verified list remains ready and drops arbitrary extra output', () => {
  assert.deepEqual(gridReceipt({ ...ready(), cookie: 'never-export' }, input).rows, [])
  assert.equal(JSON.stringify(gridReceipt({ ...ready(), cookie: 'never-export' }, input)).includes('cookie'), false)
  assert.equal(gridTools.length, 2)
})
test('wrong request, source, paging, stale or impossible timestamps fail closed', () => {
  for (const change of [{ request_id: 'wrong' }, { source: 'win' }, { trading_enabled: true }, { total: 1 },
    { offset: 1 }, { next_offset: 1 }, { observed_at_ms: 1 }, { observed_at_ms: Date.now() + 60000 }, { valid_for_ms: 300001 }]) {
    assert.throws(() => gridReceipt({ ...ready(), ...change }, input), /invalid_grid_receipt/)
  }
})
test('explicit APK endpoint and validation happen before authenticated calls', async () => {
  let calls = 0
  const service = createGridService({}, async () => { calls++; throw Error('unexpected') })
  await assert.rejects(service('binance_grid_list', { request_id: 'grid_test_001', url: 'https://example.com' }), /invalid_grid_request/)
  await assert.rejects(service('binance_grid_list', { request_id: 'grid_test_001' }), /apk_endpoint_not_configured/)
  assert.equal(calls, 0)
})
test('start and poll forward unchanged and credentials stay inside transport', async () => {
  const sent = []
  const service = createGridService({ ELON_APK_MCP_URL: 'http://127.0.0.1:18787' }, async (url, body) => {
    if (url.endsWith('/health')) return { auth_token: 'secret-local-token' }
    sent.push(body.params.arguments)
    return { result: { structuredContent: ready() } }
  })
  const result = await service('binance_grid_list', { request_id: input.request_id, start: true })
  await service('binance_grid_list', { request_id: input.request_id })
  assert.equal(sent[0].start, true); assert.equal(sent[1].start, undefined)
  assert.equal(sent[0].auth_token, 'secret-local-token')
  assert.equal(JSON.stringify(result).includes('secret-local-token'), false)
})
test('phone restart interrupts stale request without automatic replay', async () => {
  let token = 'before', calls = 0
  const service = createGridService({ ELON_APK_MCP_URL: 'http://127.0.0.1:18787' }, async (url) => {
    if (url.endsWith('/health')) return { auth_token: token }
    calls++; return { result: { structuredContent: ready() } }
  })
  await service('binance_grid_list', { request_id: input.request_id, start: true }); token = 'after'
  await assert.rejects(service('binance_grid_list', { request_id: input.request_id }), /apk_session_changed/)
  assert.equal(calls, 1)
})
test('failed requests contain only bounded errors and no stale rows', () => {
  const reply = gridReceipt({ ...ready(), status: 'failed', error: 'strategy_not_found', rows: [{ cookie: 'private' }] }, input)
  assert.equal(reply.error, 'strategy_not_found'); assert.equal(reply.rows, undefined)
})
test('exact strategy detail preserves decimals and rejects foreign ID or extra fields', () => {
  const names = ['id', 'symbol', 'status', 'direction', 'spacing', 'lower', 'upper', 'count', 'leverage', 'profit', 'created',
    'investment', 'initialNotional', 'perGridQty', 'perGridQuoteQty', 'matchedPnl', 'fundingFee', 'fee', 'matchedCount', 'marginType', 'orderCurrency', 'stopUpper', 'stopLower']
  const row = Object.fromEntries(names.map(key => [key, null]))
  Object.assign(row, { id: '123', symbol: '龙虾USDT', perGridQty: '377', investment: '4029.33780960' })
  const q = { ...input, kind: 'detail', strategy_id: '123' }
  const reply = { ...ready(), kind: 'detail', row }
  assert.equal(gridReceipt(reply, q).row.investment, '4029.33780960')
  assert.throws(() => gridReceipt(reply, { ...q, strategy_id: '124' }), /invalid_grid_receipt/)
  assert.throws(() => gridReceipt({ ...reply, row: { ...row, account: 'private' } }, q), /invalid_grid_receipt/)
  assert.throws(() => gridReceipt({ ...reply, row: { ...row, investment: 0 } }, q), /invalid_grid_receipt/)
})

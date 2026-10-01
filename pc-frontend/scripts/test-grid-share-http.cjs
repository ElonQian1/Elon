const { test } = require('node:test')
const assert = require('node:assert/strict')
const { createHash, webcrypto } = require('node:crypto')
const fs = require('node:fs'), path = require('node:path'), vm = require('node:vm'), ts = require('typescript')
const { pathToFileURL } = require('node:url')
const root = path.join(__dirname, '../src/features/grid-share')
function load(name, dependencies = {}, globals = {}) {
  const module = { exports: {} }
  const source = fs.readFileSync(path.join(root, name + '.ts'), 'utf8')
  vm.runInNewContext(ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 } }).outputText,
    { module, exports: module.exports, TextEncoder, Uint8Array, AbortSignal, Date, ...globals, require: name => {
      if (!(name in dependencies)) throw Error('Unexpected import ' + name)
      return dependencies[name]
    } })
  return module.exports
}
const model = load('gridShareModel'), picker = load('gridSharePickerModel')
async function identity() {
  const webUuid = await import(pathToFileURL(path.join(__dirname, '../node_modules/uuid/dist/index.js')).href)
  return load('gridShareIdentity', { uuid: webUuid, '@noble/hashes/sha2.js': await import('@noble/hashes/sha2.js'), '@noble/hashes/utils.js': await import('@noble/hashes/utils.js') })
}
test('Win HTTP origin: UUID and digest work without randomUUID or subtle', async () => {
  const original = Object.getOwnPropertyDescriptor(globalThis, 'crypto')
  let entropyCalls = 0
  Object.defineProperty(globalThis, 'crypto', { configurable: true, value: { getRandomValues: bytes => { entropyCalls++; return webcrypto.getRandomValues(bytes) } } })
  try {
    const ids = await identity(), seen = new Set()
    for (let i = 0; i < 1000; i++) {
      const id = ids.gridReadRequestId()
      assert.match(id, /^[0-9a-f]{12}4[0-9a-f]{3}[89ab][0-9a-f]{15}$/)
      seen.add(id)
    }
    assert.equal(seen.size, 1000); assert.equal(entropyCalls, 1000)
    for (const doc of ['', 'abc', 'QNT 龙虾 🦞', { fields: { profit: '-0.000000001', note: '中文' } }, 'a'.repeat(10000)]) {
      assert.equal(ids.gridShareDigest(doc), createHash('sha256').update(JSON.stringify(doc)).digest('hex'))
    }
  } finally { Object.defineProperty(globalThis, 'crypto', original) }
})
test('position read reaches the existing adapter and keeps exact PnL in the HTTP environment', async () => {
  const ids = await identity(), source = { account: 'fixture', accountKind: 'sub', document: 'doc' }
  let request, dispatched = 0
  const state = () => ({ source, reports: request ? { [request]: { schema: 'yilong.binance_report_observation.v1', request, kind: 'positions', account: source.account, account_kind: source.accountKind, status: 'ready', coverage: 'strategy_position', rows: [{ symbol: 'QNTUSDT', pnl: '-123.45000001' }] } } : {} })
  const reader = load('readSharePositions', {
    './gridShareIdentity': ids,
    '../grid-chat/gridChatSnapshot': { object: value => value ?? {}, gridSource: state => state.source, sameGridSource: (a, b) => JSON.stringify(a) === JSON.stringify(b) },
    '../grid-chat/readGridChatSnapshot': { gridReadPort: () => ({ get: async () => state(), sleep: async () => {} }) },
    '../exchange-webview/exchangeWebviewApi': { runExchangeWebAdapterCommand: async (_site, _owner, action, payload) => { assert.equal(action, 'report'); request = JSON.parse(payload).request; dispatched++ } },
  }, { crypto: { getRandomValues: bytes => webcrypto.getRandomValues(bytes) } })
  const actual = await reader.readSharePositions('owner', { source, facts: { id: '123', symbol: 'QNTUSDT' } }, () => true)
  assert.equal(dispatched, 1); assert.equal(actual.facts.unrealizedPnl, '-123.45000001'); assert.equal(actual.positionNotice, undefined)
})
test('sending and retry use the original SHA-256 document key and reject mismatched receipts', async () => {
  const bodies = [], grid = model.publicGrid({ observedAtMs: Date.now(), facts: { symbol: 'QNTUSDT', profit: '999', id: 'private' } })
  let wrongReceipt = false
  const api = load('gridShareApi', {
    '../../api/client': { getAuthToken: () => 'fixture-token' }, '../../api/runtime': { resolveApiUrl: path => path },
    './gridShareModel': model, './gridShareIdentity': await identity(),
  }, { fetch: async (_url, init) => {
    const body = JSON.parse(init.body); bodies.push(body)
    return { ok: true, json: async () => ({ snapshot_id: 'ai_snapshot_test', message: { content: model.SHARE_PREFIX + JSON.stringify({ schema: model.SHARE_SCHEMA, provider: 'binance', snapshot_id: 'ai_snapshot_test', group_id: wrongReceipt ? 'another_group' : 'group_test', grid: body.document.grid }) } }) }
  }, crypto: {} })
  await api.publishShare('group_test', grid, new AbortController().signal)
  await api.publishShare('group_test', grid, new AbortController().signal)
  assert.equal(bodies[0].idempotency_key, bodies[1].idempotency_key)
  assert.equal(bodies[0].idempotency_key, createHash('sha256').update(JSON.stringify(bodies[0].document)).digest('hex'))
  assert.equal(bodies[0].document.grid.show_amounts, true); assert.equal(bodies[0].document.grid.fields.profit, '999'); assert.equal(bodies[0].document.grid.fields.id, undefined)
  await api.publishShare('group_test', { ...grid, observed_at_ms: grid.observed_at_ms + 1 }, new AbortController().signal)
  assert.notEqual(bodies[0].idempotency_key, bodies[2].idempotency_key)
  wrongReceipt = true
  await assert.rejects(api.publishShare('group_test', grid, new AbortController().signal), /回执不匹配/)
})
test('picker sorts exact signed decimals and leaves unknown profits last in both directions', () => {
  const rows = [
    { id: '1', symbol: 'QNTUSDT', profit: null }, { id: '2', symbol: 'QNTUSDT', profit: '0' },
    { id: '3', symbol: 'LINKUSDT', profit: '-1.5' }, { id: '4', symbol: 'BTCUSDT', profit: '9007199254740993.00001' },
    { id: '5', symbol: 'BTCUSDT', profit: '9007199254740993.00002' },
  ]
  assert.equal(picker.selectGridRows(rows, '', 'profit-desc').map(r => r.id).join(','), '5,4,2,3,1')
  assert.equal(picker.selectGridRows(rows, '', 'profit-asc').map(r => r.id).join(','), '3,2,4,5,1')
  assert.equal(picker.selectGridRows(rows, ' qnt ', 'profit-desc').map(r => r.id).join(','), '2,1')
  assert.equal(rows[0].id, '1'); assert.equal(picker.selectGridRows(rows, 'NOPE', 'symbol').length, 0)
})
test('profit presentation distinguishes missing, zero, tiny negatives and gains', () => {
  assert.equal(picker.displayProfit(null), '未读取'); assert.equal(picker.displayProfit('NaN'), '未读取')
  assert.equal(picker.displayProfit('-0.00000'), '0'); assert.equal(picker.displayProfit('-0.0000001'), '−<0.0001')
  assert.equal(picker.displayProfit('1234.56789'), '+1,234.5678'); assert.equal(picker.profitTone('-0.1'), 'loss')
  assert.equal(picker.profitTone('0'), 'neutral'); assert.equal(picker.profitTone('0.1'), 'gain')
})

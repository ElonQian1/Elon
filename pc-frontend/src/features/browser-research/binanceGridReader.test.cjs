const assert = require('node:assert/strict')
const { test } = require('node:test')
const fs = require('node:fs')
const path = require('node:path')
const Module = require('node:module')
const ts = require('typescript')
const cache = new Map()
function load(name) {
  const file = path.resolve(__dirname, name.endsWith('.ts') ? name : `${name}.ts`)
  if (cache.has(file)) return cache.get(file)
  const compiled = new Module(file, module)
  compiled.filename = file; compiled.paths = module.paths
  compiled.require = id => id.includes('exchangeWebviewApi') ? {} : id.startsWith('.')
    ? load(path.resolve(path.dirname(file), id)) : require(id)
  compiled._compile(ts.transpileModule(fs.readFileSync(file, 'utf8'), {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS }, fileName: file,
  }).outputText, file)
  cache.set(file, compiled.exports)
  return compiled.exports
}
const { createBinanceGridReader } = load('binanceGridReader')
const { parseGridReadRequest, validGridReadResult, GRID_READ_TTL } = load('binanceGridContract')
const { parseResearchResult } = load('browserResearchModel')
const project = 'a'.repeat(64)
const list = { kind: 'binance_grid_list', request_id: 'fixture_read_1', limit: 1 }
const detail = { kind: 'binance_grid_detail', request_id: 'fixture_read_2', resource_id: '124' }
function fixture(options = {}) {
  let now = 1_900_000_000_000, owner = 'owner_a', pending
  const commands = [], opens = [], gets = []
  const row = id => ({ id, symbol: '龙虾USDT', account: '888', direction: 'SHORT', count: '169',
    lower: '0.103280000000000001', upper: '0.16280', leverage: '4', cookie: 'never_outbound',
    metrics: { investment: '3000', perGridQty: id === '124' ? '250' : '100', authorization: 'never_outbound' } })
  const event = value => ({ schema: 'yilong.binance_observation.v1', account: '888', account_kind: 'primary',
    observedAtMs: now, observationSequence: now, ...value })
  const state = { windowOpen: options.closed !== true, adapterReady: true, documentToken: 'document_a',
    identity: { account: '888', account_kind: 'primary' }, list: event({ rows: [row('123'), row('124')] }),
    details: {}, unavailableAtMs: 0 }
  const port = {
    get: async () => { gets.push(owner); return structuredClone(state) },
    command: async (action, id) => { commands.push([action, id]); pending = { action, id } },
    sleep: async () => {
      now += 350
      if (options.onSleep) options.onSleep(state)
      if (!pending || options.stale) return
      if (pending.action === 'refresh') { state.list = event({ rows: options.rows ?? [row('123'), row('124')] }); state.details = {} }
      else state.details[pending.id] = event({ row: row(pending.id) })
      pending = undefined
    }, now: () => now,
  }
  const reader = createBinanceGridReader({ owner: () => owner, port: () => port, now: () => now,
    open: async key => { opens.push(key); state.windowOpen = true } })
  const run = (command = list, key = project) => reader.run(key, owner, command)
  const start = async (command = list) => {
    const initial = await run({ ...command, query: 'start' })
    await new Promise(resolve => setImmediate(resolve))
    return { initial, result: await run(command) }
  }
  return { reader, run, start, state, commands, opens, gets, advance: n => { now += n }, owner: key => { owner = key } }
}

test('only explicit start reads: cold open, immutable pagination, repeats and full outbound validation', async () => {
  const h = fixture({ closed: true })
  assert.equal(h.commands.length, 0)
  assert.equal((await h.run()).reader.error, 'request_expired')
  assert.equal(h.opens.length, 0)
  const { initial, result } = await h.start()
  assert.equal(initial.reader.status, 'pending')
  assert.equal(result.reader.status, 'ready')
  assert.equal(h.opens.length, 1)
  assert.deepEqual(h.commands, [['refresh', undefined]])
  assert.equal(result.reader.items[0].lower, '0.103280000000000001')
  assert.equal(result.reader.next_offset, 1)
  assert.equal(validGridReadResult(result, list), true)
  assert.deepEqual(parseResearchResult(result, list), result)
  const observed = result.reader.observed_at_ms
  h.state.list.rows = [] // Later page observations must not replace this request's frozen snapshot.
  const page = await h.run({ ...list, offset: 1 })
  assert.equal(page.reader.items[0].id, '124'); assert.equal(page.reader.next_offset, null)
  assert.equal(page.reader.observed_at_ms, observed)
  assert.equal(validGridReadResult(page, { ...list, offset: 1 }), true)
  await h.run({ ...list, query: 'start' }); await h.run()
  assert.equal(h.commands.length, 1)
  assert.equal(JSON.stringify(result).includes('888'), false)
  assert.equal(JSON.stringify(result).includes('never_outbound'), false)
  assert.equal((await h.run({ ...list, offset: 3 })).reader.error, 'invalid_request')
})

test('detail refreshes owned list then exact strategy; same symbol never selects another strategy', async () => {
  const h = fixture(), { result } = await h.start(detail)
  assert.equal(result.reader.status, 'ready')
  assert.deepEqual(h.commands, [['refresh', undefined], ['detail', '124']])
  assert.equal(result.reader.row.id, '124'); assert.equal(result.reader.row.perGridQty, '250')
  assert.equal(validGridReadResult(result, detail), true)
  await h.run(detail); assert.equal(h.commands.length, 2)
  assert.equal((await h.run({ ...detail, resource_id: '123' })).reader.error, 'request_conflict')
  const missing = fixture(); const read = await missing.start({ ...detail, resource_id: '999' })
  assert.equal(read.result.reader.error, 'strategy_not_found')
  assert.deepEqual(missing.commands, [['refresh', undefined]])
})

test('stale data, duplicate IDs, unavailable evidence and changing account/document fail closed', async () => {
  for (const options of [{ stale: true }, { rows: [{ id: '123', symbol: 'BTCUSDT', account: '888' }, { id: '123', symbol: 'BTCUSDT', account: '888' }] },
    { onSleep: state => { state.documentToken = 'other' } }, { onSleep: state => { state.identity.account = '999' } }]) {
    const h = fixture(options), { result } = await h.start()
    assert.equal(result.reader.status, 'failed')
    assert.equal('items' in result.reader, false)
  }
  for (const mutate of [state => { state.windowOpen = false }, state => { state.documentToken = 'b' }, state => { state.identity.account = '999' }]) {
    const h = fixture(); await h.start(); mutate(h.state)
    assert.equal((await h.run()).reader.error, 'context_changed')
    assert.equal((await h.run()).reader.error, 'context_changed')
    assert.equal(h.commands.length, 1)
  }
})

test('project/owner scope, expiration and reload queries never reuse or silently restart reads', async () => {
  const h = fixture(); await h.start()
  assert.equal((await h.run(list, 'b'.repeat(64))).reader.error, 'request_expired')
  h.owner('owner_b'); assert.equal((await h.run()).reader.error, 'request_expired')
  h.owner('owner_a'); assert.equal((await h.run()).reader.error, 'request_expired')
  const aged = fixture(); await aged.start(); aged.advance(GRID_READ_TTL)
  assert.equal((await aged.run()).reader.error, 'request_expired'); assert.equal(aged.commands.length, 1)
  const reload = fixture(); assert.equal((await reload.run()).reader.error, 'request_expired')
  reload.reader.dispose(); assert.equal((await reload.run({ ...list, query: 'start' })).reader.error, 'context_changed')
  assert.equal(reload.commands.length, 0)
})

test('concurrency and bounded snapshot capacity do not cause implicit eviction or repeat reads', async () => {
  const h = fixture()
  const first = await h.run({ ...list, query: 'start' })
  assert.equal(first.reader.status, 'pending')
  assert.equal((await h.run({ ...detail, query: 'start' })).reader.error, 'busy')
  await new Promise(resolve => setImmediate(resolve))
  for (let i = 1; i < 16; i++) await h.start({ ...list, request_id: `fixture_read_${i + 1}` })
  assert.equal((await h.run({ ...list, request_id: 'fixture_overflow', query: 'start' })).reader.error, 'busy')
  assert.equal(h.commands.length, 16)
  assert.equal((await h.run()).reader.status, 'ready')
})

test('fixed command/result contracts reject injected fields, wrong strategy and fabricated paging', async () => {
  for (const patch of [{ query: 'fetch()' }, { url: 'https://example.org' }, { owner: 'b' }, { limit: 51 }, { offset: -1 }, { request_id: 'tiny' }]) {
    assert.throws(() => parseGridReadRequest({ ...list, ...patch }))
  }
  const h = fixture(), { result } = await h.start()
  for (const mutate of [r => { r.reader.items[0].cookie = 'secret' }, r => { r.reader.items[0].lower = 0.1 },
    r => { r.reader.request_id = 'other_read' }, r => { r.reader.offset = 1 }, r => { r.reader.next_offset = null },
    r => { r.reader.expires_at_ms += GRID_READ_TTL }, r => { r.body = 'private' }]) {
    const bad = structuredClone(result); mutate(bad)
    assert.equal(validGridReadResult(bad, list), false)
    assert.throws(() => parseResearchResult(bad, list))
  }
  const exact = (await fixture().start(detail)).result
  exact.reader.row.id = '123'; assert.equal(validGridReadResult(exact, detail), false)
  const empty = (await fixture({ rows: [] }).start()).result
  assert.equal(empty.reader.total, 0); assert.equal(validGridReadResult(empty, list), true)
})

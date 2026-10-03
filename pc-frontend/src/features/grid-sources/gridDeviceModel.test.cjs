const assert = require('node:assert/strict')
const { test } = require('node:test')
const fs = require('node:fs')
const path = require('node:path')
const Module = require('node:module')
const ts = require('typescript')

function load(file) {
  const compiled = new Module(file, module)
  compiled.filename = file; compiled.paths = module.paths
  const original = compiled.require.bind(compiled)
  compiled.require = id => id.startsWith('.') ? load(path.resolve(path.dirname(file), id + '.ts')) : original(id)
  compiled._compile(ts.transpileModule(fs.readFileSync(file, 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 }, fileName: file,
  }).outputText, file)
  return compiled.exports
}
const { deviceSnapshot, parseDeviceSources, fresh, accountDigest } = load(path.join(__dirname, 'gridDeviceModel.ts'))
const device = '00000000-0000-4000-8000-000000000001', now = Date.now()
function state() {
  return { windowOpen: true, adapterReady: true, documentToken: 'doc-fixture',
    identity: { account: '42', account_kind: 'primary' },
    list: { schema: 'yilong.binance_observation.v1', account: '42', account_kind: 'primary',
      observedAtMs: now, rows: [{ id: '7', account: '42', symbol: 'BTCUSDT', status: 'WORKING',
        direction: 'LONG', spacing: 'ARITH', lower: '10', upper: '20', count: '12', leverage: '2',
        profit: '-0.00000000000000000001', created: String(now - 1000), metrics: { fee: null }, cookie: 'synthetic-secret' }] },
    details: {} }
}
test('snapshot hashes account and allowlists business fields without losing exact decimals', () => {
  const result = deviceSnapshot(state(), device, 1, now)
  assert.equal(result.account, accountDigest('42'))
  assert.equal(result.rows[0].profit, '-0.00000000000000000001')
  assert.equal(result.rows[0].account, undefined)
  assert.equal(result.rows[0].cookie, undefined)
  assert.equal(result.rows[0].metrics.fee, null)
  assert.equal(result.fresh_until_ms, now + 300000)
})
test('closed, unavailable and expired Win observers send empty tombstones', () => {
  const closed = state(); closed.windowOpen = false
  const gone = state(); gone.list = null
  for (const input of [null, closed, gone]) assert.equal(deviceSnapshot(input, device, 1, now).status, 'unavailable')
  const old = deviceSnapshot(state(), device, 2, now + 300000)
  assert.equal(old.status, 'unavailable'); assert.deepEqual(old.rows, []); assert.equal(old.account, null)
})
test('an account mismatch cannot publish or join detail from a different account', () => {
  const other = state(); other.list.account = '99'
  assert.throws(() => deviceSnapshot(other, device, 1, now))
  const detail = state()
  detail.details['7'] = { schema: 'yilong.binance_observation.v1', account: '99', account_kind: 'primary',
    row: { id: '7', symbol: 'BTCUSDT', profit: '9999' } }
  assert.equal(deviceSnapshot(detail, device, 1, now).rows[0].profit, '-0.00000000000000000001')
})
test('both sources preserve identity and old observations never become fresh on re-read', () => {
  const win = deviceSnapshot(state(), device, 2, now)
  const apk = { ...win, platform: 'android' }
  const raw = JSON.stringify({ schema: 'yilong.grid_device_sources.v1',
    sources: [{ source_id: 'a'.repeat(64), snapshot: win }, { source_id: 'b'.repeat(64), snapshot: apk }] })
  assert.equal(parseDeviceSources(raw).length, 2)
  assert.equal(fresh(win, now + 300000), false)
  assert.throws(() => parseDeviceSources(raw.replace('b'.repeat(64), 'a'.repeat(64))))
  assert.throws(() => parseDeviceSources(raw.replace('"sequence":2', '"sequence":0')))
})

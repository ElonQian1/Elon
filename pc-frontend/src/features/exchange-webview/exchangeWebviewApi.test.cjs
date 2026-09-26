const assert = require('node:assert/strict')
const { test } = require('node:test')
const fs = require('node:fs')
const path = require('node:path')
const Module = require('node:module')
const ts = require('typescript')
function load(invoke) {
  const file = path.join(__dirname, 'exchangeWebviewApi.ts')
  const compiled = new Module(file, module)
  compiled.filename = file; compiled.paths = module.paths
  compiled.require = id => id.includes('desktopShell') ? { getDesktopInvoke: () => invoke }
    : { normalizeExchangeWebviewError: error => error }
  compiled._compile(ts.transpileModule(fs.readFileSync(file, 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 }, fileName: file,
  }).outputText, file)
  return compiled.exports
}
const response = args => ({ schema: 'yilong.exchange_webview.session.v1', providerId: args.providerId,
  windowLabel: 'fixture', status: args.showWindow ? 'focused' : 'background',
  profileScope: 'local_owner_provider', cookieAccess: 'webview_only' })
const providers = [{ schema: 'yilong.exchange_webview.provider.v1', providerId: 'binance', displayName: 'Binance',
  startHost: 'www.binance.com', loginMode: 'manual_web', profileScope: 'local_owner_provider',
  desktopRuntimeVersion: 14, backgroundOpenSupported: true }]
test('background open accepts native background receipt and does not merge owners or foreground intent', async () => {
  const calls = [], releases = []
  const api = load((name, args) => {
    if (name === 'list_exchange_web_providers') return Promise.resolve(providers)
    calls.push({ name, args }); return new Promise(resolve => releases.push(() => resolve(response(args))))
  })
  const a = api.openExchangeWebSession('binance', 'owner_a', false)
  const duplicate = api.openExchangeWebSession('binance', 'owner_a', false)
  const b = api.openExchangeWebSession('binance', 'owner_b', false)
  const foreground = api.openExchangeWebSession('binance', 'owner_a')
  await new Promise(resolve => setImmediate(resolve))
  assert.equal(calls.length, 3)
  assert.deepEqual(calls.map(c => c.args.showWindow), [true, false, false])
  assert.ok(calls.every(c => c.name === 'open_exchange_web_session'))
  releases.forEach(resolve => resolve())
  assert.deepEqual((await Promise.all([a, duplicate, b, foreground])).map(r => r.status), ['background', 'background', 'background', 'focused'])
  const fresh = api.openExchangeWebSession('binance', 'owner_a', false)
  await new Promise(resolve => setImmediate(resolve))
  assert.equal(calls.length, 4); releases[3](); await fresh
})
test('background request fails on a foreground-only old host receipt', async () => {
  const api = load(async (name, args) => name === 'list_exchange_web_providers' ? providers : ({ ...response(args), status: 'created' }))
  await assert.rejects(api.openExchangeWebSession('binance', 'owner_a', false))
})
test('old hosts without background capability never receive an open that could steal focus', async () => {
  let opens = 0
  const api = load(async name => {
    if (name === 'list_exchange_web_providers') return providers.map(p => ({ ...p, backgroundOpenSupported: undefined }))
    opens++
  })
  await assert.rejects(api.openExchangeWebSession('binance', 'owner_a', false))
  assert.equal(opens, 0)
})

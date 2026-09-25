import { test } from 'node:test'
import assert from 'node:assert/strict'
import { createControl, exactRelease } from './control.mjs'

const target = '0.3.70+' + 'a'.repeat(40), base = 'http://127.0.0.1:7799'
function fixture({ online = true, host = true } = {}) {
  let time = 0, launches = 0, bootstraps = 0, calls = 0
  const options = { env: {}, projectRoot: '.', now: () => time, delay: async ms => { time += ms },
    discover: async () => online ? base : null, launch: async () => { launches++; online = true; host = true },
    request: async (url, body) => {
      calls++
      if (url.endsWith('/bootstrap')) { bootstraps++; return { mcp: { url: base + '/mcp?token=synthetic' } } }
      if (body.params.name === 'win_control_status') return { result: { structuredContent: { capabilities: {
        schema: 'elon.win_codex_control.v1', release_identity: target, tauri_available: host, frontend_available: host } } } }
      const args = body.params.arguments
      return { result: { structuredContent: { action: { action_id: 'win_act_' + 'b'.repeat(32), kind: args.kind, status: 'succeeded' } } } }
    } }
  return { options, launches: () => launches, bootstraps: () => bootstraps, calls: () => calls }
}
test('cold node and missing desktop reuse installed launcher once', async () => {
  for (const input of [{ online: false }, { host: false }, {}]) {
    const f = fixture(input), control = createControl(f.options)
    assert.equal((await control.connect()).base, base)
    assert.equal(f.launches(), Object.keys(input).length ? 1 : 0)
  }
})
test('update resume waits on its pinned endpoint without launching a second desktop', async () => {
  const f = fixture({ host: false }), control = createControl(f.options)
  await assert.rejects(control.connect(base, { waitOnly: true }), /win_host_start_timeout/)
  assert.equal(f.launches(), 0)
})
test('node token loss is rebound during exact release wait', async () => {
  const f = fixture(), original = f.options.request
  let failures = 0
  f.options.request = async (url, body) => {
    if (!url.endsWith('/bootstrap') && ++failures === 1) throw Error('transport_http_401')
    return original(url, body)
  }
  const control = createControl(f.options)
  await control.connect(base, { waitOnly: true })
  assert.equal((await control.waitRelease(target)).release_identity, target)
  assert.equal(f.bootstraps(), 2)
})
test('descriptor origin and mutation whitelist are enforced; lost mutation is not retried', async () => {
  const bad = fixture()
  bad.options.request = async () => ({ mcp: { url: 'http://example.com/token' } })
  await assert.rejects(createControl(bad.options).connect(), /invalid_mcp_descriptor/)
  const f = fixture(), control = createControl(f.options)
  await control.connect()
  await assert.rejects(control.submit('evaluate', 'script'), /invalid_control_action/)
  await assert.rejects(control.submit('update_and_restart', 'latest'), /invalid_control_action/)
  assert.equal(exactRelease(target), true); assert.equal(exactRelease(target + '/path'), false)
})
test('wrong action identity cannot be accepted as the requested receipt', async () => {
  const f = fixture(), control = createControl(f.options)
  await control.connect()
  await assert.rejects(control.actionStatus('win_act_' + 'c'.repeat(32), 'navigate'), /invalid_action_receipt/)
})

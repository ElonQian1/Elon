const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const assert = require('node:assert/strict')
const { test } = require('node:test')
const script = fs.readFileSync(path.join(__dirname, '../../desktop-shell/src-tauri/src/codex_semantic_bridge/open_group_workbench.js'), 'utf8')
function run(href, bootstrap = {}) {
  let next, scheduled
  vm.runInNewContext(script, { URL, window: { location: { href, assign: url => { next = url } },
    __ELON_PC_BOOTSTRAP__: bootstrap, setTimeout: fn => { scheduled = fn } } })
  assert.equal(next, undefined)
  scheduled()
  return new URL(next)
}
test('local workbench opens the configured cloud group route without copying credentials', () => {
  const url = run('http://127.0.0.1:7803/pc/local-tasks?private=value', {
    mode: 'local', cloudBaseUrl: 'https://cloud.example', localNodeBaseUrl: 'http://127.0.0.1:7803',
  })
  assert.equal(url.origin, 'https://cloud.example')
  assert.equal(url.pathname, '/pc/friends')
  assert.equal(url.searchParams.get('node_admin'), 'http://127.0.0.1:7803/')
  assert.equal(url.searchParams.has('private'), false)
})
test('cloud navigation keeps its current origin and fixed route', () => {
  const url = run('https://cloud.example/pc/ai#draft')
  assert.equal(url.origin, 'https://cloud.example')
  assert.equal(url.pathname, '/pc/friends')
  assert.equal(url.hash, '')
})
test('rejects credential URLs and non-loopback admin targets', () => {
  for (const bootstrap of [
    { cloudBaseUrl: 'javascript:alert(1)' }, { cloudBaseUrl: 'https://secret@cloud.example' },
    { cloudBaseUrl: 'https://cloud.example?token=private' }, { localNodeBaseUrl: 'http://outside.example' },
  ]) assert.throws(() => run('http://127.0.0.1:7799/pc', bootstrap), /invalid_/)
})

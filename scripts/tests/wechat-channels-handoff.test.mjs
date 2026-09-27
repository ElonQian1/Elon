import test from 'node:test'
import assert from 'node:assert/strict'
import vm from 'node:vm'
import { readFileSync } from 'node:fs'
const source = readFileSync(new URL('../../android/app/src/main/assets/wechat_channels_handoff.js', import.meta.url), 'utf8')
const url = 'https://channels.weixin.qq.com/finder-preview/pages/sph?id=sample123'
const scene = () => ({ dynamicExportId: 'export/abcdefgh12345678', expiredTime: Math.floor(Date.now() / 1000) + 300, entryScene: 64, entryCardType: 48, commentScene: 39, requestScene: 0 })
function host(fetcher) {
  const context = vm.createContext({ window: {}, document: { cookie: '' }, URL, AbortController, setTimeout, clearTimeout, location: { href: url, origin: new URL(url).origin, pathname: new URL(url).pathname }, fetch: fetcher })
  return { context, api: vm.runInContext(source, context) }
}
const flush = () => new Promise(resolve => setImmediate(resolve))
test('only exact HTTPS Channels pages and valid IDs', () => {
  const { api } = host()
  assert.equal(api.identity(url), 'sample123')
  assert.equal(api.identity('https://weixin.qq.com/sph/sample123'), 'sample123')
  for (const bad of ['http://weixin.qq.com/sph/sample123', 'https://weixin.qq.com.evil.test/sph/sample123', 'https://user:pass@weixin.qq.com/sph/sample123', url + '&id=other', url + '#x', 'https://channels.weixin.qq.com:8443/finder-preview/pages/sph?id=a']) assert.equal(api.identity(bad), null)
})
test('matches official feed scheme and rejects expired or malformed scene', () => {
  const { api } = host()
  assert.match(decodeURIComponent(api.buildScene(scene(), Date.now())), /^weixin:\/\/biz\/finder\/openFinderFeed\/exportId=export\/abcdefgh12345678&actionType=0&commentScene=39/)
  for (const patch of [{ expiredTime: 1 }, { dynamicExportId: 'export/12345678&evil=1' }, { entryScene: '64' }, { requestScene: -1 }]) assert.throws(() => api.buildScene({ ...scene(), ...patch }, Date.now()))
})
test('same-origin read only; one fetch per nonce; response never exposes metadata', async () => {
  let calls = 0
  const { api } = host(async (path, init) => {
    calls++
    assert.equal(path, '/finder-preview/api/feed/get_feed_info?_pageUrl=https%3A%2F%2Fchannels.weixin.qq.com%2Ffinder-preview%2Fpages%2Fsph')
    assert.equal(init.credentials, 'same-origin')
    assert.equal(init.redirect, 'error')
    assert.deepEqual(JSON.parse(init.body), { baseReq: { generalToken: '' }, shortUri: 'sample123' })
    return { ok: true, text: async () => JSON.stringify({ errCode: 0, data: { sceneInfo: scene(), privateContent: 'never-return' } }) }
  })
  api.start(url, 'one'); api.start(url, 'one'); await flush()
  assert.equal(calls, 1)
  assert.equal(api.read('one').status, 'ready')
  assert.equal(JSON.stringify(api.read('one')).includes('never-return'), false)
})
test('page token stays in the same-origin request, never in the native projection', async () => {
  const { api, context } = host(async (_path, init) => {
    assert.equal(JSON.parse(init.body).baseReq.generalToken, 'test-only-cookie')
    return { ok: true, text: async () => JSON.stringify({ errCode: 0, data: { sceneInfo: scene() } }) }
  })
  context.document.cookie = 'token=test-only-cookie'
  api.start(url, 'token-test'); await flush()
  assert.equal(api.read('token-test').status, 'ready')
  assert.equal(JSON.stringify(api.read('token-test')).includes('test-only-cookie'), false)
})
test('navigation and replacement invalidate pending results', async () => {
  let resolve
  const { api, context } = host(() => new Promise(r => { resolve = r }))
  api.start(url, 'old')
  context.location.href = url.replace('sample123', 'another')
  resolve({ ok: true, text: async () => JSON.stringify({ errCode: 0, data: { sceneInfo: scene() } }) })
  await flush()
  assert.equal(api.read('old').status, 'stale')
})
test('failure never reports readiness or leaks server error', async () => {
  for (const response of [{ ok: false }, { ok: true, text: async () => '{' }, { ok: true, text: async () => JSON.stringify({ errCode: 99, errMsg: 'sensitive' }) }]) {
    const { api } = host(async () => response)
    api.start(url, 'failed'); await flush()
    assert.equal(api.read('failed').status, 'failed')
    assert.equal(api.read('failed').url, undefined)
    assert.equal(JSON.stringify(api.read('failed')).includes('sensitive'), false)
  }
})

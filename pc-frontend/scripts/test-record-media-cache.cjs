const { test } = require('node:test')
const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const Module = require('node:module')
const ts = require('typescript')
const root = path.resolve(__dirname, '../src/features/friends/chat-records')
function compile(name, imports = {}) {
  const file = path.join(root, name + '.ts'), mod = new Module(file, module)
  mod.require = id => imports[id] || require(id)
  mod._compile(ts.transpileModule(fs.readFileSync(file, 'utf8'), { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText, file)
  return mod.exports
}
const cacheModule = compile('recordMediaCache')
const { RecordMediaCache, recordMediaCache } = cacheModule
const blob = text => new Blob([text], { type: 'image/png' })
test('memory LRU stays bounded even when persistent storage is unavailable', async () => {
  const cache = new RecordMediaCache({ memoryBytes: 6 })
  await cache.put('a', blob('aaa')); await cache.put('b', blob('bbb'))
  await cache.get('a'); await cache.put('c', blob('ccc'))
  assert.equal(await cache.get('b'), undefined)
  assert.equal(await (await cache.get('a')).text(), 'aaa')
  assert.equal(await (await cache.get('c')).text(), 'ccc')
})
test('expiry, invalid sizes and stale writers never create reusable entries', async () => {
  let time = 100
  const cache = new RecordMediaCache({ now: () => time, maxAge: 10 })
  await cache.put('a', blob('a')); time = 111
  assert.equal(await cache.get('a'), undefined)
  await cache.put('empty', blob('')); await cache.put('stale', blob('a'), () => false)
  assert.equal(await cache.get('empty'), undefined); assert.equal(await cache.get('stale'), undefined)
})
test('prefix clearing isolates users, records and origins', async () => {
  const cache = new RecordMediaCache()
  for (const key of ['one|site/record|a', 'one|site/other|a', 'two|site/record|a']) await cache.put(key, blob('a'))
  await cache.clear('one|site/record|')
  assert.equal(await cache.get('one|site/record|a'), undefined)
  assert.ok(await cache.get('one|site/other|a')); assert.ok(await cache.get('two|site/record|a'))
})

let state = { token: 'fixture', user: { id: 'one' } }, listeners = [], calls = 0, replies = []
global.location = { href: 'http://local.test/pc' }
const loader = compile('recordMediaLoader', {
  '../../../api/client': { getAuthToken: () => state.token },
  '../../../api/runtime': { resolveApiUrl: path => 'http://fixture.test' + path },
  '../../../store/auth': { useAuthStore: { getState: () => state, subscribe: fn => listeners.push(fn) } },
  './recordMediaCache': cacheModule,
  './recordApi': {
    recordPath: c => `/api/me/groups/${c.group_id}/chat-records/${c.record_id}`,
    recordRequest: (_path, signal) => { calls++; return new Promise((resolve, reject) => {
      replies.push(resolve); signal.addEventListener('abort', () => reject(new DOMException('aborted', 'AbortError')), { once: true })
    }) },
  },
})
const card = { group_id: 'group', record_id: 'record' }
const turn = () => new Promise(resolve => setImmediate(resolve))
test('shared downloads survive one cancelled subscriber and warm hits do not fetch', async () => {
  const a = new AbortController(), b = new AbortController(), baseline = calls
  const one = loader.loadRecordMedia(card, 'image', a.signal), two = loader.loadRecordMedia(card, 'image', b.signal)
  const rejected = assert.rejects(one, { name: 'AbortError' })
  await turn(); assert.equal(calls, baseline + 1)
  a.abort(); replies.shift()(blob('image')); await rejected
  assert.equal(await (await two).text(), 'image')
  assert.equal(await (await loader.loadRecordMedia(card, 'image', b.signal)).text(), 'image')
  assert.equal(calls, baseline + 1)
})
test('clearing during a download aborts it and prevents late cache repopulation', async () => {
  const pending = loader.loadRecordMedia(card, 'late', new AbortController().signal)
  const rejected = assert.rejects(pending, { name: 'AbortError' })
  await turn(); const reply = replies.shift()
  await loader.clearRecordMedia(card); reply(blob('late')); await rejected
  assert.equal(await recordMediaCache.get('one|http://fixture.test/api/me/groups/group/chat-records/record|late'), undefined)
})
test('account changes cancel pending readers and do not reuse another account bytes', async () => {
  const pending = loader.loadRecordMedia(card, 'owner', new AbortController().signal)
  const rejected = assert.rejects(pending, { name: 'AbortError' })
  await turn(); const stale = replies.shift(), previous = state
  state = { token: 'fixture-other', user: { id: 'two' } }; listeners.forEach(fn => fn(state, previous))
  stale(blob('private')); await rejected
  const baseline = calls, fresh = loader.loadRecordMedia(card, 'owner', new AbortController().signal)
  await turn(); assert.equal(calls, baseline + 1); replies.shift()(blob('current'))
  assert.equal(await (await fresh).text(), 'current')
})

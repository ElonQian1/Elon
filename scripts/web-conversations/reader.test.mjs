import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import vm from 'node:vm'
import { webcrypto } from 'node:crypto'
import { createService, conversationId } from './service.mjs'
import { localUrl } from './transport.mjs'

const projection = createRequire(import.meta.url)('../../android/app/src/main/assets/chatgpt_web_conversation_projection.js')
const readerScript = readFileSync(new URL('../../android/app/src/main/assets/chatgpt_web_conversation_reader.js', import.meta.url), 'utf8')
const id = '00000000-0000-4000-8000-000000000001'
const other = '00000000-0000-4000-8000-000000000002'
const message = (id, text) => ({ id, author: { role: 'user' }, content: { parts: [text] } })
const raw = messages => ({ id, messages })
const tick = () => new Promise(resolve => setImmediate(resolve))

test('120 messages and a long Unicode message survive every page without duplication', () => {
  const long = '中文😀\n'.repeat(30000)
  const messages = Array.from({ length: 120 }, (_, index) => message(`m-${index}`, index === 66 ? long : `第${index}条`))
  const snapshot = projection.project(raw(messages), id), blocks = []
  let offset = 0, page
  do { page = projection.page(snapshot, 'a'.repeat(32), offset); blocks.push(...page.blocks); offset += page.blocks.length
    assert.ok(Buffer.byteLength(JSON.stringify(page)) < 60 * 1024)
  } while (page.has_more)
  assert.equal(snapshot.message_count, 120)
  for (const item of messages) assert.equal(blocks.filter(b => b.message_id === item.id).map(b => b.text).join(''), item.content.parts[0])
  assert.equal(snapshot.text_complete, true)
})

test('selected branch is retained and ambiguous, cyclic, partial or wrong sources fail', () => {
  const value = { id, current_node: 'b', mapping: {
    root: { parent: null, message: message('root', 'start') },
    a: { parent: 'root', message: message('a', 'not selected') },
    b: { parent: 'root', message: message('b', 'selected') },
  } }
  assert.deepEqual(projection.project(value, id).blocks.map(b => b.text), ['start', 'selected'])
  assert.throws(() => projection.project({ ...value, current_node: null }, id), /branch_ambiguous/)
  value.mapping.root.parent = 'b'
  assert.throws(() => projection.project(value, id), /invalid_branch/)
  assert.throws(() => projection.project(raw([]), other), /conversation_mismatch/)
  assert.throws(() => projection.project({ ...raw([]), has_more: true }, id), /source_incomplete/)
  assert.equal(projection.project(raw([]), id).message_count, 0)
})

test('attachments and unsupported content are explicit gaps and never disclose download credentials', () => {
  const value = message('image', 'caption')
  value.content.parts.push({ content_type: 'image_asset_pointer', asset_pointer: 'https://private/?token=secret' }, { type: 'unknown' })
  value.metadata = { attachments: [{ name: 'file.pdf', download_url: 'https://private/?token=secret' }] }
  const snapshot = projection.project(raw([value]), id)
  assert.equal(snapshot.attachment_count, 2)
  assert.equal(snapshot.text_complete, false)
  assert.equal(snapshot.multimodal_complete, false)
  assert.doesNotMatch(JSON.stringify(snapshot), /secret|private/)
})

function pageFixture() {
  let auth = 'Bearer synthetic-a', now = Date.now(), fetches = 0
  const headers = () => ({ authorization: auth, 'account-id': 'synthetic' })
  const window = { location: { origin: 'https://chatgpt.com' }, crypto: webcrypto,
    __elonConversationProjection: projection,
    __elonChatGptPrivateTransport: { conversationPrefetchEnabled: true, acquireSameOriginRequestHeaders: async () => headers(), copySameOriginRequestHeaders: headers },
    __elonChatGptPrivateJsonRequest: { request: async (_, url, options) => {
      assert.equal(url, `/backend-api/conversations/${id}`); assert.equal(options.method, 'GET'); fetches++
      return { payload: raw(Array.from({ length: 7 }, (_, index) => message(`${index}`, 'hello'))) }
    } },
  }
  vm.runInNewContext(readerScript, { window, Date: { now: () => now }, Uint8Array })
  return { run: window.__elonConversationReader.run, change: () => { auth = 'Bearer synthetic-b' },
    advance: ms => { now += ms }, fetches: () => fetches }
}
test('asynchronous reader binds cached replies and cursors to account identity', async () => {
  const f = pageFixture(), input = { conversation_id: id, request_id: 'request-001' }
  assert.equal(f.run(input).status, 'pending'); await tick()
  const reply = f.run(input)
  assert.equal(reply.status, 'ready'); assert.equal(f.fetches(), 1)
  assert.doesNotMatch(JSON.stringify(reply), /Bearer|synthetic/)
  assert.equal(f.run({ ...input, cursor: 'x' }).error, 'request_conflict')
  f.change()
  assert.equal(f.run(input).error, 'identity_changed')
  const next = { ...input, request_id: 'request-002', cursor: reply.page.next_cursor }
  f.run(next); await tick(); assert.equal(f.run(next).error, 'cursor_expired')
})
test('idle cursor expiration is explicit; active paging renews the snapshot', async () => {
  const f = pageFixture(), input = { conversation_id: id, request_id: 'request-001' }
  f.run(input); await tick(); const page = f.run(input).page
  f.advance(160000)
  const next = { ...input, request_id: 'request-002', cursor: page.next_cursor }
  f.run(next); await tick(); const nextPage = f.run(next).page
  f.advance(160000)
  const third = { ...input, request_id: 'request-003', cursor: nextPage.next_cursor }
  f.run(third); await tick(); assert.equal(f.run(third).status, 'ready')
  f.advance(180001)
  const expired = { ...third, request_id: 'request-004' }
  f.run(expired); await tick(); assert.equal(f.run(expired).error, 'cursor_expired')
})

test('service authorization happens before device discovery; rejects unsafe URLs', async () => {
  let touched = false
  const service = createService({ env: {}, factories: { win: () => { touched = true } } })
  await assert.rejects(service.read({ reference: id }), /not_authorized/); assert.equal(touched, false)
  assert.equal(conversationId(`chatgpt-conversation://${id}`), id)
  for (const ref of [`https://evil.test/c/${id}`, `https://chatgpt.com/c/${id}?token=1`, `https://chatgpt.com/c/${id}/extra`]) assert.throws(() => conversationId(ref))
  for (const url of ['https://127.0.0.1', 'http://localhost', 'http://example.test', 'http://127.0.0.1/path']) assert.throws(() => localUrl(url))
})

test('service cursor pins device and revision and detects account/source changes', async () => {
  let revision = 'a'.repeat(32), reads = 0
  const snapshot = projection.project(raw([message('a', 'a'), message('b', 'b'), message('c', 'c')]), id)
  const host = { source: 'win', read: async input => { reads++; return { status: 'ready', request_id: input.request_id,
    page: projection.page(snapshot, revision, input.cursor ? 2 : 0) } } }
  const service = createService({ env: { ELON_WEB_CONVERSATION_IDS: id }, factories: { win: async () => host } })
  const page = await service.read({ reference: id })
  assert.equal(page.has_more, true)
  await assert.rejects(service.read({ reference: id, cursor: page.next_cursor, source: 'apk' }), /source_mismatch/)
  assert.equal(reads, 1)
  revision = 'b'.repeat(32)
  await assert.rejects(service.read({ reference: id, cursor: page.next_cursor }), /invalid_conversation_result/)
})

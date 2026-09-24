import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { webcrypto } from 'node:crypto'
import vm from 'node:vm'

const projection = createRequire(import.meta.url)('../../android/app/src/main/assets/chatgpt_web_conversation_projection.js')
const script = readFileSync(new URL('../../android/app/src/main/assets/chatgpt_web_conversation_reader.js', import.meta.url), 'utf8')
const id = '00000000-0000-4000-8000-000000000001'
const other = '00000000-0000-4000-8000-000000000002'
const message = n => ({ id: `m${n}`, author: { role: n % 2 ? 'assistant' : 'user' },
  metadata: { parent_id: n ? `m${n - 1}` : 'omitted-hidden-root' }, content: { parts: [`text${n}`] } })
const batch = (numbers, cursor = null) => ({ messages: numbers.map(message),
  page_info: { has_previous_page: cursor !== null, start_cursor: cursor } })
const first = () => ({ conversation_id: id, current_node: 'm5', ...batch([4, 5], 'older/2?opaque') })

async function read(responder) {
  const urls = [], headers = { authorization: 'Bearer synthetic-only' }
  let owner = headers
  const page = { location: { origin: 'https://chatgpt.com' }, crypto: webcrypto,
    __elonConversationProjection: projection,
    __elonChatGptPrivateAuthContext: { copyRequestHeaders: () => owner },
    __elonChatGptPrivateJsonRequest: { request: async (_, url, options) => {
      assert.equal(options.method, 'GET'); assert.equal(options.credentials, 'include'); urls.push(url)
      return { payload: await responder(url, urls.length, () => { owner = { authorization: 'Bearer changed' } }) }
    } },
  }
  vm.runInNewContext(script, { window: page, Date, Uint8Array })
  const input = { conversation_id: id, request_id: 'pagination-001' }
  assert.equal(page.__elonConversationReader.run(input).status, 'pending')
  await new Promise(resolve => setImmediate(resolve))
  return { result: page.__elonConversationReader.run(input), urls, page, input }
}

test('official complete Message[] uses chronological page order despite omitted metadata parents', () => {
  const value = { conversation_id: id, current_node: 'm1', ...batch([0, 1]) }
  assert.deepEqual(projection.project(value, id).blocks.map(b => b.text), ['text0', 'text1'])
  assert.throws(() => projection.project(first(), id), /source_incomplete/)
  assert.throws(() => projection.project({ ...value, current_node: 'other-branch' }, id), /invalid_branch/)
  assert.throws(() => projection.project({ ...value, messages: [message(1), message(1)] }, id), /invalid_branch/)
})

test('older pages are prepended, opaque cursors encoded, and output remains a stable snapshot', async () => {
  const f = await read((_, count) => [first(), batch([2, 3], 'older-1'), batch([0, 1]), first()][count - 1])
  assert.equal(f.result.status, 'ready')
  assert.equal(f.result.page.message_count, 6)
  assert.deepEqual(f.urls, [`/backend-api/conversations/${id}`,
    `/backend-api/conversations/${id}/messages?before=older%2F2%3Fopaque&include_has_versions=true`,
    `/backend-api/conversations/${id}/messages?before=older-1&include_has_versions=true`, `/backend-api/conversations/${id}`])
  let current = f.result.page, texts = current.blocks.map(b => b.text), count = 0
  while (current.has_more) {
    const input = { ...f.input, request_id: 'continuation-' + (++count), cursor: current.next_cursor }
    f.page.__elonConversationReader.run(input); await new Promise(resolve => setImmediate(resolve))
    current = f.page.__elonConversationReader.run(input).page; texts.push(...current.blocks.map(b => b.text))
  }
  assert.deepEqual(texts, [0, 1, 2, 3, 4, 5].map(n => `text${n}`))
  assert.equal(f.urls.length, 4)
  assert.equal(current.text_complete, true)
})

test('empty intermediate pages advance using the official cursor', async () => {
  const f = await read((_, count) => [first(), batch([], 'empty-next'), batch([0, 1, 2, 3]), first()][count - 1])
  assert.equal(f.result.status, 'ready'); assert.equal(f.result.page.message_count, 6)
})

test('incomplete, repeated, cross-conversation, overlapping and changed sources never emit partial success', async () => {
  for (const [next, error] of [
    [batch([], 'older/2?opaque'), 'source_incomplete'],
    [{ ...batch([0, 1]), page_info: { has_previous_page: true } }, 'source_incomplete'],
    [{ messages: [message(0)] }, 'source_incomplete'],
    [{ ...batch([0, 1]), conversation_id: other }, 'conversation_mismatch'],
    [batch([3, 4]), 'invalid_branch'],
    [{ ...batch([0, 1]), has_more: true }, 'source_incomplete'],
  ]) {
    const f = await read((_, count) => count === 1 ? first() : next)
    assert.equal(f.result.status, 'failed'); assert.equal(f.result.error, error); assert.equal(f.result.page, undefined)
  }
  const drift = await read((_, count) => count === 1 ? first() : count === 2 ? batch([0, 1, 2, 3]) : { ...first(), current_node: 'changed' })
  assert.equal(drift.result.error, 'source_incomplete')
  const account = await read((_, count, change) => { if (count === 2) change(); return count === 1 ? first() : batch([0, 1, 2, 3]) })
  assert.equal(account.result.error, 'identity_changed')
})

test('history collection is bounded even when the server never signals completion', async () => {
  const f = await read((_, count) => count === 1 ? first() : batch([], `more-${count}`))
  assert.equal(f.result.error, 'source_limit'); assert.equal(f.urls.length, 101)
})

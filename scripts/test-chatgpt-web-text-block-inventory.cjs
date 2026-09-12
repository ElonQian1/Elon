'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const base = '../android/app/src/main/assets/';
const inventory = require(base + 'chatgpt_web_text_block_inventory');
const id = '11111111-1111-4111-8111-111111111111';
function harness() {
  const state = { account: 'owner-a', requests: [], payload: { conversation_id: id,
    messages: [{ id: 'a1', author: { role: 'assistant' }, status: 'finished_successfully',
      content: { content_type: 'text', parts: [':::writing{id="block-a" variant="email"}\nprivate body\n:::\n```py\nprivate code\n```'] } }] } };
  const page = { document: {}, location: { href: 'https://chatgpt.com/c/' + id }, __elonChatGptDocumentToken: 'doc_test_scope',
    __elonChatGptPrivateConversationShareContract: { create: () => ({ identity: () => state.account }) },
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: 'do not export' }) },
    __elonChatGptPrivateHistoryProjection: require(base + 'chatgpt_web_private_history_projection'),
    __elonChatGptTextBlocks: require(base + 'chatgpt_web_text_blocks'),
    __elonChatGptPrivateJsonRequest: { async request(_, path, init, limits) {
      state.requests.push({ path, init, limits });
      await state.wait?.(); state.afterRead?.();
      if (state.error) throw Error('private error');
      return { payload: state.payload };
    } } };
  return { state, page, core: inventory.create(page) };
}
test('one bounded same-origin GET reports only canonical block counts, without composer or DOM', async () => {
  const h = harness(), value = await h.core.read();
  assert.deepEqual(value, { schema: 'elon.text_block_inventory.v2', status: 'ready', messages: 1,
    code_blocks: 1, writing_blocks: 1, writable_blocks: 1, metadata_messages: 0, unparsed_messages: 0, bounded: true,
    dom_id_matches: 0, dom_code_matches: 0, dom_writing_matches: 0, dom_line_ending_matches: 0 });
  const request = h.state.requests[0]; assert.equal(h.state.requests.length, 1);
  assert.equal(request.path, '/backend-api/conversation/' + id); assert.equal(request.init.method, 'GET');
  assert.equal(request.init.credentials, 'same-origin'); assert.equal(request.init.redirect, 'error');
  assert.deepEqual(request.limits, { timeoutMs: 8000, maxBytes: 4194304, mode: 'json' });
  assert.equal(JSON.stringify(value).includes('private'), false);
});
test('missing identity, foreign origin, changed document/account and malformed branch are not zero-capability claims', async () => {
  for (const change of [h => h.state.account = null, h => h.page.location.href = 'https://other.test/c/' + id]) {
    const h = harness(); change(h); assert.equal((await h.core.read()).status, 'context_unavailable');
    assert.equal(h.state.requests.length, 0);
  }
  for (const change of [h => h.page.document = {}, h => h.state.account = 'other',
    h => h.page.__elonChatGptDocumentToken = 'doc_new_owner', h => h.page.location.href += '#other']) {
    const h = harness(); h.state.afterRead = () => change(h);
    assert.equal((await h.core.read()).status, 'context_changed');
  }
  const h = harness(); h.state.payload = { conversation_id: id, current_node: 'missing', mapping: { a: {} } };
  assert.equal((await h.core.read()).status, 'invalid_response');
  h.state.error = true; assert.equal((await h.core.read()).status, 'read_failed');
});
test('concurrent probes do not duplicate GET and failures release the single-flight slot', async () => {
  const h = harness(); let release; h.state.wait = () => new Promise(resolve => { release = resolve; });
  const pending = h.core.read(); assert.equal((await h.core.read()).status, 'busy');
  release(); assert.equal((await pending).status, 'ready'); assert.equal(h.state.requests.length, 1);
  h.state.wait = null; assert.equal((await h.core.read()).status, 'ready');
});
test('read-only comparison distinguishes exact bodies, newline rendering and message identity', async () => {
  const h = harness();
  h.page.__elonChatGptMessages = { readMessages: () => [{ id: 'a1', role: 'assistant', content: [
    { textBlock: { content: 'private body\n' } }, { textBlock: { content: 'private code' } }
  ] }] };
  const value = await h.core.read();
  assert.equal(value.dom_id_matches, 1); assert.equal(value.dom_writing_matches, 1);
  assert.equal(value.dom_code_matches, 0); assert.equal(value.dom_line_ending_matches, 1);
});

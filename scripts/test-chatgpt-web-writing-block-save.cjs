'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const crypto = require('node:crypto');
const base = '../android/app/src/main/assets/';
const parser = require(base + 'chatgpt_web_text_blocks');
const policy = require(base + 'chatgpt_web_writing_block_policy');
const transport = require(base + 'chatgpt_web_private_writing_blocks');
const context = require(base + 'chatgpt_web_writing_block_context');
const conversationId = '11111111-1111-4111-8111-111111111111', messageId = 'message-a', path = '/c/' + conversationId;
const original = '  Example\r\n\r\n';
function payload() {
  return { conversation_id: conversationId, is_do_not_remember: false, current_node: messageId,
    mapping: { [messageId]: { id: messageId, parent: null, message: { id: messageId, author: { role: 'assistant' },
      status: 'finished_successfully', content: { content_type: 'text', parts: [':::writing{id="block-a" variant="standard" title="Example"}\r\n' + original + ':::'] },
      metadata: { writing_blocks: { 'block-a': { content: original, variant: 'standard', title: 'Example', metadata: { tag: 'keep' } } } } } } } };
}
function harness() {
  const state = { payload: payload(), current: true, local: true, sync: true, posts: [], requests: [], effects: true, postError: null };
  const root = { crypto: crypto.webcrypto, __elonChatGptTextBlocks: parser, __elonChatGptWritingBlockPolicy: policy,
    __elonChatGptPrivateConversationMutationsEnabled: true,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({}) },
    __elonChatGptPrivateJsonRequest: { async request(_, url, init, limits) {
      state.requests.push({ url, init, limits });
      if (init.method === 'POST') {
        const body = JSON.parse(init.body); state.posts.push(body);
        if (state.effects) state.payload.mapping[messageId].message.metadata.writing_blocks['block-a'] = { ...body.writing_block };
        if (state.postError) throw Error(state.postError);
        return {};
      }
      if (state.readError) throw Error(state.readError);
      return { payload: structuredClone(state.payload) };
    } } };
  const ctx = { async capture() { return { id: conversationId, path, current: () => state.current,
    local() { if (!state.local) throw Error('writing_web_edit_pending'); }, reconcile: async () => state.sync }; } };
  const core = transport.create(root, { context: ctx });
  const prepare = () => core.run({ operation: 'prepare', path, messageId, id: 'block-a', content: original }, false);
  return { state, root, core, prepare, save: (ticket, content = 'changed') => core.run({ operation: 'save', path, ticket, content }, true) };
}
test('explicit provider ownership is retained without exposing write metadata in display cards', () => {
  const message = payload().mapping[messageId].message;
  const value = parser.project(message);
  assert.equal(value.parts[0].textBlock.sourceMessageId, messageId);
  assert.equal(value.writeSources, undefined);
  assert.equal(parser.project(message, true).writeSources[0].content, original);
  message.content.parts = [':::writing{id="block-a"}\nbody\n:::']; message.metadata = {};
  assert.equal(parser.project(message).parts[0].textBlock.sourceMessageId, undefined);
  message.metadata = { writing_blocks: { 'block-a': { variant: 'standard', library_file_id: 'libfile_a' } } };
  assert.equal(parser.project(message).parts[0].textBlock.sourceMessageId, undefined);
});
test('branch, duplicates, streaming, promoted library and unknown ownership reject writes', () => {
  for (const mutate of [
    p => { p.current_node = 'missing'; },
    p => { p.mapping[messageId].parent = messageId; },
    p => { p.mapping[messageId].message.status = 'in_progress'; },
    p => { p.mapping[messageId].message.clientMetadata = { writingBlockOwners: {} }; },
    p => { p.mapping[messageId].message.content.parts[0] += '\n:::writing{id="block-a" variant="standard"}\nother\n:::'; },
    p => { p.mapping[messageId].message.metadata.writing_blocks['block-a'].library_file_id = 'libfile_a'; },
    p => { p.mapping[messageId].message.metadata.writing_blocks['block-a'].locallyEdited = true; },
    p => { p.is_do_not_remember = true; },
    p => { p.mapping[messageId].message.author.role = 'user'; }
  ]) { const p = payload(); mutate(p); assert.throws(() => policy.source(p, conversationId, messageId, 'block-a', parser)); }
});
test('request contract rejects extra scope, malformed text and path; accepts empty body', () => {
  const input = { operation: 'prepare', path, messageId, id: 'block-a', content: '' };
  assert.deepEqual(policy.parse(JSON.stringify(input)), input);
  for (const changed of [{ ...input, path: '/g/project/c/' + conversationId }, { ...input, content: '\ud800' },
    { ...input, content: 'a'.repeat(120001) }, { ...input, cookie: 'x' }, { ...input, id: '../file' }])
    assert.throws(() => policy.parse(JSON.stringify(changed)));
});
test('truncated block catalogs and duplicate oversized blocks grant no save ownership', () => {
  const message = payload().mapping[messageId].message;
  const literal = message.content.parts[0];
  message.content.parts = [Array.from({ length: 17 }, (_, i) => literal.replaceAll('block-a', 'block-' + i)).join('\n')];
  assert.equal(parser.project(message, true).writeSources.length, 0);
  message.content.parts = [literal + '\n:::writing{id="block-a" variant="standard"}\n' + 'a'.repeat(120001) + '\n:::'];
  message.metadata = {};
  assert.equal(parser.project(message, true).writeSources.length, 0);
});
test('save uses the verified message endpoint and preserves metadata, index and empty content', async () => {
  const h = harness(), prepared = await h.prepare(); assert.equal(prepared.code, 'writing_ready');
  const result = await h.save(prepared.ticket, '');
  assert.equal(result.code, 'writing_saved'); assert.equal(result.pending, false); assert.equal(h.state.posts.length, 1);
  const body = h.state.posts[0];
  assert.deepEqual(body.writing_block, { id: 'block-a', index: '0', variant: 'standard', title: 'Example', metadata: { tag: 'keep' }, content: '' });
  assert.equal(body.message_id, messageId); assert.equal(body.conversation_id, conversationId);
  assert.ok(!Number.isNaN(Date.parse(body.updated_at)));
  const post = h.state.requests.find(row => row.init.method === 'POST');
  assert.equal(post.url, '/backend-api/conversation/message/writing-blocks');
  assert.equal(post.init.credentials, 'same-origin'); assert.equal(post.init.redirect, 'error');
  assert.equal(post.limits.mode, 'none'); assert.ok(post.limits.timeoutMs <= 8000);
  assert.equal((await h.save(prepared.ticket, '')).code, 'writing_saved'); assert.equal(h.state.posts.length, 1);
});
test('changed provider data or pending page edits are not overwritten', async () => {
  const h = harness(), prepared = await h.prepare();
  h.state.payload.mapping[messageId].message.metadata.writing_blocks['block-a'].content = 'other client';
  assert.equal((await h.save(prepared.ticket)).code, 'writing_version_conflict'); assert.equal(h.state.posts.length, 0);
  const g = harness(), p = await g.prepare(); g.state.local = false;
  assert.equal((await g.save(p.ticket)).code, 'writing_web_edit_pending'); assert.equal(g.state.posts.length, 0);
});
test('timeout after dispatch locks out another write; verify reconciles exactly once', async () => {
  const h = harness(), prepared = await h.prepare(); h.state.postError = 'timeout';
  const first = await h.save(prepared.ticket); assert.equal(first.pending, true);
  assert.equal((await h.save(prepared.ticket, 'second')).code, 'writing_write_unconfirmed');
  assert.equal((await h.prepare()).pending, true); assert.equal(h.state.posts.length, 1);
  const verified = await h.core.run({ operation: 'verify', path, ticket: prepared.ticket }, false);
  assert.equal(verified.code, 'writing_saved'); assert.equal(verified.pending, false); assert.equal(h.state.posts.length, 1);
});
test('stale readback or pending website sync never claims success', async () => {
  const h = harness(), prepared = await h.prepare(); h.state.effects = false;
  assert.equal((await h.save(prepared.ticket)).code, 'writing_write_unconfirmed');
  assert.equal((await h.core.run({ operation: 'verify', path, ticket: prepared.ticket }, false)).pending, true);
  assert.equal(h.state.posts.length, 1);
  const g = harness(), p = await g.prepare(); g.state.sync = false;
  assert.equal((await g.save(p.ticket)).code, 'writing_saved_sync_pending');
  g.state.sync = true;
  assert.equal((await g.core.run({ operation: 'verify', path, ticket: p.ticket }, false)).code, 'writing_saved');
});
test('identity change, user confirmation and mutation switch prevent dispatch', async () => {
  const h = harness(), p = await h.prepare(); h.state.current = false;
  assert.equal((await h.save(p.ticket)).code, 'writing_selection_expired');
  h.state.current = true; h.root.__elonChatGptPrivateConversationMutationsEnabled = false;
  assert.equal((await h.save(p.ticket)).code, 'writing_disabled');
  assert.equal((await h.core.run({ operation: 'save', path, ticket: p.ticket, content: 'x' }, false)).code, 'writing_confirmation_required');
  assert.equal(h.state.posts.length, 0);
});
test('explicit permission rejection is not confused with uncertain server timeout', async () => {
  const h = harness(), p = await h.prepare(); h.state.effects = false; h.state.postError = 'http_403';
  assert.equal((await h.save(p.ticket)).code, 'writing_http_403');
  const g = harness(), q = await g.prepare(); g.state.postError = 'http_408';
  assert.equal((await g.save(q.ticket)).pending, true);
  await g.save(q.ticket); assert.equal(g.state.posts.length, 1);
});
test('command retries share one promise, conflicting payload rejected, receipts never include body', async () => {
  const h = harness(), p = await h.prepare(), replies = [], events = [];
  const command = { value: JSON.stringify({ operation: 'save', path, ticket: p.ticket, content: 'changed' }), selected: true, requestId: 'mcp_wb1' };
  const reply = (...v) => replies.push(v);
  h.core.handle('writing_block', command, reply, () => ({}), e => events.push(e));
  h.core.handle('writing_block', command, reply, () => ({}), e => events.push(e));
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(h.state.posts.length, 1); assert.equal(replies.length, 2);
  assert.equal(JSON.stringify(events).includes('changed'), false);
  h.core.handle('writing_block', { ...command, value: command.value.replace('changed', 'different') }, reply, () => ({}), () => {});
  assert.equal(replies.at(-1)[2], 'writing_request_conflict');
});
for (const localEditDuringRead of [false, true]) test('context hydrate guards late local edits=' + localEditDuringRead + ' without composer/DOM', async () => {
  let account = 'session-a', message = payload().mapping[messageId].message;
  const selected = { id: 'client-a', serverId$: () => conversationId };
  const root = { document: {}, location: { href: 'https://chatgpt.com' + path }, __elonChatGptDocumentToken: 'doc_test_owner',
    setTimeout, clearTimeout, AbortController, __elonChatGptTextBlocks: parser, __elonChatGptWritingBlockPolicy: policy,
    __elonChatGptPrivateConversationShareContract: { create: () => ({ identity: () => account }) } };
  const shared = { canvasConversations: () => [selected], XM: () => ({}), Fl: () => false,
    HM: { getNodeIfExists: () => ({ message }), getCurrentLeafId: () => messageId, getRequestId: () => null } };
  const conversation = { async textHydrateHistory(_, options) {
    const next = payload(); next.mapping[messageId].message.metadata.writing_blocks['block-a'].content = 'saved';
    options.onConversationLoadedFromNetwork(next);
    if (localEditDuringRead) message.metadata.writing_blocks['block-a'].content = 'new local edit';
    assert.equal(options.shouldApplyResponse(), !localEditDuringRead);
    if (options.shouldApplyResponse()) message = next.mapping[messageId].message;
  } };
  root.__elonChatGptPrivateRuntimeBindings = { state: () => ({ profile_id: 'web_20260912' }), load: async role => ({ shared, conversation })[role] };
  const binding = await context.create(root).capture({ path }, () => ({ url: root.location.href, streaming: false, composerReady: false }));
  const source = policy.source(payload(), conversationId, messageId, 'block-a', parser);
  binding.local(source);
  assert.equal(await binding.reconcile({ ...source, content: 'saved' }, Date.now() + 1000), !localEditDuringRead);
  assert.equal(message.metadata.writing_blocks['block-a'].content, localEditDuringRead ? 'new local edit' : 'saved');
  account = 'session-b'; assert.equal(binding.current(), false);
});

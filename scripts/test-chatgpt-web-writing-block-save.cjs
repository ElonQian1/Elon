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
  const state = { payload: payload(), current: true, local: true, sync: true, posts: [], requests: [], effects: true,
    postError: null, now: Date.now(), refreshes: [] };
  const root = { crypto: crypto.webcrypto, __elonChatGptTextBlocks: parser, __elonChatGptWritingBlockPolicy: policy,
    __elonChatGptPrivateConversationMutationsEnabled: true,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({}),
      prefetchConversation: (...args) => state.refreshes.push(args) },
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
  const core = transport.create(root, { context: ctx, now: () => state.now });
  const prepare = (id = 'block-a') => core.run({ operation: 'prepare', path, messageId, id, content: original }, false);
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

function selectBlock(h, index) {
  const id = 'block-' + index;
  const next = payload(), message = next.mapping[messageId].message;
  message.content.parts[0] = message.content.parts[0].replace('block-a', id);
  message.metadata.writing_blocks = { [id]: message.metadata.writing_blocks['block-a'] };
  h.state.payload = next;
  return h.prepare(id);
}

test('expired preparations and bounded nonpending eviction do not permanently exhaust the editor', async () => {
  for (const expire of [false, true]) {
    const h = harness(); let first;
    for (let i = 0; i < 16; i++) {
      const p = await selectBlock(h, i); assert.equal(p.code, 'writing_ready'); first ||= p;
      h.state.now += 1;
    }
    if (expire) h.state.now += 1800001;
    assert.equal((await selectBlock(h, 16)).code, 'writing_ready');
    assert.equal((await h.save(first.ticket)).code, 'writing_selection_expired');
    assert.equal(h.state.posts.length, 0);
  }
});

test('uncertain writes survive TTL and cache pressure; only readback releases the barrier', async () => {
  const h = harness(), p = await h.prepare(); h.state.postError = 'timeout';
  assert.equal((await h.save(p.ticket)).pending, true);
  const written = structuredClone(h.state.payload);
  h.state.now += 1800001;
  for (let i = 0; i < 18; i++) assert.equal((await selectBlock(h, i)).code, 'writing_ready');
  h.state.current = false;
  assert.equal((await h.prepare()).code, 'writing_selection_expired');
  h.state.current = true;
  assert.equal((await h.prepare()).code, 'writing_write_unconfirmed');
  assert.equal((await h.save(p.ticket, 'do not replay')).pending, true);
  h.state.payload = written;
  assert.equal((await h.core.run({ operation: 'verify', path, ticket: p.ticket }, false)).code, 'writing_saved');
  assert.equal(h.state.posts.length, 1);
});
test('command retries share one promise, conflicting payload rejected, receipts never include body', async () => {
  const h = harness(), p = await h.prepare(), replies = [], events = [];
  const command = { value: JSON.stringify({ operation: 'save', path, ticket: p.ticket, content: 'changed' }), selected: true, requestId: 'mcp_wb1' };
  const reply = (...v) => replies.push(v);
  h.core.handle('writing_block', command, reply, () => ({}), e => events.push(e));
  h.core.handle('writing_block', command, reply, () => ({}), e => events.push(e));
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(h.state.posts.length, 1); assert.equal(replies.length, 2);
  assert.equal(h.state.refreshes.length, 1); assert.equal(h.state.refreshes[0][0], path);
  assert.equal(h.state.refreshes[0][2], null);
  assert.equal(JSON.stringify(events).includes('changed'), false);
  h.core.handle('writing_block', { ...command, value: command.value.replace('changed', 'different') }, reply, () => ({}), () => {});
  assert.equal(replies.at(-1)[2], 'writing_request_conflict');
});
test('uncertain save does not refresh the native body until verified', async () => {
  const h = harness(), p = await h.prepare(); h.state.sync = false;
  h.core.handle('writing_block', { value: JSON.stringify({ operation: 'save', path, ticket: p.ticket, content: 'saved' }),
    selected: true, requestId: 'mcp_pending' }, () => {}, () => ({}), () => {});
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(h.state.refreshes.length, 0); assert.equal(h.state.posts.length, 1);
});

async function contextHarness() {
  const state = { account: 'session-a', leaf: messageId, message: payload().mapping[messageId].message,
    updates: 0, beforeUpdate: () => {}, missing: false };
  state.message.metadata.other = { keep: true };
  state.message.metadata.writing_blocks.unrelated = { content: 'keep' };
  const selected = { id: 'client-a', serverId$: () => conversationId };
  const root = { document: {}, location: { href: 'https://chatgpt.com' + path }, __elonChatGptDocumentToken: 'doc_test_owner',
    setTimeout, clearTimeout, AbortController, __elonChatGptTextBlocks: parser, __elonChatGptWritingBlockPolicy: policy,
    __elonChatGptPrivateConversationShareContract: { create: () => ({ identity: () => state.account }) } };
  const shared = { canvasConversations: () => [selected], XM: () => ({}), Fl: () => false,
    HM: { getNodeIfExists: () => ({ message: state.message }), getCurrentLeafId: () => state.leaf, getRequestId: () => null },
    writingUpdateState(id, callback) { assert.equal(id, selected.id); state.beforeUpdate(); callback({}); },
    writingTreeOwner: { updateTree(_, callback) { callback({
      containsNode: id => id === messageId && !state.missing, getMaybeMessage: () => state.message,
      updateNodeMessageMetadata(id, value) { assert.equal(id, messageId); state.updates++; Object.assign(state.message.metadata, value); }
    }); } } };
  root.__elonChatGptPrivateRuntimeBindings = { state: () => ({ profile_id: 'web_20260912' }),
    load: async role => { assert.equal(role, 'shared'); return shared; } };
  const binding = await context.create(root).capture({ path }, () => ({ url: root.location.href, streaming: false, composerReady: false }));
  const source = policy.source(payload(), conversationId, messageId, 'block-a', parser);
  binding.local(source);
  return { state, root, shared, binding, source, apply: (value = { ...source, content: 'saved' }, deadline = Date.now() + 1000) =>
    binding.reconcile(value, deadline) };
}
test('server-confirmed content updates exactly one official tree node, without composer or history reload', async () => {
  const h = await contextHarness(); assert.equal(await h.apply(), true); assert.equal(h.state.updates, 1);
  assert.equal(h.state.message.metadata.writing_blocks['block-a'].content, 'saved');
  assert.equal(h.state.message.metadata.writing_blocks['block-a'].locallyEdited, undefined);
  assert.deepEqual(h.state.message.metadata.writing_blocks.unrelated, { content: 'keep' });
  assert.deepEqual(h.state.message.metadata.other, { keep: true });
  assert.equal(await h.apply(), true); assert.equal(h.state.updates, 1);
});
for (const change of ['localEdit', 'lateLocalEdit', 'lateAccount', 'document', 'leaf', 'missing', 'deadline', 'metadata', 'message'])
  test('writing reconciliation rejects changed ' + change, async () => {
    const h = await contextHarness(); let value = { ...h.source, content: 'saved' }, deadline = Date.now() + 1000;
    const edit = () => { h.state.message.metadata.writing_blocks['block-a'].content = 'new local edit'; };
    if (change === 'localEdit') edit();
    if (change === 'lateLocalEdit') h.state.beforeUpdate = edit;
    if (change === 'lateAccount') h.state.beforeUpdate = () => { h.state.account = 'other'; };
    if (change === 'document') h.root.document = {};
    if (change === 'leaf') h.state.leaf = 'other';
    if (change === 'missing') h.state.missing = true;
    if (change === 'deadline') deadline = Date.now() - 1;
    if (change === 'metadata') value.metadata = { tag: 'changed' };
    if (change === 'message') value.messageId = 'other';
    assert.equal(await h.apply(value, deadline), false); assert.equal(h.state.updates, 0);
  });
test('missing official tree owner fails preparation without touching state', async () => {
  const h = await contextHarness(); delete h.shared.writingTreeOwner;
  await assert.rejects(context.create(h.root).capture({ path }, () => ({})), /writing_runtime_unavailable/);
});

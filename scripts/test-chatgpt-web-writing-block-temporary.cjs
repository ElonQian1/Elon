'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const fs = require('node:fs'), vm = require('node:vm');
const base = '../android/app/src/main/assets/';
const parser = require(base + 'chatgpt_web_text_blocks');
const policy = require(base + 'chatgpt_web_writing_block_policy');
const context = require(base + 'chatgpt_web_writing_block_context');
const transport = require(base + 'chatgpt_web_private_writing_blocks');
const streamPolicy = require(base + 'chatgpt_web_private_stream_policy');
const streamSource = fs.readFileSync(require.resolve(base + 'chatgpt_web_private_stream_transport'), 'utf8');
const cid = '11111111-1111-4111-8111-111111111111', mid = 'message-a', bid = 'block-a';
const path = '/?temporary-chat=true';

function harness() {
  const message = { id: mid, author: { role: 'assistant' }, status: 'finished_successfully',
    content: { content_type: 'text', parts: [':::writing{id="block-a" variant="standard"}\nOriginal\n:::'] },
    metadata: { writing_blocks: { [bid]: { content: 'Original\n', variant: 'standard', metadata: { keep: true } },
      other: { content: 'Keep other block' } } } };
  const thread = { isLoading: false, is_do_not_remember: true, mode: { kind: 'primary_assistant' } };
  const selected = { id: 'WEB:owned', serverId$: () => cid };
  const state = { account: 'account-a', personal: true, selected, owners: [selected], leaf: mid, temporary: true,
    thread, message, payload: { conversation_id: cid, is_do_not_remember: true, is_temporary_chat: true,
      current_node: mid, mapping: { [mid]: { id: mid, parent: null, message: structuredClone(message) } } },
    posts: [], requests: [], updates: 0, snapshots: 0, refreshes: 0, networkCalls: 0,
    afterRequest() {}, beforeUpdate() {}, postError: null };
  const shared = { canvasConversations: () => state.owners, XM: () => thread,
    cX: () => state.temporary, uo: () => false, wV: () => state.personal, SV: { isPersonalWorkspace() {} },
    Fl: () => false, HM: { getGizmoId: () => null, getIsNewConversation: () => false,
      getRequestId: () => null, getCurrentLeafId: () => state.leaf,
      getNodeIfExists: (_, id) => id === mid ? { message } : null },
    writingUpdateState(_, action) { state.beforeUpdate(); action(thread); },
    writingTreeOwner: { updateTree(_, action) { action({ containsNode: id => id === mid,
      getMaybeMessage: id => id === mid ? message : null,
      updateNodeMessageMetadata(_, value) { state.updates++; Object.assign(message.metadata, value); }
    }); } } };
  const session = streamPolicy.createSession({ now: Date.now });
  session.accept({ conversation_id: cid, message: structuredClone(message) });
  const page = { document: {}, location: { href: 'https://chatgpt.com' + path, origin: 'https://chatgpt.com', pathname: '/' },
    setTimeout, clearTimeout, crypto: require('node:crypto').webcrypto, __elonChatGptDocumentToken: 'doc_temporary_test',
    __elonChatGptTextBlocks: parser, __elonChatGptWritingBlockPolicy: policy,
    __elonChatGptWritingBlockContext: context, __elonChatGptPrivateConversationMutationsEnabled: true,
    __elonChatGptPrivateConversationShareContract: { create: () => ({ identity: () => state.account }) },
    __elonChatGptPrivateRuntimeBindings: { state: () => ({ profile_id: 'web_20260912' }),
      load: async () => shared, peek: () => shared },
    __elonChatGptTemporaryChat: { ownsSelectedConversation: owner => owner === state.selected },
    __elonChatGptPrivateStreamObserverEnabled: true,
    __elonChatGptPrivateStreamPolicy: { ...streamPolicy, createSession: () => session },
    fetch: async () => { state.networkCalls++; throw Error('Unexpected network'); },
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({}), prefetchConversation() { state.refreshes++; } },
    __elonChatGptPrivateJsonRequest: { async request(_, url, init) {
      state.requests.push({ url, method: init.method });
      if (init.method === 'POST') {
        assert.equal(url, '/backend-api/conversation/message/writing-blocks');
        const body = JSON.parse(init.body); state.posts.push(body);
        state.payload.mapping[mid].message.metadata.writing_blocks[bid] = body.writing_block;
      } else assert.equal(url, '/backend-api/conversation/' + cid);
      state.afterRequest(init.method);
      if (init.method === 'POST' && state.postError) throw Error(state.postError);
      return { payload: structuredClone(state.payload) };
    } } };
  vm.runInNewContext(streamSource, { window: page, location: page.location, TextDecoder, URL, Date });
  const snapshot = force => { if (force === true) state.snapshots++; return { url: 'https://chatgpt.com/',
    streaming: false, composerReady: false }; };
  const core = transport.create(page);
  const input = { operation: 'prepare', path, messageId: mid, id: bid, content: 'Original\n' };
  return { state, page, shared, session, core, snapshot, input,
    prepare: () => core.run(input, false, snapshot),
    save: ticket => core.run({ operation: 'save', path, ticket, content: 'Edited\n\n' }, true, snapshot),
    verify: ticket => core.run({ operation: 'verify', path, ticket }, false, snapshot) };
}

test('temporary save reuses one private POST, preserves privacy and updates the completed stream/native block', async () => {
  const h = harness(), prepared = await h.prepare();
  assert.equal(prepared.code, 'writing_ready');
  assert.equal((await h.save(prepared.ticket)).code, 'writing_saved');
  assert.equal(h.state.posts.length, 1); assert.equal(h.state.updates, 1);
  assert.equal(h.state.posts[0].conversation_id, cid);
  assert.deepEqual(Object.keys(h.state.posts[0]).sort(), ['conversation_id', 'id', 'index', 'message_id', 'updated_at', 'writing_block']);
  assert.equal(h.state.thread.is_do_not_remember, true); assert.equal(h.state.payload.is_do_not_remember, true);
  assert.equal(h.state.message.metadata.writing_blocks.other.content, 'Keep other block');
  const runtime = parser.runtimeProjection(h.page, mid);
  assert.equal(runtime.parts[0].textBlock.content, 'Edited\n\n');
  const merged = h.page.__elonChatGptPrivateStreamTransport.mergeMessages([
    { role: 'user', id: 'user-a', content: [{ type: 'text', text: 'Fixture' }] },
    { role: 'assistant', id: mid, state: 'completed', content: [{ type: 'markdown', text: runtime.text }, ...runtime.parts] }
  ], '/');
  assert.equal(merged[1].content.find(part => part.textBlock)?.textBlock.content, 'Edited\n\n');
  assert.equal((await h.save(prepared.ticket)).code, 'writing_saved'); assert.equal(h.state.posts.length, 1);
  assert.equal(h.state.networkCalls, 0); assert.equal(h.state.refreshes, 0);
});

test('missing, ambiguous, shared, project, unpersisted or ordinary owners cannot authorize temporary writes', async () => {
  for (const change of [h => { h.state.selected = null; }, h => { h.state.owners.push(h.state.selected); },
    h => { h.state.selected.serverId$ = () => null; }, h => { h.state.temporary = false; },
    h => { h.state.thread.is_do_not_remember = false; }, h => { h.state.personal = false; },
    h => { h.state.thread.isLoading = true; }, h => { h.shared.HM.getIsNewConversation = () => true; },
    h => { h.shared.HM.getGizmoId = () => 'g-p-' + 'a'.repeat(32); },
    h => { h.state.thread.mode.kind = 'gizmo_interaction'; }, h => { h.state.thread.contextScopes = ['shared']; },
    h => { h.state.thread.sharedProjectConversationOwner = {}; },
    h => { h.state.thread.continuingFromSharedConversationId = cid; },
    h => { h.page.location.href = 'https://chatgpt.com/'; }]) {
    const h = harness(); change(h);
    assert.equal((await h.prepare()).ok, false); assert.equal(h.state.requests.length, 0);
    assert.equal(parser.runtimeProjection(h.page, mid), null);
  }
});

test('server scope and Library-backed documents remain rejected in temporary conversations', async () => {
  for (const change of [p => { p.is_do_not_remember = false; }, p => { delete p.is_do_not_remember; },
    p => { p.is_temporary_chat = false; }, p => { p.gizmo_id = 'g-p-' + 'a'.repeat(32); },
    p => { p.shared_project_conversation_owner = {}; }, p => { p.conversation_id = 'other'; },
    p => { p.mapping[mid].message.metadata.writing_blocks[bid].library_file_id = 'libfile_a'; }]) {
    const h = harness(); change(h.state.payload);
    assert.equal((await h.prepare()).ok, false); assert.equal(h.state.posts.length, 0);
  }
});

test('same homepage does not make a new temporary conversation, account or branch own the old ticket', async () => {
  for (const change of [h => { h.state.selected = { ...h.state.selected }; h.state.owners = [h.state.selected]; },
    h => { h.state.account = 'account-b'; }, h => { h.state.leaf = 'other'; },
    h => { h.page.__elonChatGptDocumentToken = 'doc_changed'; }, h => { h.state.temporary = false; }]) {
    const h = harness(), prepared = await h.prepare(); assert.equal(prepared.code, 'writing_ready'); change(h);
    assert.equal((await h.save(prepared.ticket)).ok, false); assert.equal(h.state.posts.length, 0);
  }
});

test('uncertain temporary POST is only read back, never resent', async () => {
  const h = harness(), prepared = await h.prepare(); h.state.postError = 'timeout';
  assert.equal((await h.save(prepared.ticket)).code, 'writing_write_unconfirmed');
  assert.equal((await h.save(prepared.ticket)).pending, true); assert.equal(h.state.posts.length, 1);
  assert.equal((await h.verify(prepared.ticket)).code, 'writing_saved');
  assert.equal(h.state.posts.length, 1); assert.equal(h.state.updates, 1);
});

test('owner or stream changes after dispatch keep sync pending instead of claiming a native update', async () => {
  const h = harness(), prepared = await h.prepare();
  h.state.beforeUpdate = () => { h.state.temporary = false; };
  assert.equal((await h.save(prepared.ticket)).code, 'writing_saved_sync_pending');
  assert.equal(h.state.updates, 0); assert.equal(h.state.posts.length, 1);
  const second = harness(), ready = await second.prepare();
  second.state.afterRequest = method => { if (method === 'POST') {
    second.session.reset(); second.session.accept({ conversation_id: cid,
      message: { ...second.state.message, status: 'in_progress' } });
  } };
  assert.equal((await second.save(ready.ticket)).code, 'writing_saved_sync_pending');
  assert.equal(second.session.current('/').state, 'streaming');
});

test('native save receipt refreshes only the current temporary snapshot, not persistent history', async () => {
  const h = harness(), prepared = await h.prepare();
  const receipt = new Promise(resolve => h.core.handle('writing_block', { requestId: 'mcp_temp', selected: true,
    value: JSON.stringify({ operation: 'save', path, ticket: prepared.ticket, content: 'Edited\n\n' }) },
  (_, ok, code) => resolve({ ok, code }), h.snapshot, value => {
    assert.equal(value.path, path); assert.equal(value.type, 'writing_block');
  }));
  assert.deepEqual(await receipt, { ok: true, code: 'writing_saved' });
  assert.equal(h.state.snapshots, 1); assert.equal(h.state.refreshes, 0); assert.equal(h.state.posts.length, 1);
});

test('temporary route rejects other query strings and shared routes', () => {
  assert.equal(policy.route(path).temporary, true);
  for (const value of ['/', path + '&x=1', path + '#block', '/c/' + cid + '?temporary-chat=true', '/share/' + cid]) {
    assert.equal(policy.route(value), null);
    assert.throws(() => policy.parse(JSON.stringify({ ...harness().input, path: value })));
  }
});

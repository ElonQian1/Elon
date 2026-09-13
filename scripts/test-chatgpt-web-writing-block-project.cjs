'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const base = '../android/app/src/main/assets/';
const parser = require(base + 'chatgpt_web_text_blocks');
const policy = require(base + 'chatgpt_web_writing_block_policy');
const context = require(base + 'chatgpt_web_writing_block_context');
const transport = require(base + 'chatgpt_web_private_writing_blocks');
const cid = '11111111-1111-4111-8111-111111111111', pid = 'g-p-' + 'a'.repeat(32), other = 'g-p-' + 'b'.repeat(32);
const mid = 'message-a', bid = 'block-a', plainPath = '/c/' + cid, projectPath = '/g/' + pid + '-example' + plainPath;

function harness(path = projectPath) {
  const message = { id: mid, author: { role: 'assistant' }, status: 'finished_successfully',
    content: { content_type: 'text', parts: [':::writing{id="block-a" variant="standard" title="Example"}\nOriginal\n:::'] },
    metadata: { writing_blocks: { [bid]: { content: 'Original', variant: 'standard', title: 'Example', metadata: { keep: true } } } } };
  const thread = { isLoading: false, is_do_not_remember: false, projectId: pid };
  const state = { account: 'identity-a', user: 'user-a', personal: true, thread, message, leaf: mid,
    payload: { conversation_id: cid, gizmo_id: pid, is_do_not_remember: false, current_node: mid,
      mapping: { [mid]: { id: mid, parent: null, message: structuredClone(message) } } },
    requests: [], posts: [], updates: 0, afterRequest() {}, beforeUpdate() {} };
  const selected = { id: 'client-a', serverId$: () => cid };
  const shared = { canvasConversations: () => [selected], XM: id => { assert.equal(id, selected.id); return thread; },
    H3: () => true, wV: () => state.personal, SV: { isPersonalWorkspace() {} },
    mq: () => ({ isWorkspaceAccount: () => false, isQuorum: () => false, normalizedAccountUserId: state.user }),
    Fl: () => false, HM: { getGizmoId: value => value.projectId, getRequestId: () => null,
      getCurrentLeafId: () => state.leaf, getNodeIfExists: (_, id) => id === mid ? { message } : null },
    writingUpdateState(id, action) { assert.equal(id, selected.id); state.beforeUpdate(); action(thread); },
    writingTreeOwner: { updateTree(_, action) { action({ containsNode: id => id === mid,
      getMaybeMessage: id => id === mid ? message : null,
      updateNodeMessageMetadata(id, value) { assert.equal(id, mid); state.updates++; Object.assign(message.metadata, value); }
    }); } } };
  const page = { document: {}, location: { href: 'https://chatgpt.com' + path }, setTimeout, clearTimeout,
    crypto: require('node:crypto').webcrypto, __elonChatGptDocumentToken: 'doc_project_test',
    __elonChatGptTextBlocks: parser, __elonChatGptWritingBlockPolicy: policy,
    __elonChatGptWritingBlockContext: context, __elonChatGptPrivateConversationMutationsEnabled: true,
    __elonChatGptPrivateConversationShareContract: { create: () => ({ identity: () => state.account }) },
    __elonChatGptPrivateRuntimeBindings: { state: () => ({ profile_id: 'web_20260912' }),
      load: async role => { assert.equal(role, 'shared'); return shared; }, peek: () => shared },
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({}) },
    __elonChatGptPrivateJsonRequest: { async request(_, url, init) {
      state.requests.push({ url, method: init.method });
      if (init.method === 'POST') {
        assert.equal(url, '/backend-api/conversation/message/writing-blocks');
        const body = JSON.parse(init.body); state.posts.push(body);
        state.payload.mapping[mid].message.metadata.writing_blocks[bid] = body.writing_block;
      } else assert.equal(url, '/backend-api/conversation/' + cid);
      state.afterRequest(init.method);
      return { payload: structuredClone(state.payload) };
    } } };
  const snapshot = () => ({ url: page.location.href, streaming: false, composerReady: false });
  const core = transport.create(page);
  const prepare = () => core.run({ operation: 'prepare', path, messageId: mid, id: bid, content: 'Original' }, false, snapshot);
  const save = ticket => core.run({ operation: 'save', path, ticket, content: 'Edited\n\n' }, true, snapshot);
  const verify = ticket => core.run({ operation: 'verify', path, ticket }, false, snapshot);
  return { state, page, shared, prepare, save, verify };
}

test('project route grammar is bounded and does not turn URLs or shared pages into writable conversations', () => {
  for (const path of [plainPath, projectPath, '/g/' + pid + plainPath]) {
    assert.deepEqual(policy.route(path), { id: cid, projectId: path === plainPath ? null : pid });
    assert.equal(policy.parse(JSON.stringify({ operation: 'prepare', path, messageId: mid, id: bid, content: '' })).path, path);
  }
  for (const path of ['/g/' + pid + '/project', '/g/' + pid + '/shared' + plainPath,
    projectPath + '?temporary-chat=true', projectPath + '#block', '/g/g-custom' + plainPath,
    '/g/' + pid + '-' + 'x'.repeat(125) + plainPath, 'https://chatgpt.com' + projectPath]) assert.equal(policy.route(path), null);
});

test('project write binds runtime and server scope, writes once, and updates native projection without composer', async () => {
  for (const path of [projectPath, plainPath]) {
    const h = harness(path), prepared = await h.prepare();
    assert.equal(prepared.code, 'writing_ready'); assert.equal(prepared.path, path);
    const saved = await h.save(prepared.ticket);
    assert.equal(saved.code, 'writing_saved'); assert.equal(saved.pending, false);
    assert.equal(h.state.posts.length, 1); assert.equal(h.state.updates, 1);
    assert.equal(h.state.posts[0].conversation_id, cid); assert.equal(h.state.posts[0].message_id, mid);
    assert.deepEqual(h.state.posts[0].writing_block.metadata, { keep: true });
    assert.equal(h.state.posts[0].gizmo_id, undefined);
    assert.equal(parser.runtimeProjection(h.page, mid).parts[0].textBlock.content, 'Edited\n\n');
    assert.equal((await h.save(prepared.ticket)).code, 'writing_saved'); assert.equal(h.state.posts.length, 1);
  }
});

test('project scope changes before save cannot write into the old or new project', async () => {
  for (const change of [h => { h.state.thread.projectId = other; }, h => { h.state.payload.gizmo_id = other; },
    h => { h.state.thread.projectId = null; }, h => { h.state.payload.gizmo_id = null; },
    h => { h.state.thread.sharedProjectConversationOwner = { id: 'other' }; },
    h => { h.state.thread.continuingFromSharedProjectConversationId = cid; },
    h => { h.state.payload.shared_project_conversation_owner = { id: 'other' }; },
    h => { h.state.thread.is_do_not_remember = true; }, h => { h.state.personal = false; },
    h => { h.state.user = 'user-b'; }, h => { h.state.account = 'identity-b'; },
    h => { h.state.leaf = 'other-branch'; }, h => { h.state.thread.contextScopes = ['other']; }]) {
    const h = harness(), prepared = await h.prepare(); assert.equal(prepared.code, 'writing_ready'); change(h);
    assert.equal((await h.save(prepared.ticket)).ok, false); assert.equal(h.state.posts.length, 0); assert.equal(h.state.updates, 0);
  }
});

test('project paths and local state alone are not sufficient authority for a write', async () => {
  for (const change of [h => { h.state.thread.projectId = other; }, h => { h.state.thread.projectId = null; },
    h => { h.state.thread.projectId = 'g-custom'; }, h => { h.state.personal = false; },
    h => { h.state.thread.isLoading = true; }, h => { h.state.payload.gizmo_id = null; },
    h => { delete h.shared.HM.getGizmoId; }, h => { h.page.location.href += '?temporary-chat=true'; },
    h => { h.state.thread.contextScopes = {}; }]) {
    const h = harness(); change(h); assert.equal((await h.prepare()).ok, false); assert.equal(h.state.posts.length, 0);
  }
});

test('project removal during the preflight read cancels before POST', async () => {
  const h = harness(), prepared = await h.prepare();
  h.state.afterRequest = method => { if (method === 'GET') h.state.thread.projectId = null; };
  assert.equal((await h.save(prepared.ticket)).ok, false); assert.equal(h.state.posts.length, 0);
});

test('project change after POST preserves uncertain write and never replays it', async () => {
  const h = harness(), prepared = await h.prepare();
  h.state.afterRequest = method => { if (method === 'POST') h.state.thread.projectId = other; };
  assert.equal((await h.save(prepared.ticket)).pending, true); assert.equal(h.state.updates, 0);
  assert.equal((await h.save(prepared.ticket)).pending, true); assert.equal(h.state.posts.length, 1);
  h.state.thread.projectId = pid;
  assert.equal((await h.verify(prepared.ticket)).code, 'writing_saved');
  assert.equal(h.state.posts.length, 1); assert.equal(h.state.updates, 1);
});

test('project changes inside the official update transaction prevent local mutation', async () => {
  const h = harness(), prepared = await h.prepare();
  h.state.beforeUpdate = () => { h.state.thread.projectId = other; };
  assert.equal((await h.save(prepared.ticket)).code, 'writing_saved_sync_pending');
  assert.equal(h.state.posts.length, 1); assert.equal(h.state.updates, 0);
});

test('project runtime projection refuses a different project without requesting or importing', () => {
  const h = harness(); h.shared.HM.getGizmoId = () => other;
  h.page.__elonChatGptPrivateRuntimeBindings.load = () => assert.fail('render cannot import');
  assert.equal(parser.runtimeProjection(h.page, mid), null); assert.equal(h.state.requests.length, 0);
});

'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const base = '../android/app/src/main/assets/';
const parser = require(base + 'chatgpt_web_text_blocks');
const policy = require(base + 'chatgpt_web_writing_block_policy');
const context = require(base + 'chatgpt_web_writing_block_context');
const transport = require(base + 'chatgpt_web_private_writing_blocks');
const stream = require(base + 'chatgpt_web_private_stream_policy');
const history = require(base + 'chatgpt_web_private_history_projection').create({ streamPolicy: stream });
const cid = '11111111-1111-4111-8111-111111111111', mid = 'message-a', bid = 'block-a';
const pid = 'g-p-' + 'a'.repeat(32), original = 'Original\r\n\r\n';
function message() {
  return { id: mid, author: { role: 'assistant' }, status: 'finished_successfully',
    content: { content_type: 'text', parts: [''] }, metadata: { writing_blocks: {}, content_references: [
      { type: 'citation', title: 'Unrelated' },
      { type: 'client_defined_widget', category: 'writing_block', data: { id: bid, title: 'Draft',
        variant: 'email', subject: 'Example', content: original, metadata: '{"tag":"keep"}' } }
    ] } };
}
const project = m => parser.project(m, true);
const card = m => project(m).parts[0].textBlock;
const data = m => m.metadata.content_references[1].data;

function harness(projectId = null) {
  const path = (projectId ? '/g/' + projectId : '') + '/c/' + cid;
  const thread = { isLoading: false, is_do_not_remember: false, projectId };
  const state = { account: 'identity-a', leaf: mid, thread, message: message(), posts: [], updates: 0,
    payload: { conversation_id: cid, gizmo_id: projectId, is_do_not_remember: false, current_node: mid,
      mapping: { [mid]: { id: mid, parent: null, message: message() } } }, afterRequest() {}, failPost: false };
  const owner = { id: 'client-a', serverId$: () => cid };
  const shared = { canvasConversations: () => [owner], XM: () => thread, Fl: () => false,
    H3: () => true, wV: () => true, SV: { isPersonalWorkspace() {} },
    mq: () => ({ isWorkspaceAccount: () => false, isQuorum: () => false, normalizedAccountUserId: 'user-a' }),
    HM: { getGizmoId: () => thread.projectId, getCurrentLeafId: () => state.leaf, getRequestId: () => null,
      getNodeIfExists: (_, id) => id === mid ? { message: state.message } : null },
    writingUpdateState(id, action) { assert.equal(id, owner.id); action(thread); },
    writingTreeOwner: { updateTree(_, action) { action({ containsNode: id => id === mid,
      getMaybeMessage: () => state.message,
      updateNodeMessageMetadata(id, value) { assert.equal(id, mid); state.updates++; Object.assign(state.message.metadata, value); }
    }); } } };
  const page = { document: {}, location: { href: 'https://chatgpt.com' + path }, setTimeout, clearTimeout,
    crypto: require('node:crypto').webcrypto, __elonChatGptDocumentToken: 'doc_widget_test',
    __elonChatGptTextBlocks: parser, __elonChatGptWritingBlockPolicy: policy, __elonChatGptWritingBlockContext: context,
    __elonChatGptPrivateConversationMutationsEnabled: true,
    __elonChatGptPrivateConversationShareContract: { create: () => ({ identity: () => state.account }) },
    __elonChatGptPrivateRuntimeBindings: { state: () => ({ profile_id: 'web_20260912' }), load: async () => shared, peek: () => shared },
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({}) },
    __elonChatGptPrivateJsonRequest: { async request(_, url, init) {
      if (init.method === 'POST') {
        assert.equal(url, '/backend-api/conversation/message/writing-blocks');
        const body = JSON.parse(init.body); state.posts.push(body);
        state.payload.mapping[mid].message.metadata.writing_blocks[bid] = body.writing_block;
        if (state.failPost) throw Error('timeout');
      } else assert.equal(url, '/backend-api/conversation/' + cid);
      state.afterRequest(init.method);
      return { payload: structuredClone(state.payload) };
    } } };
  const snapshot = () => ({ url: page.location.href, streaming: false, composerReady: false });
  const core = transport.create(page);
  return { state, page, prepare: () => core.run({ operation: 'prepare', path, messageId: mid, id: bid, content: original }, false, snapshot),
    save: (ticket, content) => core.run({ operation: 'save', path, ticket, content }, true, snapshot),
    verify: ticket => core.run({ operation: 'verify', path, ticket }, false, snapshot) };
}

test('typed widget uses saved content, including empty content, without modifying the original reference', () => {
  for (const content of ['New\r\n\r\n', '']) {
    const m = message(); m.metadata.writing_blocks[bid] = { content, title: '' };
    const before = JSON.stringify(m), value = card(m);
    assert.equal(value.content, content); assert.equal(value.title, '');
    assert.equal(JSON.stringify(m), before); assert.equal(data(m).content, original);
    const frame = stream.assistantFrame({ message: m });
    assert.equal(frame.blockParts[0].textBlock.content, content);
    assert.equal(history.project({ messages: [m] })[0].content[0].textBlock.content, content);
  }
});

test('widget ownership uses original reference index and preserves email and string metadata', () => {
  const m = message(); data(m).recipient = 'example.invalid'; data(m).metadata = encodeURIComponent('{"tag":"keep","subject":"override"}');
  const source = project(m).writeSources[0];
  assert.equal(source.index, 1); assert.equal(source.messageId, mid); assert.equal(source.representation, 'widget');
  assert.deepEqual(source.metadata, { recipient: 'example.invalid', cc: null, bcc: null, subject: 'override', tag: 'keep' });
  assert.equal(card(m).sourceMessageId, mid);
  assert.equal(parser.project(m).writeSources, undefined);
});

test('unknown or ambiguous widget representations retain local editing but cannot authorize cloud writes', () => {
  for (const mutate of [
    m => { delete data(m).id; }, m => { data(m).id = '../file'; },
    m => { data(m).variant = 'unknown'; }, m => { data(m).metadata = '{"nested":{}}'; },
    m => { data(m).metadata = 'bad'; }, m => { data(m).metadata = { tag: 'unknown encoding' }; },
    m => { data(m).library_file_id = 'libfile_a'; },
    m => { m.metadata.writing_blocks[bid] = { library_file_id: 'libfile_a' }; },
    m => { m.metadata.writing_blocks[bid] = { id: 'other' }; },
    m => { m.metadata.writing_blocks[bid] = { index: 0 }; },
    m => { m.metadata.writing_blocks[bid] = { metadata: [] }; },
    m => { m.clientMetadata = { writingBlockOwners: {} }; },
    m => { m.author.role = 'user'; },
    m => { m.metadata.content_references.push(structuredClone(m.metadata.content_references[1])); },
    m => { m.content.parts = [':::writing{id="block-a" variant="email"}\nOther\n:::']; },
    m => { m.metadata.content_references.push(...Array(64).fill({ type: 'citation' })); }
  ]) {
    const m = message(); mutate(m);
    const value = project(m); assert.ok(value.parts.length > 0);
    assert.equal(value.writeSources.length, 0); assert.ok(value.parts.every(p => !p.textBlock.sourceMessageId));
  }
});

test('unfinished widgets stay read-only and inherited metadata is never used', () => {
  const m = message(); m.status = 'in_progress';
  assert.equal(card(m).complete, false); assert.equal(card(m).sourceMessageId, undefined);
  m.status = 'finished_successfully'; m.metadata.writing_blocks = Object.create({ [bid]: { content: 'Other' } });
  assert.equal(card(m).content, original);
});

test('widget source does not turn malformed or oversized metadata into a save payload', () => {
  for (const mutate of [m => { data(m).title = 'x'.repeat(513); }, m => { data(m).subject = {}; },
    m => { data(m).metadata = JSON.stringify({ x: 'x'.repeat(16384) }); }]) {
    const m = message(); mutate(m); assert.equal(project(m).writeSources.length, 0);
  }
});

test('actual parser/context/transport saves widgets in ordinary and owned project conversations without DOM readiness', async () => {
  for (const projectId of [null, pid]) for (const content of ['Edited\n\n', '']) {
    const h = harness(projectId), p = await h.prepare(); assert.equal(p.code, 'writing_ready');
    const result = await h.save(p.ticket, content); assert.equal(result.code, 'writing_saved');
    assert.equal(h.state.posts.length, 1); assert.equal(h.state.updates, 1);
    const body = h.state.posts[0];
    assert.equal(body.index, '1'); assert.equal(body.writing_block.index, '1');
    assert.equal(body.writing_block.content, content); assert.equal(body.message_id, mid); assert.equal(body.conversation_id, cid);
    assert.deepEqual(body.writing_block.metadata, { recipient: null, cc: null, bcc: null, subject: 'Example', tag: 'keep' });
    assert.equal(body.writing_block.representation, undefined);
    assert.equal(parser.runtimeProjection(h.page, mid).parts[0].textBlock.content, content);
    assert.equal(data(h.state.message).content, original);
    assert.equal((await h.save(p.ticket, content)).code, 'writing_saved'); assert.equal(h.state.posts.length, 1);
  }
});

test('changed reference order, branch, scope, identity and local draft prevent dispatch', async () => {
  for (const change of [
    h => { h.state.payload.mapping[mid].message.metadata.content_references.unshift({ type: 'citation' }); },
    h => { h.state.message.metadata.content_references.unshift({ type: 'citation' }); },
    h => { h.state.message.metadata.writing_blocks[bid] = { content: 'Local edit', locallyEdited: true }; },
    h => { h.state.leaf = 'other'; }, h => { h.state.account = 'other'; },
    h => { h.state.payload.is_do_not_remember = true; },
    h => { h.state.payload.shared_project_conversation_owner = { id: 'other' }; }
  ]) {
    const h = harness(), p = await h.prepare(); assert.equal(p.code, 'writing_ready'); change(h);
    assert.equal((await h.save(p.ticket, 'Edited')).ok, false); assert.equal(h.state.posts.length, 0);
  }
});

test('uncertain widget POST can be verified but is never replayed', async () => {
  const h = harness(), p = await h.prepare(); h.state.failPost = true;
  assert.equal((await h.save(p.ticket, 'Saved')).pending, true);
  assert.equal((await h.save(p.ticket, 'Different')).pending, true); assert.equal(h.state.posts.length, 1);
  assert.equal((await h.verify(p.ticket)).code, 'writing_saved'); assert.equal(h.state.updates, 1);
  assert.equal(card(h.state.message).content, 'Saved'); assert.equal(h.state.posts.length, 1);
});

test('catalog bounds never expose an unverified duplicate as a cloud-save target', () => {
  const m = message();
  m.metadata.content_references = Array.from({ length: 17 }, (_, i) => ({ type: 'client_defined_widget', category: 'writing_block',
    data: { id: 'block-' + i, content: 'Original', variant: 'standard' } }));
  const value = project(m); assert.equal(value.parts.length, 16); assert.equal(value.writeSources.length, 0);
});

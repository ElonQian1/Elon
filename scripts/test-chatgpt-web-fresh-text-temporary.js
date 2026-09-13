'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture, CID, PID } = require('./fixtures/chatgpt-fresh-text-context');
const assets = '../android/app/src/main/assets/';
const requestModule = require(assets + 'chatgpt_web_fresh_text_request');
const historyModule = require(assets + 'chatgpt_web_fresh_text_reconcile');
const transactionModule = require(assets + 'chatgpt_web_fresh_text_transaction');
const policy = require(assets + 'chatgpt_web_private_stream_policy');
const ROOT = 'client-created-root', URL = 'https://chatgpt.com/?temporary-chat=true';
const UID = '33333333-3333-4333-8333-333333333333', AID = '44444444-4444-4444-8444-444444444444';
const UID2 = '55555555-5555-4555-8555-555555555555', AID2 = '66666666-6666-4666-8666-666666666666';
const admission = { allowTemporary: true, allowNewConversations: true };
const input = { id: UID, author: { role: 'user' }, content: { content_type: 'text', parts: ['Synthetic question'] } };
const answer = { id: AID, author: { role: 'assistant' }, status: 'finished_successfully', end_turn: true,
  content: { content_type: 'text', parts: ['Synthetic answer'] } };

function temporaryFixture(isNew = true) {
  const f = fixture();
  f.serverId = isNew ? null : CID; f.key = 'temporary-entry'; f.binds = 0; f.personalization = false; f.gate = true;
  f.selected.id = 'WEB:temporary-owner'; f.selected.serverId$ = () => f.serverId;
  f.page.location.href = f.binding.href = URL;
  f.binding.temporary = true; f.binding.newThread = f.props.isNewThread = isNew;
  f.binding.serverId = f.serverId; f.tree.isLoading = false; f.tree.is_do_not_remember = false;
  if (isNew) Object.assign(f.parent, { id: ROOT, author: { role: 'root' }, content: { content_type: 'text', parts: [] } });
  f.leaf = f.parent; f.props.currentLeafId = f.parent.id; f.messages = [f.parent];
  Object.assign(f.shared.HM, { getCurrentMessage: () => f.leaf, getIsNewConversation: () => f.leaf.id === ROOT,
    getAllMessages: () => f.messages, getNodeIfExists: (_, id) => f.nodes?.[id],
    getParentNode: (_, id) => ({ id: f.nodes?.[id]?.parent }),
    getParentPromptNode: (_, id) => ({ id: f.nodes?.[id]?.parent }) });
  Object.assign(f.shared, {
    cX: () => true, textHistoryDisabled: () => true, textNavigationKey: () => f.key,
    textClientConversation: () => true, textResolvedConversationId: () => f.serverId,
    textBindConversationId(id, server) { assert.equal(id, f.selected.id); f.serverId = server; f.binds++; },
    textTemporaryPersonalizationEnabled: () => f.gate,
    textTemporaryPersonalization: id => { assert.equal(id, f.selected.id); return f.personalization; },
    textReadUntracked: read => read()
  });
  Object.assign(f.conversation, { textRequestedDefaultModel: () => undefined, textRememberFirstModel() {} });
  f.page.setTimeout = setTimeout; f.page.clearTimeout = clearTimeout;
  return f;
}

function history(parent = ROOT) {
  return { conversation_id: CID, gizmo_id: null, is_do_not_remember: true, is_temporary_chat: true,
    async_status: null, current_node: AID, mapping: {
      [parent]: { id: parent, parent: null, children: [UID], message: null },
      [UID]: { id: UID, parent, message: input }, [AID]: { id: AID, parent: UID, message: answer }
    } };
}

function apply(f, value) {
  f.nodes = value.mapping; f.leaf = f.nodes[value.current_node].message;
  f.messages = Object.values(f.nodes).map(row => row.message).filter(Boolean);
  f.tree.is_do_not_remember = value.is_do_not_remember;
  f.props.currentLeafId = f.leaf.id; f.props.isNewThread = f.binding.newThread = false;
  f.binding.serverId = f.serverId;
}

test('temporary ownership requires explicit scope, committed thread and actual history-disabled state', async () => {
  for (const isNew of [true, false]) {
    const f = temporaryFixture(isNew);
    await assert.rejects(f.api.capture(f.node, null, { allowNewConversations: true }), /scope_unsupported/);
    const binding = await f.api.capture(f.node, null, admission);
    assert.equal(binding.temporary, true); assert.equal(binding.newConversation, isNew);
    assert.equal(binding.historyDisabled, true);
    assert.equal(binding.doNotRemember, false, 'legacy memory field is not the selected temporary state');
    assert.equal(binding.parentId, isNew ? ROOT : PID);
    f.node.isConnected = false; assert.equal(binding.current(), true);
    f.key = 'same-home-different-entry'; assert.equal(binding.owns(), false);
  }
  for (const change of [f => { f.shared.cX = () => false; }, f => { f.shared.textHistoryDisabled = () => false; },
    f => { f.binding.temporary = false; }, f => { delete f.tree.is_do_not_remember; },
    f => { f.files.files$ = () => [{}]; }, f => { f.props.conversation = {}; },
    f => { f.binding.newThread = false; }, f => { f.shared.canvasConversations = () => []; },
    f => { f.shared.HM.getGizmoId = () => 'g-p-' + 'a'.repeat(32); },
    f => { f.page.location.href = f.binding.href = 'https://chatgpt.com/c/' + CID + '?temporary-chat=true'; }]) {
    const f = temporaryFixture(); change(f); await assert.rejects(f.api.capture(f.node, null, admission));
    assert.equal(f.binds, 0);
  }
});

test('first temporary prepare/dispatch preserve personalization false, true and an absent official gate', async () => {
  for (const isNew of [true, false]) for (const gate of [false, true]) for (const selected of [false, true]) {
    const f = temporaryFixture(isNew); f.gate = gate; f.personalization = selected;
    f.page.crypto = { randomUUID: () => UID };
    const binding = await f.api.capture(f.node, null, admission);
    const req = requestModule.create(f.page).create(binding, { requestId: 'mcp_1', prompt: 'Synthetic question' });
    const sent = req.consume({ conduit_token: 'synthetic' }, { chatReq: { token: 'synthetic' } },
      () => ({ 'openai-sentinel-chat-requirements-token': 'synthetic' }), binding.current);
    for (const body of [req.preparationBody(), sent.body]) {
      assert.equal(body.history_and_training_disabled, true);
      assert.equal('is_do_not_remember' in body, false);
      assert.equal(body.conversation_id, isNew ? undefined : CID);
      assert.equal(body.temporary_chat_requests_personalization, isNew && gate ? selected : undefined);
    }
    if (isNew && gate) { f.personalization = !selected; assert.equal(binding.current(), false); }
  }
  for (const change of [f => { f.gate = 'unknown'; }, f => { f.personalization = null; },
    f => { delete f.shared.textReadUntracked; }]) {
    const f = temporaryFixture(); change(f); await assert.rejects(f.api.capture(f.node, null, admission));
  }
});

test('a temporary memory flag is forwarded separately and invalid privacy bodies cannot be dispatched', async () => {
  const f = temporaryFixture(); f.tree.is_do_not_remember = true;
  f.page.crypto = { randomUUID: () => UID };
  const binding = await f.api.capture(f.node, null, admission), requests = requestModule.create(f.page);
  assert.equal(requests.create(binding, { requestId: 'mcp_1', prompt: 'Synthetic' }).preparationBody().is_do_not_remember, true);
  for (const patch of [{ historyDisabled: false }, { temporary: false },
    { projectId: 'g-p-' + 'a'.repeat(32) }, { temporaryPersonalization: 'true' }]) {
    assert.throws(() => requests.create({ ...binding, ...patch }, { requestId: 'mcp_2', prompt: 'Synthetic' }), /context_invalid/);
  }
});

test('temporary requests lose ownership on privacy, account or committed-thread changes', async () => {
  for (const adopted of [false, true]) for (const change of [
    f => { f.shared.textHistoryDisabled = () => false; },
    f => { f.shared.cX = () => false; },
    f => { f.identity('another-fixture-account'); },
    f => { f.props.conversation = {}; },
    f => { f.key = 'another-temporary-entry'; },
    f => { f.page.location.href = 'https://chatgpt.com/'; },
  ]) {
    const f = temporaryFixture(), binding = await f.api.capture(f.node, null, admission);
    if (adopted) assert.equal(binding.adoptConversation(CID, { input_message: input }, UID), true);
    change(f);
    assert.equal(binding.owns(), false);
    assert.equal(binding.current(), false);
    assert.throws(binding.beforeDispatch, /context_changed/);
    assert.equal(binding.canReconcile(UID), false);
    assert.equal(await binding.finalize(new AbortController().signal), false);
  }
});

test('temporary identity cannot change while official bindings are loading', async () => {
  const f = temporaryFixture(), load = f.page.__elonChatGptPrivateRuntimeBindings.load;
  f.page.__elonChatGptPrivateRuntimeBindings.load = async role => {
    const value = await load(role);
    f.identity('another-fixture-account');
    return value;
  };
  await assert.rejects(f.api.capture(f.node, null, admission), /context_changed/);
  assert.equal(f.binds, 0);
});

test('temporary completion requires private history, preserves its route and rejects owner changes', async () => {
  for (const isNew of [true, false]) {
    const f = temporaryFixture(isNew), binding = await f.api.capture(f.node, null, admission);
    if (isNew) assert.equal(binding.adoptConversation(CID, { input_message: input }, UID), true);
    const data = history(isNew ? ROOT : PID), api = historyModule.create();
    assert.equal(api.ownsResponse(data, binding, UID), true);
    for (const patch of [{ is_do_not_remember: false }, { is_temporary_chat: false },
      { gizmo_id: 'g-p-' + 'a'.repeat(32) }, { shared_project_conversation_owner: {} }]) {
      assert.equal(api.ownsResponse({ ...data, ...patch }, binding, UID), false);
    }
    f.conversation.textHydrateHistory = async (_, options) => {
      options.onConversationLoadedFromNetwork(data); assert.equal(options.shouldApplyResponse(), true); apply(f, data);
    };
    assert.equal(await api.reconcile(binding, { userMessageId: UID }, new AbortController().signal), true);
    assert.equal(f.page.location.href, URL); assert.equal(binding.owns(), true);
    f.shared.cX = () => false; assert.equal(binding.owns(), false);
  }
});

async function integratedTemporary(stop = false) {
  const f = temporaryFixture(), page = f.page, counts = { prepare: 0, posts: 0, apply: 0, stop: 0 };
  let sequence = 0, draft = '', release;
  const pause = new Promise(resolve => { release = resolve; });
  Object.assign(page, { AbortController, crypto: { randomUUID: () => [UID, AID, UID2, AID2][sequence++] },
    __elonChatGptPrivateTextTransactionsEnabled: true, __elonChatGptFreshTextNewConversationsEnabled: true,
    __elonChatGptFreshTextTemporaryEnabled: true, __elonChatGptFreshTextRequest: requestModule,
    __elonChatGptFreshTextReconcile: historyModule,
    __elonChatGptFreshTextReceipts: require(assets + 'chatgpt_web_fresh_text_receipts'),
    __elonChatGptFreshTextRecovery: require(assets + 'chatgpt_web_fresh_text_recovery'),
    __elonChatGptFreshTextStop: require(assets + 'chatgpt_web_fresh_text_stop'),
    __elonChatGptFreshTextStream: require(assets + 'chatgpt_web_fresh_text_stream') });
  const session = policy.createSession({ now: Date.now });
  const owned = require(assets + 'chatgpt_web_private_owned_stream').create({ policy, session,
    conversationId: value => value.conversation_id || '', notify() {} });
  page.__elonChatGptPrivateStreamTransport = { preparePrivateSend() { session.reset(); return true; },
    beginPrivateStream: owned.begin, finishPrivateSend() { session.finish(); owned.reset(); } };
  f.shared.textTopic = () => { throw Error('SSE-only fixture'); }; f.shared.$3 = () => true;
  f.shared.textApi.safePost = async (path, options) => {
    if (path === '/stop_conversation') { counts.stop++; assert.equal(options.requestBody.conversation_id, CID); return {}; }
    counts.prepare++; assert.equal(path, '/f/conversation/prepare');
    assert.equal(options.requestBody.history_and_training_disabled, true);
    assert.equal(options.requestBody.temporary_chat_requests_personalization, counts.prepare === 1 ? false : undefined);
    return { conduit_token: 'synthetic' };
  };
  f.shared.textSecurityHeaders = () => ({ 'openai-sentinel-chat-requirements-token': 'synthetic' });
  f.conversation.textSecurity = () => ({ chatReq: { token: 'synthetic' } });
  f.conversation.textStream = async function* (_, options) {
    options.onBeforeRequestStart(); counts.posts++;
    const first = counts.posts === 1, user = { ...input, id: first ? UID : UID2 }, reply = { ...answer, id: first ? AID : AID2 };
    assert.equal(options.body.history_and_training_disabled, true);
    assert.equal(options.body.conversation_id, first ? undefined : CID);
    assert.equal(options.body.parent_message_id, first ? ROOT : AID);
    assert.equal(options.body.messages[0].id, user.id);
    yield { response: { ok: true, headers: { get: () => 'text/event-stream' } } };
    yield { data: { conversation_id: CID, input_message: user } };
    yield { data: { conversation_id: CID, message: { ...reply, status: 'in_progress', end_turn: false } } };
    if (stop) await pause;
    yield { data: { conversation_id: CID, message: reply } };
  };
  f.conversation.textHydrateHistory = async (_, options) => {
    const data = history();
    if (counts.posts === 2) {
      data.mapping[UID2] = { id: UID2, parent: AID, message: { ...input, id: UID2 } };
      data.mapping[AID2] = { id: AID2, parent: UID2, message: { ...answer, id: AID2 } };
      data.current_node = AID2;
    }
    if (stop && !counts.stop) data.async_status = 3;
    if (counts.stop) data.mapping[AID].message = { ...answer, status: 'finished_partial_completion' };
    options.onConversationLoadedFromNetwork(data);
    if (options.shouldApplyResponse()) { apply(f, data); counts.apply++; }
  };
  const api = transactionModule.create(page, { context: f.api });
  const command = { requestId: 'mcp_1', prompt: 'Synthetic question', expectedDraft: '', composer: f.node,
    readDraft: () => draft, onDispatch() {}, onSettled() {} };
  const sent = api.send(command); assert.equal((await sent.completion).status, 'accepted');
  return { f, api, command, session, counts, release, setDraft: value => { draft = value; }, readDraft: () => draft };
}
const tick = () => new Promise(resolve => setTimeout(resolve, 5));
async function settled(api) { for (let i = 0; i < 80 && api.state().pending; i++) await tick(); }

test('first and continuing temporary private sends stream and reconcile without saved-chat navigation', async () => {
  const r = await integratedTemporary(); r.setDraft('Later unsent draft'); await settled(r.api);
  assert.equal(r.api.state().pending, false); assert.equal(r.readDraft(), 'Later unsent draft');
  assert.equal(r.f.page.location.href, URL); assert.equal(r.session.current('/').text, 'Synthetic answer');
  assert.equal(r.f.tree.is_do_not_remember, true); assert.equal(r.f.binds, 1);
  const previous = r.api.send(r.command); assert.equal(previous.handled, true); assert.equal(r.counts.posts, 1);
  r.setDraft('');
  assert.equal((await r.api.send({ ...r.command, requestId: 'mcp_2' }).completion).status, 'accepted');
  await settled(r.api); assert.equal(r.api.state().pending, false);
  assert.deepEqual(r.counts, { prepare: 2, posts: 2, apply: 2, stop: 0 });
  assert.equal(r.f.page.location.href, URL); assert.equal(r.f.binds, 1); assert.equal(r.f.leaf.id, AID2);
  assert.equal(r.api.dispose(), true);
});

test('temporary stop consumes only its acquired server ownership and remains in the same temporary chat', async () => {
  const r = await integratedTemporary(true);
  for (let i = 0; i < 30 && !r.f.serverId; i++) await tick();
  assert.equal((await r.api.stop().completion).status, 'accepted');
  assert.equal(r.counts.stop, 1); assert.equal(r.counts.posts, 1); assert.equal(r.f.page.location.href, URL);
  assert.equal(r.session.current('/').text, 'Synthetic answer');
  r.release(); await settled(r.api); assert.equal(r.api.dispose(), true);
});

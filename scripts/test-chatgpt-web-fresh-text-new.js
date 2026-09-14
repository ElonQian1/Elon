'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture, CID } = require('./fixtures/chatgpt-fresh-text-context');
const assets = '../android/app/src/main/assets/';
const requestModule = require(assets + 'chatgpt_web_fresh_text_request');
const historyModule = require(assets + 'chatgpt_web_fresh_text_reconcile');
const transactionModule = require(assets + 'chatgpt_web_fresh_text_transaction');
const policy = require(assets + 'chatgpt_web_private_stream_policy');
const ROOT = 'client-created-root';
const UID = '33333333-3333-4333-8333-333333333333';
const AID = '44444444-4444-4444-8444-444444444444';
const UID2 = '55555555-5555-4555-8555-555555555555';
const AID2 = '66666666-6666-4666-8666-666666666666';
const PROJECT = 'g-p-' + 'a'.repeat(32);
const admission = { allowNewConversations: true, allowProjects: true, allowTools: true };
const input = { id: UID, author: { role: 'user' }, content: { content_type: 'text', parts: ['Synthetic first turn'] } };
const response = { id: AID, author: { role: 'assistant' }, status: 'finished_successfully', end_turn: true,
  content: { content_type: 'text', parts: ['Synthetic answer'] } };

function newFixture(project = false) {
  const f = fixture();
  f.serverId = null; f.key = 'initial-key'; f.binds = 0; f.navigations = 0; f.remembered = [];
  f.canonicalRootId = ROOT;
  f.selected.id = 'WEB:synthetic-owner'; f.selected.serverId$ = () => f.serverId;
  f.binding.serverId = null; f.binding.newThread = true; f.props.isNewThread = true;
  f.page.location.href = f.binding.href = 'https://chatgpt.com' + (project ? '/g/' + PROJECT + '/project' : '/');
  f.parent.id = ROOT; f.parent.author = { role: 'root' }; f.parent.content = { content_type: 'text', parts: [] };
  f.props.currentLeafId = ROOT; f.leaf = f.parent;
  f.tree.isLoading = false; f.tree.is_do_not_remember = false;
  f.messages = [f.parent];
  Object.assign(f.shared.HM, { getCurrentMessage: () => f.leaf, getIsNewConversation: () => f.leaf.id === ROOT,
    getAllMessages: () => f.messages, getNodeIfExists: (_, id) => f.nodes?.[id],
    getParentNode: (_, id) => f.nodes?.[f.nodes?.[id]?.parentId],
    getParentPromptNode: (_, id) => f.nodes?.[id]?.message?.author?.role === 'assistant'
      ? { id: f.nodes[id].parent } : null });
  Object.assign(f.shared, {
    textClientConversation: id => id.startsWith('WEB:'), textNavigationKey: () => f.key,
    textResolvedConversationId: () => f.serverId,
    textBindConversationId(id, value) { assert.equal(id, f.selected.id); f.binds++; f.serverId = value; },
    canvasQueryClient: () => 'synthetic-client',
    textNavigate(path, options) {
      assert.equal(options.replace, true);
      f.navigations++; f.key = 'server-key'; f.page.location.href = 'https://chatgpt.com' + path;
    }
  });
  Object.assign(f.conversation, {
    textRequestedDefaultModel: () => 'fixture-default',
    textRememberFirstModel: (owner, model) => { assert.equal(owner, f.selected); f.remembered.push(model); },
    textNavigateConversation(navigate, client, id, isProject, unused, replace) {
      assert.equal(typeof navigate, 'function'); assert.equal(client, 'synthetic-client');
      assert.equal(id, CID); assert.equal(isProject, project); assert.equal(replace, true);
      navigate((project ? '/g/' + PROJECT + '-synthetic' : '') + '/c/' + id, { replace });
    }
  });
  if (project) {
    f.tree.mode = { kind: 'gizmo_interaction', gizmo_id: PROJECT };
    f.shared.HM.getGizmoId = state => state.mode.gizmo_id;
    f.shared.HM.getConversationTurns = () => [];
    Object.assign(f.shared, { textBusinessContext: () => null, textProjectHeaders: () => undefined,
      textLockedProjectId: () => null, textLockedChatPin: () => undefined });
  }
  f.page.setTimeout = setTimeout; f.page.clearTimeout = clearTimeout;
  return f;
}

function payload(project = false, emptyRoot = false, parentId = ROOT) {
  const root = emptyRoot ? '' : parentId;
  return { conversation_id: CID, gizmo_id: project ? PROJECT : null, is_do_not_remember: false,
    async_status: null, current_node: AID, mapping: {
      [root]: { id: root, parent: null, children: [UID], message: null },
      [UID]: { id: UID, parent: root, message: input },
      [AID]: { id: AID, parent: UID, message: response }
    } };
}

function applyHistory(f, data, retainPaginationRoot = true) {
  f.nodes = structuredClone(data.mapping);
  // The reviewed paginated reducer updates an existing Cx tree in place using
  // its current rootId; the response's synthetic pagination root is not added.
  if (data.__paginatedConversationPage && retainPaginationRoot) {
    const serverRoot = 'paginated-root:' + CID, root = f.nodes[serverRoot];
    delete f.nodes[serverRoot];
    f.nodes[f.canonicalRootId] = { ...root, id: f.canonicalRootId };
    for (const row of Object.values(f.nodes)) {
      if (row.parent === serverRoot) row.parent = f.canonicalRootId;
    }
  } else {
    f.canonicalRootId = Object.values(f.nodes).find(row => !row.parent).id || ROOT;
  }
  if (f.nodes['']) {
    f.nodes[ROOT] = { ...f.nodes[''], id: ROOT };
    delete f.nodes[''];
    f.nodes[UID].parent = ROOT;
  }
  for (const row of Object.values(f.nodes)) {
    row.parentId = row.parent ?? '';
    row.message ??= { id: row.id, author: { role: 'root' }, content: { content_type: 'text', parts: [] } };
  }
  f.leaf = f.nodes[data.current_node].message;
  f.messages = Object.values(f.nodes).map(row => row.message).filter(Boolean);
  f.props.currentLeafId = f.leaf.id; f.props.isNewThread = f.binding.newThread = false;
  f.binding.serverId = f.serverId;
}

function injectedHistory(project = false) {
  const value = payload(project), systemId = '77777777-7777-4777-8777-777777777777';
  delete value.mapping[ROOT];
  value.mapping['synthetic-server-root'] = { id: 'synthetic-server-root', parent: '', children: [systemId], message: null };
  value.mapping[systemId] = { id: systemId, parent: 'synthetic-server-root', children: [UID], message: {
    id: systemId, author: { role: 'system' }, content: { content_type: 'text', parts: ['Synthetic context'] },
    metadata: { is_visually_hidden_from_conversation: true }
  } };
  value.mapping[UID].parent = systemId;
  return value;
}

function paginatedHistory(project = false, hidden = true) {
  const value = hidden ? injectedHistory(project) : payload(project), rootId = 'paginated-root:' + CID;
  const oldRoot = hidden ? 'synthetic-server-root' : ROOT, root = value.mapping[oldRoot];
  delete value.mapping[oldRoot];
  value.mapping[rootId] = { ...root, id: rootId, parent: '' };
  value.mapping[root.children[0]].parent = rootId;
  const messages = []; let id = value.current_node;
  while (id !== rootId) { messages.push(value.mapping[id].message); id = value.mapping[id].parent; }
  messages.forEach((message, index) => { value.mapping[message.id].children = index ? [messages[index - 1].id] : []; });
  value.__paginatedConversationPage = { cursor: null, oldestMessageId: messages.at(-1).id,
    messagesLeafToRoot: messages, serverCurrentLeafId: value.current_node };
  return value;
}

test('new first-turn routing settles before history hydration can retire the empty-home composer', async () => {
  for (const project of [false, true]) {
    const f = newFixture(project), binding = await f.api.capture(f.node, null, admission);
    binding.adoptConversation(CID, { input_message: input }, UID);
    const value = paginatedHistory(project), history = historyModule.create();
    let reads = 0;
    f.conversation.textHydrateHistory = async (_, options) => {
      reads++;
      assert.equal(binding.navigationReady(), true, 'owned route must settle before hydration');
      assert.equal(f.navigations, 1);
      options.onConversationLoadedFromNetwork(value);
      assert.equal(options.shouldApplyResponse(), true);
      applyHistory(f, value);
    };
    assert.equal(await history.reconcile(binding, { userMessageId: UID }, new AbortController().signal), true);
    assert.equal(reads, 1); assert.equal(f.navigations, 1);
  }
});

test('failed or cancelled first-turn navigation never starts the history loader', async () => {
  for (const cancel of [false, true]) {
    const f = newFixture(), binding = await f.api.capture(f.node, null, admission);
    binding.adoptConversation(CID, { input_message: input }, UID);
    const controller = new AbortController(); let reads = 0;
    f.conversation.textNavigateConversation = () => {
      if (cancel) controller.abort();
      else f.page.location.href = 'https://chatgpt.com/c/' + AID;
    };
    f.conversation.textHydrateHistory = async () => { reads++; };
    assert.equal(await historyModule.create().reconcile(binding, { userMessageId: UID }, controller.signal), false);
    assert.equal(reads, 0);
  }
});

test('complete provider pagination roots reconcile only their exact conversation and full message chain', async () => {
  for (const hidden of [false, true]) for (const retained of [false, true]) {
    const f = newFixture(), binding = await f.api.capture(f.node, null, admission);
    binding.adoptConversation(CID, { input_message: input }, UID);
    const value = paginatedHistory(false, hidden), history = historyModule.create();
    assert.equal(history.ownsResponse(value, binding, UID), true);
    f.conversation.textHydrateHistory = async (_, options) => {
      options.onConversationLoadedFromNetwork(value);
      assert.equal(options.shouldApplyResponse(), true); applyHistory(f, value, retained);
    };
    assert.equal(await history.reconcile(binding, { userMessageId: UID }, new AbortController().signal), true);
    for (const mutate of [
      p => { delete p.__paginatedConversationPage; },
      p => { p.__paginatedConversationPage.cursor = 'synthetic-older-page'; },
      p => { delete p.__paginatedConversationPage.cursor; },
      p => { p.__paginatedConversationPage.oldestMessageId = AID; },
      p => { p.__paginatedConversationPage.serverCurrentLeafId = UID; },
      p => { p.__paginatedConversationPage.messagesLeafToRoot.reverse(); },
      p => { p.__paginatedConversationPage.messagesLeafToRoot.push(input); },
      p => { p.__paginatedConversationPage.messagesLeafToRoot = []; },
      p => { p.mapping[UID].parent = 'paginated-root:' + AID; }
    ]) {
      const changed = paginatedHistory(false, hidden); mutate(changed);
      assert.equal(history.ownsResponse(changed, binding, UID), false);
    }
  }
});

test('retained pagination roots do not authorize changed canonical identities or branches', async () => {
  for (const mutate of [
    f => { delete f.nodes[ROOT]; },
    f => { f.nodes[ROOT].id = AID; },
    f => { f.nodes[ROOT].message.id = AID; },
    f => { f.nodes[ROOT].message.author.role = 'assistant'; },
    f => { f.nodes[ROOT].parentId = AID; },
    f => { f.nodes[ROOT].children.push(AID); },
    f => { f.nodes[ROOT].children = [UID]; },
    f => { f.nodes[f.nodes[UID].parentId].parentId = AID; },
    f => { f.nodes[f.nodes[UID].parentId].message.metadata.is_visually_hidden_from_conversation = false; },
    f => { f.nodes['paginated-root:' + CID] = { ...f.nodes[ROOT], id: 'paginated-root:' + CID }; },
    f => {
      f.nodes.otherRoot = { ...f.nodes[ROOT], id: 'otherRoot', message: { ...f.nodes[ROOT].message, id: 'otherRoot' } };
      f.nodes[f.nodes[UID].parentId].parentId = 'otherRoot'; delete f.nodes[ROOT];
    }
  ]) {
    const f = newFixture(), binding = await f.api.capture(f.node, null, admission);
    binding.adoptConversation(CID, { input_message: input }, UID);
    f.conversation.textHydrateHistory = async (_, options) => {
      const value = paginatedHistory();
      options.onConversationLoadedFromNetwork(value);
      assert.equal(options.shouldApplyResponse(), true);
      applyHistory(f, value); mutate(f);
    };
    assert.equal(await historyModule.create().reconcile(binding, { userMessageId: UID }, new AbortController().signal), false);
    assert.equal(binding.reconciliationFailure(), 'store_parent_mismatch');
    assert.equal(f.navigations, 1);
  }
});

test('an owned first turn reconciles through the observed hidden system ancestor and exact canonical root', async () => {
  for (const project of [false, true]) {
    const f = newFixture(project), binding = await f.api.capture(f.node, null, admission);
    binding.adoptConversation(CID, { input_message: input }, UID);
    const value = injectedHistory(project), history = historyModule.create();
    assert.equal(history.ownsResponse(value, binding, UID), true);
    f.conversation.textHydrateHistory = async (_, options) => {
      options.onConversationLoadedFromNetwork(value);
      assert.equal(options.shouldApplyResponse(), true);
      applyHistory(f, value);
    };
    assert.equal(await history.reconcile(binding, { userMessageId: UID }, new AbortController().signal), true);
    assert.equal(f.navigations, 1);
    assert.equal(binding.reconciled(UID), false, 'no unverified standalone parent exception');
  }
});

test('a hidden ancestor cannot authorize a different or modified canonical history chain', async () => {
  for (const mutate of [
    f => { f.nodes[UID].parentId = AID; },
    f => { f.nodes[f.nodes[UID].parent].message.metadata.is_visually_hidden_from_conversation = false; },
    f => { f.nodes[f.nodes[UID].parent].message.author.role = 'user'; },
    f => { f.nodes[f.nodes[UID].parent].message.id = AID; },
    f => { f.nodes[f.nodes[UID].parent].parentId = ROOT; },
    f => { f.nodes['synthetic-server-root'].children.push(AID); },
    f => { f.nodes['synthetic-server-root'].message.author.role = 'assistant'; },
    f => { f.nodes['synthetic-server-root'].parentId = AID; }
  ]) {
    const f = newFixture(), binding = await f.api.capture(f.node, null, admission);
    binding.adoptConversation(CID, { input_message: input }, UID);
    f.conversation.textHydrateHistory = async (_, options) => {
      const value = injectedHistory();
      options.onConversationLoadedFromNetwork(value);
      assert.equal(options.shouldApplyResponse(), true);
      applyHistory(f, value); mutate(f);
    };
    assert.equal(await historyModule.create().reconcile(binding, { userMessageId: UID }, new AbortController().signal), false);
    assert.equal(f.navigations, 1, 'the owned server route is independent of accepting a modified history tree');
  }
});

test('first-send parent history rejects visible, unrelated, malformed or cyclic ancestors before applying', async () => {
  for (const mutate of [
    p => { p.mapping[p.mapping[UID].parent].message.author.role = 'user'; },
    p => { p.mapping[p.mapping[UID].parent].message.author.role = 'developer'; },
    p => { p.mapping[p.mapping[UID].parent].message.metadata = {}; },
    p => { p.mapping[p.mapping[UID].parent].message.content.content_type = 'code'; },
    p => { p.mapping[p.mapping[UID].parent].children = [AID]; },
    p => { p.mapping[p.mapping[UID].parent].children.push(AID); },
    p => { p.mapping[p.mapping[UID].parent].message.id = AID; },
    p => { p.mapping[p.mapping[UID].parent].parent = UID; },
    p => { p.mapping['synthetic-server-root'].id = ROOT; },
    p => { p.mapping['synthetic-server-root'].parent = AID; },
    p => { p.mapping['synthetic-server-root'].children = []; },
    p => { p.mapping['synthetic-server-root'].message = response; },
    p => { p.mapping[''] = { id: '', message: null }; }
  ]) {
    const f = newFixture(), binding = await f.api.capture(f.node, null, admission);
    binding.adoptConversation(CID, { input_message: input }, UID);
    const value = injectedHistory(); mutate(value);
    assert.equal(historyModule.create().ownsResponse(value, binding, UID), false);
  }
});

test('new personal/project roots require explicit admission and preserve selected model state', async () => {
  for (const project of [false, true]) {
    const f = newFixture(project);
    await assert.rejects(f.api.capture(f.node), /scope_unsupported/);
    const binding = await f.api.capture(f.node, null, admission);
    assert.equal(binding.conversationId, null); assert.equal(binding.parentId, ROOT);
    assert.equal(binding.newConversation, true); assert.equal(binding.projectId, project ? PROJECT : null);
    assert.equal(binding.requestedDefaultModel, 'fixture-default');
    assert.equal(binding.canReconcile(UID), false);
    f.node.isConnected = false;
    assert.equal(binding.current(), true, 'no ongoing composer readiness/DOM polling');
    binding.beforeDispatch(); assert.deepEqual(f.remembered, ['fixture-default']);
    f.key = 'different-home-entry'; assert.equal(binding.owns(), false);
  }
});

test('new root admission rejects hidden existing state, privacy, files and uncertain model ownership', async () => {
  for (const change of [f => { f.binding.newThread = false; }, f => { f.binding.serverId = CID; },
    f => { f.props.isNewThread = false; }, f => { f.parent.id = AID; },
    f => { f.parent.author.role = 'assistant'; }, f => { f.messages.push(input); },
    f => { f.parent.content.parts.push('not an empty root'); }, f => { f.tree.isLoading = true; },
    f => { f.tree.is_do_not_remember = true; }, f => { f.shared.textHistoryDisabled = () => true; },
    f => { f.key = null; }, f => { f.files.files$ = () => [{}]; },
    f => { f.shared.textClientConversation = () => false; }, f => { f.shared.textResolvedConversationId = () => CID; },
    f => { f.shared.canvasConversations = () => [f.selected, { ...f.selected }]; },
    f => { f.conversation.textRequestedDefaultModel = () => ({ model: 'unknown' }); },
    f => { f.page.location.href += '?temporary-chat=true'; }]) {
    const f = newFixture(); change(f);
    await assert.rejects(f.api.capture(f.node, null, admission));
    assert.equal(f.binds, 0); assert.equal(f.navigations, 0);
  }
});

test('the fresh body omits an invented conversation id and stop waits for the server binding', async () => {
  const f = newFixture(), binding = await f.api.capture(f.node, null, admission);
  f.page.crypto = { randomUUID: () => UID };
  const req = requestModule.create(f.page).create(binding, { requestId: 'mcp_1', prompt: 'Synthetic first turn' });
  assert.equal('conversation_id' in req.preparationBody(), false);
  assert.equal(req.preparationBody().parent_message_id, ROOT);
  assert.equal('requested_default_model' in req.preparationBody(), false);
  const dispatched = req.consume({ conduit_token: 'synthetic' }, { chatReq: { token: 'synthetic' } },
    () => ({ 'openai-sentinel-chat-requirements-token': 'synthetic' }), binding.current);
  assert.equal('conversation_id' in dispatched.body, false);
  assert.equal(dispatched.body.requested_default_model, 'fixture-default');
  assert.equal(dispatched.body.one_off_model_override, true);
  assert.throws(() => req.consumeStop([], () => true), /stop_ownership_unavailable/);
  assert.equal(binding.adoptConversation(CID, { input_message: input }, UID), true);
  assert.equal(binding.conversationId, CID); assert.equal(f.binds, 1);
  assert.equal(req.consumeStop([], binding.owns).requestBody.conversation_id, CID);
  assert.throws(() => req.consumeStop([], binding.owns), /stop_ownership_unavailable/);
});

test('server identity adoption is single-owner, immutable and not authorized by arbitrary metadata', async () => {
  for (const change of [f => { f.key = 'new-home'; }, f => { f.identity('different-account'); },
    f => { f.page.document = {}; }, f => { f.props.conversation = {}; },
    f => { f.shared.cX = () => true; }, f => { f.tree.is_do_not_remember = true; },
    f => { f.shared.textHistoryDisabled = () => true; }, f => { f.shared.uo = () => true; },
    f => { f.shared.canvasConversations = () => [f.selected, { serverId$: () => CID }]; }]) {
    const f = newFixture(), binding = await f.api.capture(f.node, null, admission); change(f);
    assert.equal(binding.adoptConversation(CID, { input_message: input }, UID), false);
    assert.equal(f.binds, 0);
  }
  const f = newFixture(), binding = await f.api.capture(f.node, null, admission);
  assert.equal(binding.adoptConversation(CID, { type: 'unknown' }, UID), false);
  assert.equal(binding.adoptConversation(CID, { input_message: { ...input, id: AID } }, UID), false);
  assert.equal(binding.adoptConversation(CID, { type: 'resume_conversation_token' }, UID), true);
  assert.equal(binding.adoptConversation(CID, { input_message: input }, UID), true);
  assert.equal(binding.adoptConversation(AID, { message: response }, UID), false);
  assert.equal(f.binds, 1);
});

test('owned navigation precedes history; empty legacy root is normalized without accepting another parent', async () => {
  for (const project of [false, true]) for (const emptyRoot of [false, true]) {
    const f = newFixture(project), binding = await f.api.capture(f.node, null, admission);
    binding.adoptConversation(CID, { input_message: input }, UID);
    const history = historyModule.create(), value = payload(project, emptyRoot);
    assert.equal(history.ownsResponse(value, binding, UID), true);
    assert.equal(history.ownsResponse(payload(project, false, AID), binding, UID), false);
    for (const privacy of [{ is_do_not_remember: true }, { is_temporary_chat: true },
      { shared_project_conversation_owner: 'another-owner' }]) {
      assert.equal(history.ownsResponse({ ...value, ...privacy }, binding, UID), false);
    }
    assert.equal(binding.navigationReady(), false);
    f.conversation.textHydrateHistory = async (id, options) => {
      assert.equal(id, CID); assert.equal(f.navigations, 1);
      options.onConversationLoadedFromNetwork(value);
      assert.equal(options.shouldApplyResponse(), true);
      applyHistory(f, value);
    };
    assert.equal(await history.reconcile(binding, { userMessageId: UID }, new AbortController().signal), true);
    assert.equal(binding.navigationReady(), true); assert.equal(binding.owns(), true); assert.equal(f.navigations, 1);
    f.key = 'another-entry-at-same-url'; assert.equal(binding.owns(), false);
  }
});

test('delayed official navigation expires on route/identity changes, cancellation and timeout', async () => {
  for (const change of [f => { f.key = 'another-home'; }, f => { f.identity('another-account'); },
    f => { f.page.document = {}; }, (f, controller) => controller.abort(), () => {}]) {
    const f = newFixture(true), binding = await f.api.capture(f.node, null, admission);
    binding.adoptConversation(CID, { input_message: input }, UID);
    let navigate;
    f.conversation.textNavigateConversation = callback => { navigate = callback; };
    f.page.setTimeout = callback => setTimeout(callback, 0);
    const controller = new AbortController(), pending = binding.finalize(controller.signal);
    change(f, controller);
    assert.equal(await pending, false);
    navigate('/g/' + PROJECT + '/c/' + CID, { replace: true });
    assert.equal(f.navigations, 0); assert.equal(binding.navigationReady(), false);
  }
});

test('a rejected asynchronous project route never reports successful navigation', async () => {
  const f = newFixture(true), binding = await f.api.capture(f.node, null, admission);
  binding.adoptConversation(CID, { input_message: input }, UID);
  f.conversation.textNavigateConversation = async () => { throw Error('synthetic route unavailable'); };
  f.page.setTimeout = callback => setTimeout(callback, 0);
  assert.equal(await binding.finalize(new AbortController().signal), false);
  assert.equal(binding.navigationReady(), false); assert.equal(f.navigations, 0);
});

test('first-response navigation and history finalization share one pending route request', async () => {
  const f = newFixture(true), binding = await f.api.capture(f.node, null, admission);
  binding.adoptConversation(CID, { input_message: input }, UID);
  let navigate, requests = 0;
  f.conversation.textNavigateConversation = callback => { requests++; navigate = callback; };
  const signal = new AbortController().signal;
  const first = binding.finalize(signal), history = binding.finalize(signal);
  assert.equal(requests, 1);
  navigate('/g/' + PROJECT + '/c/' + CID, { replace: true });
  assert.equal(await first, true); assert.equal(await history, true);
  assert.equal(f.navigations, 1);
});

test('a cancelled navigation waiter cannot report another waiter success', async () => {
  const f = newFixture(true), binding = await f.api.capture(f.node, null, admission);
  binding.adoptConversation(CID, { input_message: input }, UID);
  let navigate, requests = 0;
  f.conversation.textNavigateConversation = callback => { requests++; navigate = callback; };
  const controller = new AbortController(), first = binding.finalize(new AbortController().signal);
  const cancelled = binding.finalize(controller.signal); controller.abort();
  navigate('/g/' + PROJECT + '/c/' + CID, { replace: true });
  assert.equal(await first, true); assert.equal(await cancelled, false);
  assert.equal(requests, 1); assert.equal(f.navigations, 1);
});

test('the owned v1 decoder binds the first server id before publishing native text', async () => {
  const f = newFixture(), binding = await f.api.capture(f.node, null, admission);
  const session = policy.createSession({ now: Date.now });
  const owned = require(assets + 'chatgpt_web_private_owned_stream').create({ policy, session,
    conversationId: value => value.conversation_id || '', notify() {} });
  const sink = owned.begin({ conversationId: null, current: binding.owns,
    adoptConversation: (id, value) => binding.adoptConversation(id, value, UID) });
  sink.push({ event: 'delta_encoding', data: 'v1' });
  sink.push({ event: 'delta', data: { p: '', o: 'add', v: { conversation_id: CID,
    message: { ...response, status: 'in_progress', content: { content_type: 'text', parts: ['Hello'] } } } } });
  sink.push({ event: 'delta', data: { p: '/message/content/parts/0', o: 'append', v: ' world' } });
  assert.equal(f.binds, 1); assert.equal(binding.conversationId, CID);
  assert.equal(session.current('/').text, 'Hello world');
  assert.throws(() => sink.push({ data: { conversation_id: AID, message: response } }), /stream_owner_changed/);
});

async function integration(project = false, settings = {}) {
  const f = newFixture(project), counts = { prepare: 0, posts: 0, apply: 0, stopped: 0, renders: 0 };
  const page = f.page; let draft = '', sequence = 0;
  Object.assign(page, { AbortController, crypto: { randomUUID: () => [UID, AID, UID2, AID2][sequence++] },
    __elonChatGptPrivateTextTransactionsEnabled: true,
    __elonChatGptFreshTextNewConversationsEnabled: true, __elonChatGptFreshTextProjectsEnabled: project,
    __elonChatGptFreshTextRequest: requestModule, __elonChatGptFreshTextReconcile: historyModule,
    __elonChatGptFreshTextReceipts: require(assets + 'chatgpt_web_fresh_text_receipts'),
    __elonChatGptFreshTextRecovery: require(assets + 'chatgpt_web_fresh_text_recovery'),
    __elonChatGptFreshTextStop: require(assets + 'chatgpt_web_fresh_text_stop'),
    __elonChatGptFreshTextStream: require(assets + 'chatgpt_web_fresh_text_stream') });
  if (settings.acceptedDefault) delete page.__elonChatGptFreshTextNewConversationsEnabled;
  const session = policy.createSession({ now: Date.now });
  const owned = require(assets + 'chatgpt_web_private_owned_stream').create({ policy, session,
    conversationId: value => value.conversation_id || value.conversationId || '',
    notify() { counts.renders++; } });
  page.__elonChatGptPrivateStreamTransport = {
    preparePrivateSend() { session.reset(); return true; }, beginPrivateStream: owned.begin,
    finishPrivateSend() { session.finish(); owned.reset(); }
  };
  f.shared.textTopic = () => { throw Error('this fixture uses root SSE'); };
  f.shared.$3 = () => true;
  f.shared.textApi.safePost = async (path, options) => {
    if (path === '/stop_conversation') {
      counts.stopped++; assert.equal(options.requestBody.conversation_id, CID); return {};
    }
    counts.prepare++; assert.equal(path, '/f/conversation/prepare');
    assert.equal(options.requestBody.conversation_id, counts.prepare === 1 ? undefined : CID);
    return { conduit_token: 'synthetic' };
  };
  f.shared.textSecurityHeaders = () => ({ 'openai-sentinel-chat-requirements-token': 'synthetic' });
  f.conversation.textSecurity = () => ({ chatReq: { token: 'synthetic' } });
  let release;
  const pause = new Promise(resolve => { release = resolve; });
  f.conversation.textStream = async function* (_, options) {
    options.onBeforeRequestStart(); counts.posts++;
    const first = counts.posts === 1, user = first ? input : { ...input, id: UID2 },
      answer = first ? response : { ...response, id: AID2 };
    assert.equal(options.body.conversation_id, first ? undefined : CID);
    assert.equal(options.body.parent_message_id, first ? ROOT : AID);
    assert.equal(options.body.messages[0].id, user.id);
    yield { response: { ok: true, headers: { get: () => 'text/event-stream' } } };
    if (settings.lostBeforeId) throw Error('synthetic connection lost');
    yield { data: { conversation_id: CID, input_message: user } };
    if (first && settings.retireUnnavigatedHome) {
      counts.firstEventNavigations = f.navigations;
      // A server-bound thread must leave the empty-home surface before the next
      // asynchronous event can replace that surface's conversation provider.
      if (f.navigations === 0) f.props.conversation = {};
    }
    yield { data: { conversation_id: CID, message: { ...answer, status: 'in_progress', end_turn: false,
      content: { content_type: 'text', parts: ['Synthetic'] } } } };
    if (settings.stop) await pause;
    yield { data: { conversation_id: CID, message: answer } };
  };
  f.conversation.textHydrateHistory = async (_, options) => {
    const data = settings.injectedParents === 'paginated' ? paginatedHistory(project) :
      settings.injectedParents ? injectedHistory(project) : payload(project);
    if (counts.posts === 2) {
      data.mapping[UID2] = { id: UID2, parent: AID, message: { ...input, id: UID2 } };
      data.mapping[AID2] = { id: AID2, parent: UID2, message: { ...response, id: AID2 } };
      data.current_node = AID2;
    }
    if (settings.stop && !counts.stopped) data.async_status = 3;
    if (counts.stopped) data.mapping[AID].message = { ...response, status: 'finished_partial_completion' };
    options.onConversationLoadedFromNetwork(data);
    if (options.shouldApplyResponse()) { applyHistory(f, data); counts.apply++; }
  };
  if (settings.navigationFails) {
    f.conversation.textNavigateConversation = async () => { throw Error('synthetic navigation failed'); };
    page.setTimeout = (callback, delay) => setTimeout(callback, delay === 100 ? 0 : delay);
  }
  if (settings.delayedNavigation) {
    const navigate = f.conversation.textNavigateConversation;
    f.conversation.textNavigateConversation = (...args) => {
      counts.navigationRequests = (counts.navigationRequests || 0) + 1;
      f.completeNavigation = () => navigate(...args);
    };
  }
  const api = transactionModule.create(page, { context: f.api, reconcileTimeoutMs: 1000 });
  const command = { requestId: 'mcp_1', prompt: 'Synthetic first turn', expectedDraft: '', composer: f.node,
    readDraft: () => draft, onDispatch() {}, onSettled() {} };
  const sent = api.send(command); assert.equal((await sent.completion).status, 'accepted');
  return { f, api, counts, session, sent, command, release, setDraft: value => { draft = value; } };
}

const tick = () => new Promise(resolve => setTimeout(resolve, 5));
async function settled(api) { for (let i = 0; i < 80 && api.state().pending; i++) await tick(); }

test('the first owned server event navigates before later stream events can retire the home owner', async () => {
  for (const project of [false, true]) {
    const r = await integration(project, { retireUnnavigatedHome: true });
    await settled(r.api);
    assert.equal(r.counts.firstEventNavigations, 1);
    assert.equal(r.api.state().pending, false);
    assert.equal(r.session.current(new URL(r.f.page.location.href).pathname).text, 'Synthetic answer');
    assert.deepEqual([r.counts.posts, r.counts.apply, r.f.binds, r.f.navigations], [1, 1, 1, 1]);
    assert.equal(r.api.dispose(), true);
  }
});

test('delayed project navigation does not block native streaming or duplicate its route request', async () => {
  const r = await integration(true, { delayedNavigation: true });
  for (let i = 0; i < 30 && r.api.state().phase !== 'reconciling'; i++) await tick();
  assert.equal(r.session.current('/').text, 'Synthetic answer');
  assert.equal(r.api.state().pending, true); assert.equal(r.counts.apply, 0);
  assert.equal(r.counts.navigationRequests, 1);
  r.f.completeNavigation();
  await settled(r.api);
  assert.equal(r.api.state().pending, false);
  assert.deepEqual([r.counts.posts, r.counts.apply, r.f.navigations, r.counts.navigationRequests], [1, 1, 1, 1]);
  assert.equal(r.api.dispose(), true);
});

test('personal default and project opt-in bind, stream, hydrate and permit one existing-conversation follow-up', async () => {
  for (const project of [false, true]) for (const injectedParents of [false, true, 'paginated']) {
    const r = await integration(project, { injectedParents, acceptedDefault: !project }); r.setDraft('a later unsent draft');
    await settled(r.api);
    assert.equal(r.api.state().pending, false);
    assert.deepEqual([r.counts.prepare, r.counts.posts, r.counts.apply, r.f.binds, r.f.navigations], [1, 1, 1, 1, 1]);
    assert.ok(r.counts.renders > 0);
    assert.equal(r.session.current(new URL(r.f.page.location.href).pathname).text, 'Synthetic answer');
    r.f.binding.href = r.f.page.location.href;
    const next = await r.f.api.capture(r.f.node, null, { allowProjects: project });
    assert.equal(next.newConversation, false); assert.equal(next.conversationId, CID); assert.equal(next.parentId, AID);
    assert.equal(r.api.send(r.command).handled, true);
    assert.equal(r.counts.posts, 1, 'a repeated native command cannot create a second first turn');
    r.setDraft('');
    const followup = r.api.send({ ...r.command, requestId: 'mcp_2', prompt: 'Synthetic follow up' });
    assert.equal((await followup.completion).status, 'accepted');
    await settled(r.api);
    assert.equal(r.api.state().pending, false);
    assert.deepEqual([r.counts.prepare, r.counts.posts, r.counts.apply, r.f.binds, r.f.navigations], [2, 2, 2, 1, 1]);
    assert.equal(r.f.nodes[UID2].parent, AID); assert.equal(r.f.leaf.id, AID2);
    assert.equal(r.api.dispose(), true);
  }
});

test('failed navigation retains the writer without applying history or replaying POST', async () => {
  const r = await integration(true, { navigationFails: true });
  for (let i = 0; i < 30 && !r.api.state().dispatched; i++) await tick();
  await r.api.recover().completion;
  assert.equal(r.counts.apply, 0); assert.equal(r.f.navigations, 0);
  assert.equal(r.api.state().pending, true);
  assert.equal((await r.api.send({ ...r.command, requestId: 'mcp_2' }).completion).code, 'busy');
  assert.equal(r.counts.posts, 1); assert.equal(r.sent.claimFallback(), false);
  r.f.page.document = {}; r.api.state(); assert.equal(r.api.dispose(), true);
});

test('new-conversation stop uses the acquired server id and keeps partial native text', async () => {
  const r = await integration(false, { stop: true });
  for (let i = 0; i < 30 && !r.f.serverId; i++) await tick();
  const stopped = await r.api.stop().completion;
  assert.equal(stopped.status, 'accepted'); assert.equal(r.counts.stopped, 1);
  assert.equal(r.counts.posts, 1); assert.equal(r.f.navigations, 1);
  assert.equal(r.session.current('/c/' + CID).text, 'Synthetic');
  r.release(); await settled(r.api); assert.equal(r.api.dispose(), true);
});

test('losing the response before server identity preserves an uncertain writer and never guesses or replays', async () => {
  const r = await integration(false, { lostBeforeId: true });
  await tick(); await tick();
  assert.equal(r.api.state().pending, true); assert.equal(r.f.binds, 0); assert.equal(r.f.navigations, 0);
  assert.equal(r.counts.apply, 0); assert.equal(r.api.send({ ...r.command, requestId: 'mcp_2' }).handled, true);
  assert.equal(r.counts.posts, 1); assert.equal(r.sent.claimFallback(), false);
  r.f.page.document = {}; r.api.state(); assert.equal(r.api.dispose(), true);
});

'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture, CID, PID } = require('./fixtures/chatgpt-fresh-text-context');

const PROJECT = 'g-p-' + 'a'.repeat(32);
function projectFixture(path = '/c/' + CID) {
  const f = fixture();
  f.tree.mode = { kind: 'gizmo_interaction', gizmo_id: PROJECT, gizmo: { name: 'fixture display only' } };
  f.tree.isLoading = false; f.tree.is_do_not_remember = false;
  f.shared.HM.getGizmoId = state => state.mode.gizmo_id;
  f.shared.HM.getConversationTurns = () => [];
  f.shared.textBusinessContext = () => null;
  f.shared.canvasQueryClient = () => 'fixture-query-client';
  f.shared.textLockedChatPin = () => undefined;
  f.shared.textLockedProjectId = () => null;
  f.shared.textProjectHeaders = () => undefined;
  f.page.location.href = f.binding.href = 'https://chatgpt.com' + path;
  return f;
}

test('owned project is admitted only explicitly, on both canonical and project routes', async () => {
  for (const path of ['/c/' + CID, '/g/' + PROJECT + '/c/' + CID, '/g/' + PROJECT + '-fixture/c/' + CID]) {
    const f = projectFixture(path);
    await assert.rejects(f.api.capture(f.node), /scope_unsupported/);
    const binding = await f.api.capture(f.node, null, { allowProjects: true });
    assert.equal(binding.projectId, PROJECT);
    assert.equal(binding.current(), true);
    assert.equal(binding.owns(), true);
    assert.deepEqual(binding.projectHeaders, {});
    f.node.isConnected = false;
    assert.equal(binding.current(), true, 'later DOM layout is not the dispatch owner');
  }
});

test('project scope rejects foreign, incomplete, temporary and business-agent contexts', async () => {
  for (const mutate of [
    f => { f.page.location.href = f.binding.href = 'https://chatgpt.com/g/g-p-' + 'b'.repeat(32) + '/c/' + CID; },
    f => { f.tree.mode.gizmo_id = 'g-custom'; }, f => { f.tree.mode.kind = 'gizmo_test'; },
    f => { f.tree.mode.unknown = true; }, f => { f.tree.isLoading = true; },
    f => { delete f.tree.isLoading; }, f => { f.tree.is_do_not_remember = true; },
    f => { f.tree.sharedProjectConversationOwner = {}; }, f => { f.tree.contextScopes = [{}]; },
    f => { f.tree.continuingFromSharedProjectConversationId = CID; },
    f => { f.shared.textBusinessContext = () => ({ businessAgentId: 'fixture' }); },
    f => { f.shared.textBusinessContext = () => undefined; },
    f => { f.shared.textLockedChatPin = undefined; },
    f => { f.shared.HM.getGizmoId = () => null; },
    f => { f.shared.SV.isPersonalWorkspace = () => false; },
    f => { f.shared.textProjectHeaders = () => ({ authorization: 'not-allowed' }); }
  ]) {
    const f = projectFixture(); mutate(f);
    await assert.rejects(f.api.capture(f.node, null, { allowProjects: true }));
  }
});

test('project move or owner restrictions after capture stop writes and history apply', async () => {
  for (const mutate of [
    f => { f.tree.mode.gizmo_id = 'g-p-' + 'b'.repeat(32); },
    f => { f.tree.sharedProjectConversationOwner = {}; },
    f => { f.tree.is_do_not_remember = true; },
    f => { f.shared.SV.isPersonalWorkspace = () => false; },
    f => { f.tree.mode = { kind: 'primary_assistant' }; }
  ]) {
    const f = projectFixture(), binding = await f.api.capture(f.node, null, { allowProjects: true });
    mutate(f);
    assert.equal(binding.current(), false); assert.equal(binding.owns(), false);
    assert.equal(binding.canReconcile('fixture-user'), false);
    assert.equal(binding.canStop('fixture-user'), false);
  }
  const f = fixture(), binding = await f.api.capture(f.node, null, { allowProjects: true });
  f.tree.mode = { kind: 'gizmo_interaction', gizmo_id: PROJECT };
  assert.equal(binding.owns(), false, 'ordinary chat cannot become a project behind an owned send');
});

test('project headers use current page helpers and change invalidates a prepared request', async () => {
  const f = projectFixture(), calls = [];
  let pin = 'fixture-authorized-pin';
  f.shared.textBusinessContext = value => { calls.push(value); return null; };
  f.shared.textLockedProjectId = query => { assert.equal(query, 'fixture-query-client'); return PROJECT; };
  f.shared.textLockedChatPin = () => pin;
  f.shared.textProjectHeaders = (project, locked, value) => {
    assert.equal(project, PROJECT); assert.equal(locked, PROJECT); assert.equal(value, pin);
    return { 'x-openai-locked-chats-pin': value };
  };
  const binding = await f.api.capture(f.node, null, { allowProjects: true });
  assert.deepEqual(calls[0], { turns: [], gizmoId: PROJECT, conversationId: CID });
  assert.equal(Object.isFrozen(binding.projectHeaders), true);
  assert.equal(binding.current(), true);
  pin = 'fixture-changed-pin';
  assert.equal(binding.current(), false);
  assert.equal(binding.owns(), true, 'post-dispatch read-only recovery retains its original scope');
});

test('project tools reuse the existing selected-tool admission without changing scope', async () => {
  const f = projectFixture(); f.hints.activeSystemHintType = 'search';
  await assert.rejects(f.api.capture(f.node, null, { allowProjects: true }), /tools_active/);
  const binding = await f.api.capture(f.node, null, { allowProjects: true, allowTools: true });
  assert.equal(binding.projectId, PROJECT); assert.equal(binding.tool, 'search');
});

test('real project modules compose from ownership to one POST and authoritative history', async () => {
  const f = projectFixture('/g/' + PROJECT + '/c/' + CID), calls = [];
  const asset = name => require('../android/app/src/main/assets/chatgpt_web_fresh_text_' + name);
  const events = new EventTarget();
  f.page.document = new EventTarget(); f.page.document.visibilityState = 'visible';
  Object.assign(f.page, { crypto: require('node:crypto'), AbortController, setTimeout, clearTimeout,
    addEventListener: events.addEventListener.bind(events), removeEventListener: events.removeEventListener.bind(events),
    __elonChatGptPrivateTextTransactionsEnabled: true,
    __elonChatGptFreshTextProjectsEnabled: true, __elonChatGptFreshTextStream: asset('stream') });
  f.page.location.origin = 'https://chatgpt.com';
  let sent;
  f.shared.textApi.safePost = async (_, options) => {
    calls.push('prepare'); assert.equal(options.requestBody.conversation_mode.gizmo_id, PROJECT);
    return { conduit_token: 'fixture-conduit' };
  };
  f.shared.textSecurityHeaders = () => ({ 'OpenAI-Sentinel-Chat-Requirements-Token': 'fixture' });
  f.shared.textTopic = () => { throw Error('unexpected handoff'); };
  f.conversation.textSecurity = () => ({ chatReq: { token: 'fixture-fresh' } });
  f.conversation.textStream = (_, options) => (async function* () {
    options.onBeforeRequestStart(); calls.push('post'); sent = options.body;
    yield { response: new Response(null, { headers: { 'content-type': 'text/event-stream' } }) };
    yield { data: { type: 'done' } };
  })();
  f.page.__elonChatGptPrivateStreamTransport = { preparePrivateSend: () => true,
    beginPrivateStream: () => ({ push() {}, finish() {} }), finishPrivateSend: () => calls.push('finish') };
  f.conversation.textHydrateHistory = async (_, options) => {
    calls.push('history');
    const uid = sent.messages[0].id, aid = '44444444-4444-4444-8444-444444444444';
    const payload = { conversation_id: CID, gizmo_id: PROJECT, is_do_not_remember: false,
      current_node: aid, async_status: null, mapping: {
        [uid]: { id: uid, parent: PID, message: { id: uid, author: { role: 'user' } } },
        [aid]: { id: aid, parent: uid, message: { id: aid, author: { role: 'assistant' },
          status: 'finished_successfully', end_turn: true } }
      } };
    options.onConversationLoadedFromNetwork(payload);
    assert.equal(options.shouldApplyResponse(), true);
    f.parent.id = aid;
    f.shared.HM.getNodeIfExists = (_, id) => payload.mapping[id];
    f.shared.HM.getParentNode = () => ({ id: PID });
    f.shared.HM.getParentPromptNode = () => ({ id: uid });
  };
  const reconciliation = asset('reconcile').create();
  const recovery = asset('recovery').create(f.page, { reconciliation, delays: [0], timeoutMs: 1000 });
  const api = asset('transaction').create(f.page, { context: f.api, requests: asset('request'),
    receipts: asset('receipts'), reconciliation, recovery });
  const result = api.send({ requestId: 'mcp_project1', prompt: 'Synthetic project fixture',
    expectedDraft: '', composer: f.node, readDraft: () => '', clearDraft() {} });
  assert.equal(result.handled, true);
  const receipt = await result.completion;
  assert.equal(receipt.status, 'accepted', JSON.stringify(receipt));
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(api.state().pending, false);
  assert.deepEqual(calls, ['prepare', 'post', 'history', 'finish']);
  assert.equal(api.trialControl('state').reconciled, true);
  assert.equal(api.dispose(), true);
});

test('capture does not use submitComposer or isComposerSubmissionReady', async () => {
  const f = fixture();
  Object.defineProperty(f.props, 'isComposerSubmissionReady', { get() { throw Error('must not read'); } });
  Object.defineProperty(f.props, 'submitComposer', { get() { throw Error('must not read'); } });
  const binding = await f.api.capture(f.node);
  assert.equal(binding.parentId, PID); assert.equal(binding.model, 'fixture-model');
  assert.equal(binding.current(), true);
  f.node.isConnected = false;
  assert.equal(binding.current(), true, 'owned in-memory conversation is independent of later editor rendering');
});

for (const [name, mutate] of [
  ['provider profile', f => { f.page.__elonChatGptPrivateRuntimeBindings.state = () => ({ profile_id: 'unknown' }); }],
  ['project route', f => { f.page.location.href = 'https://chatgpt.com/g/g-p-fixture/c/' + CID; }],
  ['temporary route', f => { f.page.location.href += '?temporary-chat=true'; }],
  ['workspace', f => { f.shared.SV.isPersonalWorkspace = () => false; }],
  ['project state', f => { f.tree.mode = { kind: 'gizmo_interaction' }; }],
  ['consumer restriction', f => { f.props.shouldBlockConsumerLockdownModeActionsForConversation = true; }],
  ['upload', f => { f.files.hasUploadInProgress$ = () => true; }],
  ['attachment', f => { f.files.files$ = () => [{}]; }],
  ['search', f => { f.hints.activeSystemHintType = 'search'; }],
  ['connector', f => { f.hints.activeConnectorSystemHintTypes.add('fixture-tool'); }],
  ['review', f => { f.conversation.textReviewAck = () => ({}); }],
  ['voice', f => { f.shared.Fx = () => ({ value: 5 }); }],
  ['request', f => { f.shared.Fl = () => true; }],
  ['override model', f => { f.shared.textModelOverride = () => ({ model_slug: 'fixture-model' }); }],
  ['unfinished parent', f => { f.parent.end_turn = false; }],
  ['shared continuation', f => { f.tree.continuingFromSharedConversationId = 'fixture'; }],
  ['unknown configured behavior', f => { f.selected.config.alwaysOnSystemHints = ['fixture']; }]
]) test(name + ' declines before HTTP preparation', async () => {
  const f = fixture(); mutate(f); await assert.rejects(f.api.capture(f.node));
});

test('account switch during asynchronous binding load is rejected', async () => {
  const f = fixture(), load = f.page.__elonChatGptPrivateRuntimeBindings.load;
  f.page.__elonChatGptPrivateRuntimeBindings.load = async role => { f.identity('another-account'); return load(role); };
  await assert.rejects(f.api.capture(f.node), /context_changed/);
});

test('a user-only parent requires this sender confirmed stop and current branch proof', async () => {
  const f = fixture(); f.parent.author.role = 'user'; f.parent.end_turn = false;
  await assert.rejects(f.api.capture(f.node), /parent_unavailable/);
  await assert.rejects(f.api.capture(f.node, { id: CID, current: () => true }), /parent_unavailable/);
  await assert.rejects(f.api.capture(f.node, { id: PID, current: () => false }), /parent_unavailable/);
  let valid = true;
  const binding = await f.api.capture(f.node, { id: PID, current: () => valid });
  assert.equal(binding.parentId, PID); assert.equal(binding.parentRole, 'user');
  assert.equal(binding.current(), true);
  valid = false; assert.equal(binding.current(), false);
});

test('stopped-user continuation never bypasses busy, identity, route or selected-parent checks', async () => {
  for (const mutate of [f => { f.shared.Fl = () => true; },
    f => { f.shared.Fx = () => ({ value: 3 }); },
    f => { f.props.currentLeafId = CID; }, f => { f.page.location.href += '?temporary-chat=true'; }]) {
    const f = fixture(); f.parent.author.role = 'user'; mutate(f);
    await assert.rejects(f.api.capture(f.node, { id: PID, current: () => true }));
  }
  const f = fixture(); f.parent.author.role = 'user';
  const binding = await f.api.capture(f.node, { id: PID, current: () => true });
  f.identity('another'); assert.equal(binding.current(), false);
});

test('account, parent, model and document must still match before dispatch', async () => {
  for (const mutate of [f => f.identity('another'), f => { f.parent.id = CID; },
    f => { f.conversation.Nrn = () => ({ id: 'another-model' }); },
    f => { f.page.document = {}; }, f => { f.page.location.href += '?changed'; }]) {
    const f = fixture(), binding = await f.api.capture(f.node); mutate(f); assert.equal(binding.current(), false);
  }
});

test('read-only reconciliation requires own user and original parent on the selected branch', async () => {
  const f = fixture(), binding = await f.api.capture(f.node), uid = '33333333-3333-4333-8333-333333333333';
  f.shared.HM.getNodeIfExists = (_, id) => id === uid ? { message: { author: { role: 'user' } } } : null;
  f.shared.HM.getParentNode = () => ({ id: PID });
  f.shared.HM.getParentPromptNode = () => ({ id: uid });
  assert.equal(binding.canReconcile(uid), true);
  assert.equal(binding.reconciled(uid), false);
  f.parent.id = '44444444-4444-4444-8444-444444444444';
  assert.equal(binding.reconciled(uid), true);
  f.shared.HM.getParentPromptNode = () => ({ id: 'other' });
  assert.equal(binding.reconciled(uid), false);
  assert.equal(binding.canReconcile(uid), false);
});

test('stopped partial assistant can be the next parent but an active writer cannot be stopped', async () => {
  const f = fixture(); f.parent.status = 'finished_partial_completion'; f.parent.end_turn = false;
  const binding = await f.api.capture(f.node);
  assert.equal(binding.parentId, PID); assert.equal(binding.canStop('fixture-user'), true);
  f.shared.Fx = () => ({ value: 3 });
  assert.equal(binding.canStop('fixture-user'), true, 'history will prove which server stream owns this request');
  assert.equal(binding.reconciled('fixture-user', true), false);
  f.shared.Fl = () => true; assert.equal(binding.canStop('fixture-user'), false);
  f.shared.Fl = () => false; f.shared.Fx = () => ({ value: 5 });
  assert.equal(binding.canStop('fixture-user'), false);
});

test('history apply never replaces a newly active runtime writer or voice session', async () => {
  for (const mutate of [f => { f.shared.Fl = () => true; },
    f => { f.shared.Fx = () => ({ value: 5 }); },
    f => { f.page.__elonChatGptPrivateTextRuntimeSubmit.state = () => ({ pending: true }); },
    f => { f.page.__elonChatGptPrivateRegenerateRuntime = { state: () => ({ pending: true }) }; },
    f => { f.page.__elonChatGptPrivateTextTransactionRelay = { state: () => ({ active: true }) }; },
    f => { f.page.__elonChatGptCanvasDocumentActions = { generationPending: () => true }; }]) {
    const f = fixture(), binding = await f.api.capture(f.node);
    assert.equal(binding.canReconcile('fixture-user'), true);
    mutate(f);
    assert.equal(binding.canReconcile('fixture-user'), false);
    assert.equal(binding.reconciled('fixture-user'), false);
  }
});

test('real history reconciliation rejects a writer that starts between network receipt and apply', async () => {
  const f = fixture(), binding = await f.api.capture(f.node);
  const uid = '33333333-3333-4333-8333-333333333333', aid = '44444444-4444-4444-8444-444444444444';
  const payload = { conversation_id: CID, current_node: aid, async_status: null, mapping: {
    [uid]: { id: uid, parent: PID, message: { id: uid, author: { role: 'user' } } },
    [aid]: { id: aid, parent: uid, message: { id: aid, author: { role: 'assistant' },
      status: 'finished_successfully', end_turn: true } }
  } };
  let applies = 0;
  f.conversation.textHydrateHistory = async (_, options) => {
    options.onConversationLoadedFromNetwork(payload);
    f.shared.Fl = () => true;
    if (options.shouldApplyResponse()) applies++;
  };
  const history = require('../android/app/src/main/assets/chatgpt_web_fresh_text_reconcile').create();
  assert.equal(await history.reconcile(binding, { userMessageId: uid }, new AbortController().signal), false);
  assert.equal(applies, 0);
});

for (const tool of ['search', 'picture_v2']) {
  test(tool + ' needs explicit trial scope and existing account/model tool admission', async () => {
    const f = fixture(); f.hints.activeSystemHintType = tool;
    await assert.rejects(f.api.capture(f.node), /tools_active/);
    const capture = f.page.__elonChatGptPrivateComposerToolContext.capture;
    let admissions = 0;
    f.page.__elonChatGptPrivateComposerToolContext.capture = () => { admissions++; return capture(); };
    const owned = await f.api.capture(f.node, null, { allowTools: true });
    assert.equal(owned.tool, tool); assert.equal(owned.current(), true); assert.equal(admissions, 1);
    f.page.__elonChatGptPrivateComposerToolContext.capture = () => { throw Error('must not poll DOM'); };
    f.node.isConnected = false;
    assert.equal(owned.current(), true);
    f.hints.activeSystemHintType = null;
    assert.equal(owned.current(), false);
  });
}

test('plain text does not acquire or depend on the tool menu', async () => {
  const f = fixture(); delete f.page.__elonChatGptPrivateComposerToolContext;
  const owned = await f.api.capture(f.node, null, { allowTools: true });
  assert.equal(owned.tool, null); assert.equal(owned.current(), true);
  f.hints.activeSystemHintType = 'search'; assert.equal(owned.current(), false);
});

test('tool admission does not bypass ownership, filtering, restrictions or busy state', async () => {
  for (const mutate of [
    f => { f.page.__elonChatGptPrivateComposerToolContext.capture = () => null; },
    f => { f.hints.locked = true; },
    f => { f.hints.activeConnectorSystemHintTypes.add('connector'); },
    f => { f.hints.activeCustomAgentSystemHintType = 'agent'; },
    f => { f.hints.coldStartCampaignCreativeId = 'campaign'; },
    f => { f.hints.activeSystemHintType = 'canvas'; },
    f => { f.hints.activeSystemHintType = 'tatertot'; },
    f => { f.files.files$ = () => [{}]; },
    f => { f.props.isDisabled = true; },
    f => { f.shared.Fl = () => true; }
  ]) {
    const f = fixture(); f.hints.activeSystemHintType = 'search'; mutate(f);
    await assert.rejects(f.api.capture(f.node, null, { allowTools: true }));
  }
  for (const field of ['controller', 'conversation', 'shared', 'serverId', 'href', 'token', 'account', 'document', 'model', 'allowed']) {
    const f = fixture(); f.hints.activeSystemHintType = 'picture_v2';
    const capture = f.page.__elonChatGptPrivateComposerToolContext.capture;
    f.page.__elonChatGptPrivateComposerToolContext.capture = () => ({ ...capture(), [field]: 'different' });
    await assert.rejects(f.api.capture(f.node, null, { allowTools: true }), /tools_active|context_changed/);
  }
});

test('tool transactions freeze model and hint selection until actual dispatch', async () => {
  for (const mutate of [f => { f.conversation.Nrn = () => ({ id: 'another-model' }); },
    f => { f.hints.activeSystemHintType = 'picture_v2'; },
    f => { f.hints.locked = true; }, f => { f.identity('another'); }]) {
    const f = fixture(); f.hints.activeSystemHintType = 'search';
    const owned = await f.api.capture(f.node, null, { allowTools: true });
    mutate(f); assert.equal(owned.current(), false);
  }
});

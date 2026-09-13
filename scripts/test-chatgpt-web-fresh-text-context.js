'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const contextModule = require('../android/app/src/main/assets/chatgpt_web_fresh_text_context');
const CID = '11111111-1111-4111-8111-111111111111';
const PID = '22222222-2222-4222-8222-222222222222';
function fixture() {
  let identity = 'fixture-account';
  const node = { isConnected: true };
  const page = { document: {}, __elonChatGptDocumentToken: 'doc_fixture_context',
    location: { href: 'https://chatgpt.com/c/' + CID } };
  const selected = { id: 'fixture-local-thread', serverId$: () => CID, config: {} };
  const controller = { conversation: selected };
  const props = { conversation: selected, composerController: controller, isDisabled: false,
    isConsumerLockdownModeLoadingForConversation: false, shouldBlockConsumerLockdownModeActionsForConversation: false,
    structuredInputMessageId: null, currentLeafId: PID, isComposerSubmissionReady: false };
  const tree = { mode: { kind: 'primary_assistant' } };
  const parent = { id: PID, author: { role: 'assistant' }, status: 'finished_successfully', end_turn: true };
  const files = { files$: () => [], readyFiles$: () => [], hasUploadInProgress$: () => false };
  const hints = { locked: false, activeSystemHintType: null, activeConnectorSystemHintTypes: new Set(),
    activeCustomAgentSystemHintType: null, coldStartCampaignCreativeId: null };
  const shared = { wV: fn => fn(), SV: { isPersonalWorkspace: () => true },
    textApi: { safePost() {} }, textSecurityHeaders() {}, textHistoryDisabled: () => false,
    textModelOverride: () => ({ model_slug: 'another-model' }), XM: () => tree,
    HM: { getGizmoId: () => null, getRequestId: () => null, getCurrentMessage: () => parent },
    cX: () => false, uo: () => false, Fx: () => null, Fl: () => false, v7: { STREAMING: 3, UNREAD: 4 },
    canvasConversations: () => [selected] };
  const conversation = { textSecurity() {}, textStream() {}, textHydrateHistory() {},
    textPrepareEnabled: () => true, textReviewAck: () => null,
    Nrn: () => ({ id: 'fixture-model' }), yRt: () => ({ conversationThinkingEffort$: () => 'high' }),
    l0: () => ({ getServiceTierForSubmission$: () => 'standard' }) };
  const binding = { href: page.location.href, newThread: false, temporary: false, conversation: selected,
    controller, shared: { getSharedProps: () => props }, files, serverId: CID, token: page.__elonChatGptDocumentToken };
  page.__elonChatGptPrivateModelContract = { create: () => ({ withRuntimeIdentity: () => ({ account: identity }) }) };
  page.__elonChatGptPrivateTextRuntimeSubmit = { captureConversation: () => ({ ...binding, account: identity }), state: () => ({ pending: false }) };
  page.__elonChatGptPrivateRuntimeBindings = { state: () => ({ profile_id: 'web_20260912' }),
    peek: () => shared,
    load: async role => ({ shared, conversation, composer: { Ng: () => hints } })[role] };
  return { page, node, selected, props, tree, parent, files, hints, shared, conversation, binding,
    identity: value => { identity = value; }, api: contextModule.create(page) };
}

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

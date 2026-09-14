'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const { fixture, CID, PID, UID, ROOT, AID, OTHER } = require('./fixtures/chatgpt-fresh-regeneration');
const requests = require('../android/app/src/main/assets/chatgpt_web_fresh_text_request');
const history = require('../android/app/src/main/assets/chatgpt_web_fresh_text_reconcile').create();

test('ordinary retry captures the actual committed menu but never invokes its callback', async () => {
  const f = fixture(), binding = await f.capture();
  assert.equal(binding.operation, 'regenerate');
  assert.equal(binding.parentId, UID); assert.equal(binding.historyParentId, ROOT);
  assert.equal(binding.model, 'fixture-model'); assert.equal(binding.effort, 'high');
  assert.equal(binding.serviceTier, null); assert.equal(binding.variantPurpose, 'comparison_implicit');
  assert.equal(binding.current(), true); assert.equal(f.retry.calls.length, 0);
  f.node.isConnected = false; f.retry.turn.isConnected = false;
  assert.equal(binding.owns(), true, 'post-dispatch ownership does not depend on rendered buttons');
});

for (const resolved of [null, undefined]) test('ordinary retry retains its requested model when the optional work resolver returns ' + resolved, async () => {
  const f = fixture();
  f.conversation.textResolveRequestedModel = () => resolved;
  const binding = await f.capture();
  assert.equal(binding.model, 'fixture-model');
  assert.equal(binding.effort, 'high');
  assert.equal(binding.current(), true);
  assert.equal(f.retry.calls.length, 0);
});

test('ordinary retry without reply effort does not borrow a work-model default', async () => {
  const f = fixture();
  f.conversation.textResolveRequestedModel = () => null;
  delete f.parent.metadata.thinking_effort;
  const binding = await f.capture();
  assert.equal(binding.effort, null);
  f.retry.modelMenu.modelsData.models.get('fixture-model').defaultThinkingEffort = 'medium';
  assert.equal(binding.current(), true);
  f.parent.metadata.thinking_effort = 'high';
  assert.equal(binding.current(), false);
});

for (const [stage, change] of [
  ['ready', () => {}],
  ['base_composer', f => { f.props.isDisabled = true; }],
  ['base_prepare', f => { f.conversation.textPrepareEnabled = () => false; }],
  ['base_config', f => { f.selected.config.synthetic = true; }],
  ['base_branch', f => { f.tree.conversationOrigin = 'synthetic'; }],
  ['model', f => { f.conversation.textResolveRequestedModel = () => 'other-model'; }],
  ['user_identity', f => { f.user.message.id = OTHER; }],
  ['user_content', f => { f.user.message.content.parts = []; }],
  ['user_parent', f => { delete f.user.parentId; }],
  ['user_channel', f => { f.user.message.channel = 'final'; }],
  ['user_recipient', f => { f.user.message.recipient = 'python'; }],
  ['user_metadata', f => { f.user.message.metadata.system_hints = ['search']; }],
  ['reply_metadata', f => { f.parent.metadata.map_search_parameters = {}; }]
]) test('admission identifies ' + stage + ' without invoking a writer or exposing content', async () => {
  const f = fixture(); change(f);
  const value = await f.inspect();
  assert.equal(value.stage, stage);
  assert.equal(value.code, stage === 'ready' ? 'ready' : 'scope_unsupported');
  assert.deepEqual(Object.keys(value).sort(), ['code', 'schema', 'stage']);
  assert.equal(f.retry.calls.length, 0);
  assert.doesNotMatch(JSON.stringify(value), /Synthetic|fixture|[a-f0-9]{8}-/);
});

test('admission is single-flight and bounded when a read-only model resolver stalls', async () => {
  const f = fixture(); let timeout, clears = 0, resolves = 0;
  f.page.setTimeout = callback => { timeout = callback; return 1; };
  f.page.clearTimeout = () => { clears++; };
  f.conversation.textResolveRequestedModel = () => { resolves++; return new Promise(() => {}); };
  const a = f.inspect(), b = f.inspect(); assert.equal(a, b);
  await new Promise(setImmediate); assert.equal(resolves, 1); timeout();
  assert.equal((await a).code, 'timeout'); assert.equal(clears, 1);
  assert.equal(f.retry.calls.length, 0);
});

test('admission does not export arbitrary exception data', async () => {
  const f = fixture();
  f.conversation.textResolveRequestedModel = () => { throw Object.assign(Error('private-content'), { admissionStage: 'private-header' }); };
  assert.deepEqual(await f.inspect(), { schema: 'elon.fresh_regenerate_admission.v1', code: 'read_failed', stage: 'base_context' });
});

test('retry uses the reply effort or resolved model default, never the composer current tier', async () => {
  const f = fixture(); delete f.parent.metadata.thinking_effort;
  f.tree.variants.push(OTHER);
  const binding = await f.capture();
  assert.equal(binding.effort, 'low'); assert.equal(binding.variantPurpose, 'none');
  f.retry.modelMenu.modelsData.models.get('fixture-model').defaultThinkingEffort = 'medium';
  assert.equal(binding.current(), false);
});

for (const [name, change] of [
  ['disabled model', f => { f.retry.modelMenu.modelSwitcherDenialsBySlug['fixture-model'] = true; }],
  ['changed model resolver', f => { f.conversation.textResolveRequestedModel = () => 'different-model'; }],
  ['file parent', f => { f.user.message.metadata.attachments = [{ id: 'file-synthetic' }]; }],
  ['tool parent', f => { f.user.message.metadata.system_hints = ['search']; }],
  ['hidden feedback retry', f => { f.user.message.metadata.is_contextual_retry_user_message = true; }],
  ['map result', f => { f.parent.metadata.map_search_parameters = {}; }],
  ['missing catalog', f => { f.retry.modelMenu.modelsData.models.clear(); }],
  ['invalid ancestor', f => { f.user.parentId = 'unreviewed-root'; }],
  ['non-string ancestor', f => { f.user.parentId = [ROOT]; }],
  ['wire-shaped runtime node', f => { f.user.parent = f.user.parentId; delete f.user.parentId; }]
]) test(name + ' does not guess an independent regeneration contract', async () => {
  const f = fixture(); change(f);
  await assert.rejects(f.capture(), /context_|scope_/);
  assert.equal(f.retry.calls.length, 0);
});

test('branch and account changes invalidate admission before writing', async () => {
  const f = fixture(), binding = await f.capture();
  f.user.message.content.parts[0] = 'changed prompt'; assert.equal(binding.current(), false);
  f.user.message.content.parts[0] = 'Synthetic original prompt'; assert.equal(binding.current(), true);
  f.tree.variants.push(OTHER); assert.equal(binding.current(), false); f.tree.variants.pop();
  f.retry.identity.accountId = OTHER; assert.equal(binding.owns(), false);
});

test('runtime parentId and wire parent stay separate across history reconciliation', async () => {
  const f = fixture(), binding = await f.capture();
  assert.equal(f.user.parentId, ROOT); assert.equal('parent' in f.user, false);
  const snapshot = f.history();
  assert.equal(snapshot.mapping[UID].parent, ROOT);
  assert.equal('parentId' in snapshot.mapping[UID], false);
  f.apply(snapshot);
  assert.equal(f.tree.nodes[UID].parentId, ROOT);
  assert.equal('parent' in f.tree.nodes[UID], false);
  assert.equal(binding.canReconcile(UID), true);
  f.tree.nodes[UID].parentId = OTHER;
  assert.equal(binding.canReconcile(UID), false, 'runtime reparenting invalidates the owned prompt');
});

test('fresh variant prepares action next, dispatches variant and never creates a second user message', async () => {
  const f = fixture(), binding = await f.capture();
  f.page.crypto = crypto;
  const request = requests.create(f.page).create(binding, f.command);
  assert.equal(request.userMessageId, UID); assert.notEqual(request.turnId, UID);
  assert.equal(request.preparationBody().action, 'next');
  assert.equal(request.preparationBody().parent_message_id, UID);
  assert.equal('messages' in request.preparationBody(), false);
  const sent = request.consume({ conduit_token: 'synthetic-conduit' }, { chatReq: { token: 'synthetic-proof' } },
    value => ({ 'openai-sentinel-chat-requirements-token': value.token }), binding.current);
  assert.equal(sent.body.action, 'variant'); assert.equal(sent.body.conversation_id, CID);
  assert.equal(sent.body.parent_message_id, UID); assert.equal(sent.body.variant_purpose, 'comparison_implicit');
  assert.equal(sent.body.enable_message_followups, true); assert.equal('messages' in sent.body, false);
  assert.equal('service_tier' in sent.body, false); assert.equal('one_off_model_override' in sent.body, false);
  assert.throws(() => request.consume({}, {}, () => ({}), binding.current), /preparation_consumed/);
  assert.equal(request.consumeStop([], binding.current).requestBody.conversation_id, CID);
});

test('history requires a new reply observed in this exact owned stream, not merely a new variant', async () => {
  const f = fixture(), binding = await f.capture();
  assert.equal(history.ownsResponse(f.history(PID), binding, UID), false);
  assert.equal(history.ownsResponse(f.history(OTHER), binding, UID), false);
  assert.equal(binding.observePayload({ message: f.parent }), false);
  assert.equal(binding.observePayload({ input_message: { id: OTHER } }), false);
  assert.equal(binding.observePayload({ message: f.reply() }), true);
  assert.equal(history.ownsResponse(f.history(), binding, UID), true);
  assert.equal(history.ownsResponse(f.history(OTHER), binding, UID), false);
  const wrong = f.history(); wrong.mapping[UID].parent = OTHER;
  assert.equal(history.ownsResponse(wrong, binding, UID), false);
  const changed = structuredClone(f.history());
  changed.mapping[UID].message.content.parts = ['Edited elsewhere'];
  assert.equal(history.ownsResponse(changed, binding, UID), false, 'changed prompt is rejected before hydration');
  f.apply(f.history()); assert.equal(binding.reconciled(UID), true);
  f.user.message.metadata.timestamp_ = 12345;
  f.user.message.metadata.system_hints = []; f.user.message.recipient = 'all';
  assert.equal(binding.reconciled(UID), true, 'incidental server metadata does not change the prompt identity');
  f.apply(f.history(OTHER)); assert.equal(binding.reconciled(UID), false);
});

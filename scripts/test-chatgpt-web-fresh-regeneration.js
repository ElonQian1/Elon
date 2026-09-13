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
  ['invalid ancestor', f => { f.user.parent = 'unreviewed-root'; }]
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
  const wrong = f.history(); wrong.mapping[UID] = { ...f.user, parent: OTHER };
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

'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture, UID, PID, AID, OTHER } = require('./fixtures/chatgpt-fresh-regeneration');
const history = require('../android/app/src/main/assets/chatgpt_web_fresh_text_reconcile').create();

async function setup(options = {}) {
  const f = fixture(), binding = await f.capture(), controller = new AbortController(), stages = [];
  binding.observePayload({ message: f.reply() });
  let reads = 0;
  f.conversation.textHydrateHistory = async (_, request) => {
    reads++;
    const payload = f.history();
    options.beforeRead?.(f, payload);
    request.onConversationLoadedFromNetwork(payload);
    options.beforeApply?.(f, controller);
    if (request.shouldApplyResponse()) f.apply(payload, true);
    options.afterApply?.(f, controller);
  };
  return Object.assign(f, { binding, controller, stages, reads: () => reads,
    reconcile: () => history.reconcile(binding, { userMessageId: UID }, controller.signal,
      options.stopped === true, false, stage => stages.push(stage)) });
}

test('history preserving the old sibling selects only the verified owned reply through the official state action', async () => {
  const f = await setup();
  assert.equal(f.tree.leaf, PID);
  assert.equal(await f.reconcile(), true);
  assert.equal(f.tree.leaf, AID);
  assert.deepEqual(f.branchSelections, [AID]);
  assert.equal(f.reads(), 1);
  assert.equal(f.stages.at(-1), 'reconciled');
  assert.equal(f.retry.calls.length, 0);
  assert.equal(Object.values(f.tree.nodes).filter(node => node.message?.author?.role === 'user').length, 1);
  assert.equal(await f.reconcile(), true);
  assert.deepEqual(f.branchSelections, [AID], 'already selected history does not repeat a state transaction');
});

test('an owned stopped partial reply can replace the old completed sibling', async () => {
  const f = await setup({ stopped: true, beforeRead(f, payload) {
    payload.mapping[AID].message = f.reply(AID, 'finished_partial_completion');
  } });
  assert.equal(await f.reconcile(), true);
  assert.deepEqual(f.branchSelections, [AID]);
});

for (const [name, beforeRead, afterApply] of [
  ['unobserved server reply', (f, payload) => Object.assign(payload, f.history(OTHER))],
  ['active server reply', (_, payload) => { payload.async_status = 3; }],
  ['changed original prompt', (_, payload) => { payload.mapping[UID].message = {
    ...payload.mapping[UID].message, content: { content_type: 'text', parts: ['Changed prompt'] } }; }],
  ['aborted request', null, (_, controller) => controller.abort()],
  ['another selected sibling', null, f => { f.tree.nodes[OTHER] = { id: OTHER, parentId: UID, message: f.reply(OTHER) };
    f.tree.leaf = OTHER; }],
  ['account changed', null, f => { f.retry.identity.accountId = OTHER; }],
  ['target not hydrated', null, f => { delete f.tree.nodes[AID]; }],
  ['target no longer terminal', null, f => { f.tree.nodes[AID].message.status = 'in_progress'; }],
  ['target reparented', null, f => { f.tree.nodes[AID].parentId = OTHER; }]
]) test(name + ' cannot select or settle a retry branch', async () => {
  const f = await setup({ beforeRead, afterApply });
  assert.equal(await f.reconcile(), false);
  assert.deepEqual(f.branchSelections, []);
  assert.equal(f.retry.calls.length, 0);
});

test('branch selection rechecks the exact state inside the official transaction', async () => {
  const f = await setup(), update = f.shared.writingUpdateState;
  f.shared.writingUpdateState = (id, change) => {
    f.tree.nodes[OTHER] = { id: OTHER, parentId: UID, message: f.reply(OTHER) };
    f.tree.leaf = OTHER;
    update(id, change);
  };
  assert.equal(await f.reconcile(), false);
  assert.deepEqual(f.branchSelections, []);
  assert.equal(f.tree.leaf, OTHER);
});

test('a user-selected sibling during the read prevents history application, not just later selection', async () => {
  const f = await setup({ beforeApply(f) {
    f.tree.nodes[OTHER] = { id: OTHER, parentId: UID, message: f.reply(OTHER) };
    f.tree.leaf = OTHER;
  } });
  assert.equal(await f.reconcile(), false);
  assert.equal(f.tree.nodes[AID], undefined);
  assert.equal(f.tree.leaf, OTHER);
  assert.deepEqual(f.branchSelections, []);
});

for (const field of ['writingUpdateState', 'writingTreeOwner']) test('missing ' + field + ' rejects before dispatch', async () => {
  const f = fixture(); delete f.shared[field];
  await assert.rejects(f.capture(), /runtime_unavailable/);
  assert.equal(f.retry.calls.length, 0);
});

test('a retired document is rejected before reading its former message tree', async () => {
  const f = await setup();
  f.page.__elonChatGptDocumentToken = 'retired-document';
  f.shared.HM.getCurrentMessage = () => { throw Error('retired tree must not be read'); };
  assert.equal(f.binding.canReconcile(UID), false);
  assert.equal(await f.reconcile(), false);
  assert.equal(f.reads(), 0);
});

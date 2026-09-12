'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const api = require('../android/app/src/main/assets/chatgpt_web_fresh_text_reconcile').create();
const CID = '11111111-1111-4111-8111-111111111111';
const PID = '22222222-2222-4222-8222-222222222222';
const UID = '33333333-3333-4333-8333-333333333333';
const AID = '44444444-4444-4444-8444-444444444444';
function fixture() {
  const payload = { conversation_id: CID, current_node: AID, mapping: {
    [UID]: { id: UID, parent: PID, message: { id: UID, author: { role: 'user' } } },
    [AID]: { id: AID, parent: UID, message: { id: AID, author: { role: 'assistant' },
      status: 'finished_successfully', end_turn: true } }
  } };
  let current = true, applied = false;
  const calls = [], binding = { conversationId: CID, parentId: PID,
    canReconcile: () => current, reconciled: () => applied, runtime: { async textHydrateHistory(id, options) {
      calls.push({ id, options }); options.onConversationLoadedFromNetwork(payload);
      applied = options.shouldApplyResponse();
    } } };
  return { payload, calls, binding, controller: new AbortController(), request: { userMessageId: UID },
    current: value => { current = value; }, applied: () => applied };
}
test('owned completed branch hydrates the existing official store without editor access', async () => {
  const f = fixture();
  assert.equal(await api.reconcile(f.binding, f.request, f.controller.signal), true);
  assert.equal(f.calls.length, 1); assert.equal(f.calls[0].id, CID);
  assert.equal(f.calls[0].options.includeMessageId, UID);
  assert.equal(f.calls[0].options.forceNetworkFetch, true);
  assert.equal(f.calls[0].options.skipIfExisting, false);
});
for (const [name, change] of [
  ['conversation', p => { p.conversation_id = 'another'; }],
  ['user identity', p => { p.mapping[UID].message.id = 'another'; }],
  ['parent', p => { p.mapping[UID].parent = 'another'; }],
  ['sibling leaf', p => { p.mapping[AID].parent = PID; }],
  ['unfinished', p => { p.mapping[AID].message.end_turn = false; }],
  ['subsequent user', p => { p.mapping[AID].message.author.role = 'user'; }],
  ['cycle', p => { p.mapping[AID].parent = AID; }],
  ['missing node', p => { delete p.mapping[UID]; }]
]) test(name + ' does not overwrite website state', async () => {
  const f = fixture(); change(f.payload);
  assert.equal(await api.reconcile(f.binding, f.request, f.controller.signal), false);
  assert.equal(f.applied(), false);
});
test('account/branch change or timeout between fetch and apply rejects a late snapshot', async () => {
  for (const mutate of [f => f.current(false), f => f.controller.abort()]) {
    const f = fixture();
    f.binding.runtime.textHydrateHistory = async (_, options) => {
      options.onConversationLoadedFromNetwork(f.payload); mutate(f);
      assert.equal(options.shouldApplyResponse(), false);
    };
    assert.equal(await api.reconcile(f.binding, f.request, f.controller.signal), false);
  }
});
test('same-owner check happens before any history request', async () => {
  const f = fixture(); f.current(false);
  assert.equal(await api.reconcile(f.binding, f.request, f.controller.signal), false);
  assert.equal(f.calls.length, 0);
});

test('stopped partial history requires an explicit idle server status, not EOF or an omitted field', async () => {
  for (const status of [undefined, 3, 5, null, 4]) {
    const f = fixture(); f.payload.async_status = status;
    f.payload.mapping[AID].message.status = 'finished_partial_completion';
    f.payload.mapping[AID].message.end_turn = false;
    assert.equal(await api.reconcile(f.binding, f.request, f.controller.signal), false);
    assert.equal(await api.reconcile(f.binding, f.request, f.controller.signal, true), status === null || status === 4);
  }
});

test('completed-looking assistant in a still-active server session cannot release the writer', async () => {
  for (const status of [3, 5, 6, 7, '4', {}, undefined]) {
    const f = fixture(); f.payload.async_status = status;
    assert.equal(await api.reconcile(f.binding, f.request, f.controller.signal), false);
    assert.equal(f.applied(), false);
  }
  for (const status of [null, 4]) {
    const f = fixture(); f.payload.async_status = status;
    assert.equal(await api.reconcile(f.binding, f.request, f.controller.signal), true);
  }
});

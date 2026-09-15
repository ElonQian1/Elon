'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture, uid, asset } = require('./fixtures/chatgpt-fresh-text-journal');
const tick = () => new Promise(resolve => setImmediate(resolve));

test('only the POST boundary persists a lease; a fresh process proves history without replay', async () => {
  const f = fixture(), lease = await f.prepare();
  assert.equal(f.storage.length, 0);
  lease.persist(); assert.equal(f.rows().length, 1);
  assert.equal(f.rows()[0].userSignatureHash, null);
  const restarted = f.create();
  assert.deepEqual(await f.prepare(restarted), { recapture: true });
  assert.equal(f.rows().length, 0);
  assert.equal(f.calls.filter(x => x.kind === 'history').length, 1);
  assert.equal(f.calls.filter(x => x.kind === 'apply').length, 1);
  assert.deepEqual(restarted.state(), { code: 'recovered', recovered: 1 });
  assert.equal((await f.prepare(restarted)).recapture, false);
});

test('HTTP acceptance alone does not erase the pending write; confirmed retirement does', async () => {
  const f = fixture(), lease = await f.prepare(); lease.persist();
  assert.equal(f.rows().length, 1);
  assert.equal(lease.settled(), true); assert.equal(f.rows().length, 0);
});

for (const kind of ['wrong_parent', 'sibling', 'not_terminal', 'active', 'temporary', 'wrong_conversation', 'missing_user']) {
  test('recreation does not apply or erase ' + kind + ' history', async () => {
    const f = fixture(); (await f.prepare()).persist();
    if (kind === 'wrong_parent') f.payload.mapping[uid(3)].parent = uid(8);
    if (kind === 'sibling') f.payload.mapping[uid(5)].parent = uid(8);
    if (kind === 'not_terminal') f.payload.mapping[uid(5)].message.end_turn = false;
    if (kind === 'active') f.payload.async_status = 3;
    if (kind === 'temporary') f.payload.is_do_not_remember = true;
    if (kind === 'wrong_conversation') f.payload.conversation_id = uid(8);
    if (kind === 'missing_user') delete f.payload.mapping[uid(3)];
    await assert.rejects(f.prepare(f.create()), /recovery_previous_unresolved/);
    assert.equal(f.calls.some(x => x.kind === 'apply'), false); assert.equal(f.rows().length, 1);
  });
}

test('new conversations adopt server IDs before recreation and retain the first-send parent', async () => {
  const f = fixture();
  Object.assign(f.binding, { conversationId: null, newConversation: true, parentId: 'client-created-root' });
  const lease = await f.prepare(); lease.persist();
  f.binding.conversationId = uid(2); lease.adoptConversation(uid(2));
  assert.equal(f.rows()[0].conversationId, uid(2));
  f.binding.newConversation = false; f.binding.parentId = uid(5);
  f.payload.mapping[uid(3)].parent = 'client-created-root';
  assert.equal((await f.prepare(f.create())).recapture, true);
});

test('an unadopted new write is not silently discarded on the empty home route', async () => {
  const f = fixture();
  Object.assign(f.binding, { conversationId: null, newConversation: true, parentId: 'client-created-root' });
  (await f.prepare()).persist();
  await assert.rejects(f.prepare(f.create()), /recovery_previous_unresolved/);
  assert.equal(f.calls.length, 0); assert.equal(f.rows().length, 1);
});

test('project recovery proves exact project owner and does not consult a composer', async () => {
  const f = fixture();
  f.binding.projectId = 'g-p-' + 'a'.repeat(32);
  f.payload.gizmo_id = f.binding.projectId;
  f.payload.owner = { user_id: 'fixture-user' };
  (await f.prepare()).persist();
  f.payload.owner.user_id = 'other-user';
  await assert.rejects(f.prepare(f.create()), /recovery_previous_unresolved/);
  f.payload.owner.user_id = 'fixture-user';
  assert.equal((await f.prepare(f.create())).recapture, true);
});

test('account switch cannot read, apply or delete the previous account journal', async () => {
  const f = fixture(); (await f.prepare()).persist();
  f.setIdentity(JSON.stringify(['other-user', 'other-account']));
  assert.equal((await f.prepare(f.create())).recapture, false);
  assert.equal(f.calls.length, 0); assert.equal(f.rows().length, 1);
});

test('temporary contexts never touch durable storage, including a denied storage getter', async () => {
  const f = fixture();
  Object.defineProperty(f.page, 'localStorage', { get() { throw Error('must not read'); } });
  for (const key of ['temporary', 'historyDisabled', 'doNotRemember']) {
    f.binding[key] = true; assert.equal(await f.prepare(f.create()), null); f.binding[key] = false;
  }
});

test('timeout aborts only the read and suppresses late callbacks without erasing uncertainty', async () => {
  let late;
  const f = fixture({ timeoutMs: 10, hydrate: (_, value) => { late = value; return new Promise(() => {}); } });
  (await f.prepare()).persist();
  await assert.rejects(f.prepare(f.create()), /recovery_history_timeout/);
  assert.equal(late.signal.aborted, true); assert.equal(f.controller.signal.aborted, false);
  late.onConversationLoadedFromNetwork(f.payload);
  assert.equal(late.shouldApplyResponse(), false); assert.equal(f.rows().length, 1);
});

test('context changes during the history read cannot apply or delete pending ownership', async () => {
  let f;
  f = fixture({ hydrate: (_, value) => {
    f.setCurrent(false); value.onConversationLoadedFromNetwork(f.payload);
    assert.equal(value.shouldApplyResponse(), false);
  } });
  (await f.prepare()).persist();
  await assert.rejects(f.prepare(f.create()), /context_changed/);
  assert.equal(f.rows().length, 1);
});

test('parent cancellation aborts a pending read without waiting for the read timeout', async () => {
  const f = fixture({ hydrate: () => new Promise(() => {}) });
  (await f.prepare()).persist(); const pending = f.prepare(f.create());
  await tick(); f.controller.abort();
  await assert.rejects(pending, /context_changed/); assert.equal(f.rows().length, 1);
});

test('provider read failures remain uncertain history, not local recording failures', async () => {
  const f = fixture({ hydrate: async () => { throw Error('fixture provider failure'); } });
  (await f.prepare()).persist();
  await assert.rejects(f.prepare(f.create()), /recovery_history_unavailable/);
  assert.equal(f.rows().length, 1);
  assert.equal(f.api.state().code.includes('fixture'), false);
});

test('regeneration recovery requires an observed new reply and the original user digest', async () => {
  const f = fixture(), message = f.payload.mapping[uid(3)].message;
  Object.assign(f.binding, { parentId: uid(3), historyParentId: uid(4),
    recoveryUserSignature: () => asset('user_identity').signature(message) });
  const lease = await f.prepare(f.api, 'regenerate'); lease.persist();
  await assert.rejects(f.prepare(f.create()), /recovery_previous_unresolved/);
  lease.observePayload({ message: f.payload.mapping[uid(5)].message });
  assert.equal(f.rows()[0].replyIds[0], uid(5));
  message.content.parts[0] = 'A changed fixture question';
  await assert.rejects(f.prepare(f.create()), /recovery_previous_unresolved/);
  assert.equal(f.calls.some(x => x.kind === 'apply'), false);
  message.content.parts[0] = 'Synthetic fixture question';
  assert.equal((await f.prepare(f.create())).recapture, true);
  assert.equal(f.calls.filter(x => x.kind === 'apply').length, 1);
});

test('stop acknowledgement survives recreation but cannot invent an assistant reply', async () => {
  const f = fixture(), lease = await f.prepare(); lease.persist(); lease.markStop();
  f.payload.current_node = uid(3);
  await assert.rejects(f.prepare(f.create()), /recovery_previous_unresolved/);
  lease.markStop(true);
  assert.equal((await f.prepare(f.create())).recapture, true);
});

test('storage write and cleanup failures remain explicit and retain records', async () => {
  const f = fixture(), lease = await f.prepare();
  const set = f.storage.setItem; f.storage.setItem = () => {};
  assert.throws(() => lease.persist(), /recovery_storage_unavailable/);
  f.storage.setItem = set; lease.persist();
  f.storage.removeItem = () => {};
  assert.equal(lease.settled(), false); assert.equal(f.rows().length, 1);
  await assert.rejects(f.prepare(f.create()), /recovery_storage_unavailable/);
});

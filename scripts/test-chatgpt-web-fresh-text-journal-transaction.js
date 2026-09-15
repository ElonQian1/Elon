'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture } = require('./fixtures/chatgpt-fresh-text-journal-transaction');
const { uid } = require('./fixtures/chatgpt-fresh-text-journal');
const tick = () => new Promise(resolve => setImmediate(resolve));

test('actual independent POST sees the durable record before the first byte can be sent', async () => {
  const f = fixture();
  assert.equal((await f.send().completion).status, 'accepted');
  assert.equal(f.calls.find(x => x.kind === 'post').rows, 1);
  assert.equal(f.rows()[0].userMessageId, f.calls.find(x => x.kind === 'post').body.messages[0].id);
  await tick(); assert.equal(f.rows().length, 1, 'HTTP acceptance is not terminal history');
});

test('production journal default off leaves the accepted writer unchanged and never reads storage', async () => {
  const f = fixture({ enabled: false, reconciled: true });
  Object.defineProperty(f.page, 'localStorage', { get() { throw Error('unexpected storage access'); } });
  assert.equal((await f.send().completion).status, 'accepted');
  assert.equal(f.calls.filter(x => x.kind === 'post').length, 1);
  assert.equal(f.rows().length, 0);
  await tick(); f.api.state(); f.api.dispose();
});

test('storage refusal stops before POST and never offers a second sender', async () => {
  const f = fixture(); f.storage.setItem = () => {};
  const attempt = f.send(), receipt = await attempt.completion;
  assert.deepEqual(receipt, { status: 'rejected', code: 'recovery_storage_unavailable' });
  assert.equal(attempt.claimFallback(), false);
  assert.equal(f.calls.some(x => x.kind === 'post'), false);
  assert.equal(f.draft(), 'Synthetic next prompt');
});

test('a crashed POST blocks the next unsent command until exact provider history is confirmed', async () => {
  const before = fixture({ postError: true });
  assert.equal((await before.send().completion).status, 'unknown'); await tick();
  const after = fixture({ storage: before.storage });
  const result = after.send();
  assert.deepEqual(await result.completion, { status: 'rejected', code: 'recovery_previous_unresolved' });
  assert.equal(result.claimFallback(), false);
  assert.equal(after.calls.some(x => ['post', 'prepare'].includes(x.kind)), false);
  assert.equal(after.draft(), 'Synthetic next prompt');
  assert.equal(after.rows().length, 1);
});

test('confirmed process recovery recaptures the updated parent before preparing the next POST', async () => {
  const before = fixture({ postError: true }); await before.send().completion; await tick();
  const row = before.rows()[0]; let after;
  after = fixture({ storage: before.storage, hydrate: (_, options) => {
    const payload = after.payload;
    payload.mapping[row.userMessageId] = { ...payload.mapping[uid(3)], id: row.userMessageId,
      message: { ...payload.mapping[uid(3)].message, id: row.userMessageId } };
    payload.mapping[uid(5)].parent = row.userMessageId;
    options.onConversationLoadedFromNetwork(payload);
    assert.equal(options.shouldApplyResponse(), true);
    after.binding.parentId = uid(5);
  } });
  assert.equal((await after.send().completion).status, 'accepted');
  assert.equal(after.captures(), 2);
  const posts = after.calls.filter(x => x.kind === 'post');
  assert.equal(posts.length, 1); assert.equal(posts[0].body.parent_message_id, uid(5));
  assert.notEqual(posts[0].body.messages[0].id, row.userMessageId);
  assert.equal(after.rows().length, 1);
  assert.equal(after.rows()[0].userMessageId, posts[0].body.messages[0].id);
});

test('only verified reconciliation retires the durable record', async () => {
  const f = fixture({ reconciled: true }); await f.send().completion;
  await tick(); await tick(); f.api.state();
  assert.equal(f.rows().length, 0); assert.equal(f.api.state().pending, false); f.api.dispose();
});

test('an account switch while preparing recovery rejects without a POST or draft clear', async () => {
  const f = fixture(); f.binding.recoveryIdentity = () => null;
  assert.deepEqual(await f.send().completion, { status: 'rejected', code: 'recovery_identity_unavailable' });
  assert.equal(f.calls.some(x => x.kind === 'post'), false);
  assert.equal(f.draft(), 'Synthetic next prompt');
});

test('a cold or missing owner cannot bypass the journal through the legacy sender', async () => {
  const before = fixture({ postError: true }); await before.send().completion; await tick();
  for (const options of [{ noIdentity: true }, { captureError: true }]) {
    const after = fixture({ ...options, storage: before.storage });
    const transaction = after.send();
    assert.equal(transaction.handled, true);
    assert.equal((await transaction.completion).status, 'rejected');
    assert.notEqual(transaction.claimFallback?.(), true);
    assert.equal(after.calls.some(x => ['post', 'prepare'].includes(x.kind)), false);
    assert.equal(after.rows().length, 1);
  }
});

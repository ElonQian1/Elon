'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture } = require('./fixtures/chatgpt-fresh-text-journal-transaction');
const { uid } = require('./fixtures/chatgpt-fresh-text-journal');
const tick = () => new Promise(resolve => setImmediate(resolve));

async function crashed() {
  const f = fixture({ postError: true });
  await f.send().completion; await tick();
  assert.equal(f.rows().length, 1); f.api.dispose();
  return f;
}

function confirm(f, row, options) {
  const payload = f.payload;
  payload.mapping[row.userMessageId] = { ...payload.mapping[uid(3)], id: row.userMessageId,
    message: { ...payload.mapping[uid(3)].message, id: row.userMessageId } };
  payload.mapping[uid(5)].parent = row.userMessageId;
  options.onConversationLoadedFromNetwork(payload);
  assert.equal(options.shouldApplyResponse(), true);
  f.binding.parentId = uid(5);
}

test('recreated sender reads old terminal history before a missing parent can block new-send capture', async () => {
  const before = await crashed(), row = before.rows()[0];
  let ready = false, after;
  after = fixture({ storage: before.storage, captureReady: () => ready,
    hydrate: (_, options) => { confirm(after, row, options); ready = true; } });
  try {
    assert.equal((await after.send().completion).status, 'accepted');
    assert.equal(after.calls[0].kind, 'history');
    assert.equal(after.captures(), 1);
    const posts = after.calls.filter(call => call.kind === 'post');
    assert.equal(posts.length, 1);
    assert.equal(posts[0].body.parent_message_id, uid(5));
    assert.notEqual(posts[0].body.messages[0].id, row.userMessageId);
  } finally { after.api.dispose(); }
});

test('unconfirmed old history blocks before touching a broken composer and keeps the next draft', async () => {
  const before = await crashed(), after = fixture({ storage: before.storage, captureError: true });
  try {
    const sent = after.send();
    assert.deepEqual(await sent.completion, { status: 'rejected', code: 'recovery_previous_unresolved' });
    assert.equal(after.captures(), 0);
    assert.equal(after.calls.filter(call => call.kind === 'history').length, 1);
    assert.equal(after.calls.some(call => ['prepare', 'post'].includes(call.kind)), false);
    assert.equal(sent.claimFallback(), false);
    assert.equal(after.draft(), 'Synthetic next prompt');
    assert.equal(after.rows().length, 1);
  } finally { after.api.dispose(); }
});

test('an empty journal never hydrates history or bypasses normal sender admission', async () => {
  const f = fixture({ captureError: true });
  try {
    assert.equal((await f.send().completion).code, 'context_unavailable');
    assert.equal(f.captures(), 1);
    assert.equal(f.calls.length, 0);
    assert.equal(f.rows().length, 0);
  } finally { f.api.dispose(); }
});

test('enabled recovery preserves memory-owned sends whose DOM draft reader is unavailable', async () => {
  const f = fixture();
  try {
    const sent = f.api.send({ requestId: 'mcp_recovered1', prompt: 'Synthetic next prompt', expectedDraft: '',
      readDraft() { throw Error('unexpected_dom_reader'); } });
    assert.equal((await sent.completion).status, 'accepted');
    assert.equal(f.calls.filter(call => call.kind === 'post').length, 1);
  } finally { f.api.dispose(); }
});

test('page snapshot restores old history even when new-send capture cannot run', async () => {
  const before = await crashed(), row = before.rows()[0]; let after;
  after = fixture({ storage: before.storage, captureError: true,
    hydrate: (_, options) => confirm(after, row, options) });
  try {
    after.api.pendingRecoverySnapshot();
    for (let i = 0; i < 100 && after.rows().length; i++) await tick();
    assert.equal(after.rows().length, 0);
    assert.equal(after.api.pendingRecoverySnapshot(), 'recovered');
    assert.equal(after.captures(), 0);
    assert.equal(after.calls.some(call => ['prepare', 'post'].includes(call.kind)), false);
  } finally { after.api.dispose(); }
});

test('real transaction cancels a passive read before claiming a new send', async () => {
  const before = await crashed(); let oldOptions, finish, reads = 0;
  const after = fixture({ storage: before.storage, hydrate: (_, options) => {
    if (++reads === 1) { oldOptions = options; return new Promise(done => { finish = done; }); }
  } });
  try {
    after.api.pendingRecoverySnapshot();
    for (let i = 0; i < 100 && !oldOptions; i++) await tick();
    assert.ok(oldOptions);
    const sent = after.send();
    assert.equal(oldOptions.signal.aborted, true);
    oldOptions.onConversationLoadedFromNetwork(after.payload);
    assert.equal(oldOptions.shouldApplyResponse(), false);
    finish();
    assert.equal((await sent.completion).code, 'recovery_previous_unresolved');
    assert.equal(after.calls.some(call => call.kind === 'post'), false);
    assert.equal(after.rows().length, 1);
  } finally { after.api.dispose(); }
});

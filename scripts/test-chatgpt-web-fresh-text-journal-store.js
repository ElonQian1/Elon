'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const api = require('../android/app/src/main/assets/chatgpt_web_fresh_text_journal_store.js');
const uid = value => '10000000-0000-4000-8000-' + String(value).padStart(12, '0');
const account = 'a'.repeat(64), other = 'b'.repeat(64);
function memory() {
  const values = new Map();
  return { values, get length() { return values.size; }, key: index => [...values.keys()][index] ?? null,
    getItem: key => values.get(key) ?? null, setItem: (key, value) => values.set(key, value), removeItem: key => values.delete(key) };
}
function record(overrides = {}) {
  return { version: 1, accountHash: account, attemptId: uid(1), conversationId: uid(2),
    userMessageId: uid(3), parentId: uid(4), projectId: null, newConversation: false,
    operation: 'send', historyParentId: null, userSignatureHash: null, replyIds: [], stopAttempted: false,
    stopAcknowledged: false, createdAtMs: 1000, ...overrides };
}

test('fresh instance restores metadata without a request body or session credentials', () => {
  const storage = memory(), first = api.create(storage);
  first.write(record());
  const second = api.create(storage);
  assert.deepEqual(second.list(account), [record()]);
  assert.deepEqual(second.list(other), []);
  assert.equal(first.write(record()).attemptId, uid(1));
  assert.equal(storage.length, 1);
  assert.equal([...storage.values.values()].some(value => /prompt|cookie|token|header|draft|audio/i.test(value)), false);
});

test('unknown data, temporary-chat data and malformed identifiers cannot be persisted', () => {
  for (const patch of [{ prompt: 'fixture secret' }, { temporary: true }, { token: 'fixture token' },
    { accountHash: 'raw-account' }, { conversationId: 'https://example.test' }, { attemptId: null },
    { accountHash: [account] }, { attemptId: [uid(1)] }, { parentId: [uid(4)] },
    { parentId: 'unknown' }, { newConversation: true }, { replyIds: [uid(5), uid(5)] }, { replyIds: Array(1) },
    { stopAcknowledged: true }, { createdAtMs: NaN }, { historyParentId: uid(4) }]) {
    const storage = memory();
    assert.throws(() => api.create(storage).write(record(patch)), /recovery_record_invalid/);
    assert.equal(storage.length, 0);
  }
});

test('new conversation adopts its server identity without changing attempt ownership', () => {
  const store = api.create(memory());
  const saved = store.write(record({ conversationId: null, newConversation: true, parentId: 'client-created-root' }));
  const updated = store.update(saved, { conversationId: uid(9) });
  assert.equal(updated.conversationId, uid(9));
  assert.throws(() => store.update(updated, { conversationId: uid(8) }), /recovery_record_invalid/);
  assert.throws(() => store.update(updated, { accountHash: other }), /recovery_record_invalid/);
  assert.throws(() => store.remove(saved), /recovery_record_changed/);
  assert.equal(store.remove(updated), true);
  assert.equal(store.remove(updated), true);
});

test('observed reply and stop evidence only move forward', () => {
  const store = api.create(memory());
  const first = store.write(record());
  const second = store.update(first, { replyIds: [uid(5)], stopAttempted: true });
  assert.throws(() => store.update(first, { replyIds: [uid(6)] }), /recovery_record_changed/);
  assert.throws(() => store.update(second, { replyIds: [] }), /recovery_record_invalid/);
  assert.throws(() => store.update(second, { stopAttempted: false }), /recovery_record_invalid/);
  const last = store.update(second, { stopAcknowledged: true });
  assert.equal(store.list(account)[0].stopAcknowledged, true);
  assert.equal(Object.isFrozen(last.replyIds), true);
});

test('capacity retains unknown writes instead of evicting or expiring them', () => {
  const storage = memory(), store = api.create(storage);
  for (let i = 1; i <= 16; i++) store.write(record({ attemptId: uid(i), createdAtMs: 0 }));
  assert.throws(() => store.write(record({ attemptId: uid(17) })), /recovery_capacity/);
  assert.equal(store.list(account).length, 16);
  store.write(record({ accountHash: other, attemptId: uid(17) }));
  assert.equal(store.list(other).length, 1);
});

test('corrupt owner records fail closed without touching another owner or provider storage', () => {
  const storage = memory(), store = api.create(storage);
  store.write(record());
  const name = [...storage.values.keys()][0];
  storage.setItem(name, '{broken');
  storage.setItem('official-preference', 'preserve');
  assert.throws(() => store.list(account), /recovery_record_invalid/);
  assert.deepEqual(store.list(other), []);
  assert.equal(storage.getItem(name), '{broken');
  assert.equal(storage.getItem('official-preference'), 'preserve');
});

test('moved record keys cannot spoof account or attempt ownership', () => {
  const storage = memory(), store = api.create(storage);
  store.write(record());
  storage.setItem('elon.fresh.pending.v1.' + other + '.' + uid(1), JSON.stringify(record()));
  assert.throws(() => store.list(other), /recovery_record_invalid/);
  assert.equal(store.list(account).length, 1);
});

test('storage failures and silent quota refusal are not successful persistence', () => {
  assert.throws(() => api.create(null).list(account), /recovery_storage_unavailable/);
  const storage = memory();
  storage.setItem = () => {};
  assert.throws(() => api.create(storage).write(record()), /recovery_storage_unavailable/);
});

test('regeneration records preserve their user and original parent identities', () => {
  const store = api.create(memory());
  const saved = store.write(record({ operation: 'regenerate', parentId: uid(3), historyParentId: uid(4), userSignatureHash: other }));
  assert.equal(saved.userMessageId, saved.parentId);
  assert.throws(() => store.write(record({ operation: 'regenerate', attemptId: uid(9) })), /recovery_record_invalid/);
  assert.throws(() => store.write(record({ operation: 'regenerate', attemptId: uid(9), parentId: uid(3),
    historyParentId: uid(4), projectId: 'g-p-' + 'a'.repeat(32) })), /recovery_record_invalid/);
});

test('version 2 requires an immutable exact attachment digest and retains version 1 records', () => {
  const store = api.create(memory());
  store.write(record());
  const next = store.write(record({ version: 2, attemptId: uid(8), attachmentSignatureHash: other }));
  assert.equal(store.list(account).length, 2);
  assert.equal(next.attachmentSignatureHash, other);
  assert.throws(() => store.update(next, { attachmentSignatureHash: account }), /recovery_record_invalid/);
  for (const patch of [{ version: 2 }, { version: 3 }, { version: 2, attachmentSignatureHash: 'file-private' },
    { version: 2, attachmentSignatureHash: [other] }, { version: 1, attachmentSignatureHash: null }]) {
    assert.throws(() => store.write(record({ attemptId: uid(9), ...patch })), /recovery_record_invalid/);
  }
});

'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const receipts = require('../android/app/src/main/assets/chatgpt_web_fresh_text_receipts');
const id = value => 'mcp_' + BigInt(value).toString(36);
const complete = () => ({ retirable: () => true });

test('long resident pages retain 32 receipts, not a 32-command lifetime limit', () => {
  const ledger = receipts.create();
  for (let i = 1; i <= 10000; i++) {
    assert.equal(ledger.admit(id(i)), ''); ledger.set(id(i), complete());
    assert.ok(ledger.size() <= 32);
  }
  assert.equal(ledger.size(), 32); assert.equal(ledger.attempts(), 10000);
  assert.equal(ledger.get(id(1)), undefined);
  assert.equal(ledger.retired(id(1)), true);
  assert.equal(ledger.admit(id(1)), 'request_retired');
  assert.equal(ledger.admit(id(9999)), 'request_id_conflict');
});

test('uncertain records are never evicted and capacity rejection is explicit', () => {
  const ledger = receipts.create(2), pending = { retirable: () => false };
  ledger.set(id(1), pending); ledger.set(id(2), pending);
  assert.equal(ledger.admit(id(3)), 'receipt_capacity');
  assert.equal(ledger.get(id(1)), pending); assert.equal(ledger.get(id(2)), pending);
  assert.equal(ledger.retired(id(1)), false);
});

test('retired range may cover a retained uncertain record but never discards it', () => {
  const ledger = receipts.create(2), pending = { retirable: () => false };
  ledger.set(id(1), pending); ledger.set(id(2), complete());
  assert.equal(ledger.admit(id(3)), ''); ledger.set(id(3), complete());
  assert.equal(ledger.get(id(1)), pending); assert.equal(ledger.size(), 2);
  assert.equal(ledger.admit(id(2)), 'request_retired');
});

test('Long command sequences stay exact above JavaScript integer precision', () => {
  const ledger = receipts.create(1), first = 9007199254740992n;
  ledger.set(id(first), complete());
  assert.equal(ledger.admit(id(first + 1n)), ''); ledger.set(id(first + 1n), complete());
  assert.equal(ledger.admit(id(first)), 'request_retired');
  assert.equal(ledger.admit(id(first + 2n)), '');
  assert.equal(ledger.admit(id(9223372036854775808n)), 'invalid_command');
});

test('invalid and noncanonical IDs cannot bypass a retired range', () => {
  const ledger = receipts.create();
  for (const value of [null, '', 'mcp_0', 'mcp_01', 'mcp_A', 'mcp_a_b', 'mcp_' + 'z'.repeat(32)]) {
    assert.equal(ledger.admit(value), 'invalid_command');
  }
  assert.throws(() => receipts.create(33), /receipt_limit_invalid/);
});

test('document reset clears retained data and the matching retired range together', () => {
  const ledger = receipts.create(1);
  ledger.set(id(1), complete()); ledger.admit(id(2)); ledger.set(id(2), complete());
  ledger.clear();
  assert.equal(ledger.size(), 0); assert.equal(ledger.attempts(), 0);
  assert.equal(ledger.admit(id(1)), '');
});

test('diagnostic counter saturates but does not block subsequent commands', () => {
  const ledger = receipts.create(1);
  for (let i = 1; i <= 65537; i++) {
    assert.equal(ledger.admit(id(i)), ''); ledger.set(id(i), complete());
  }
  assert.equal(ledger.attempts(), 65535); assert.equal(ledger.size(), 1);
});

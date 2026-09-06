'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');
const { evaluateLedgerBuffer, MAX_INPUT_BYTES } = require('../preview');
const { batch, encode, evaluate, U128_MAX, assertInvalid } = require('./helpers');

test('only bounded Buffer inputs are accepted, including at exactly one MiB', () => {
  for (const value of [null, undefined, '{}', batch(), new Uint8Array([123, 125])]) {
    assertInvalid(value, 'INVALID_INPUT');
  }
  assert.equal(MAX_INPUT_BYTES, 1048576);
  const input = encode(batch());
  const exact = Buffer.concat([input, Buffer.alloc(MAX_INPUT_BYTES - input.length, 0x20)]);
  assert.deepEqual(evaluateLedgerBuffer(exact), evaluateLedgerBuffer(input));
  assertInvalid(Buffer.concat([exact, Buffer.from(' ')]), 'INPUT_TOO_LARGE');
});

test('invalid UTF-8 and BOM cannot silently normalize untrusted documents', () => {
  for (const bytes of [[0xc0, 0xaf], [0x80], [0xed, 0xa0, 0x80], [0xe2, 0x82]]) {
    assertInvalid(Buffer.from(bytes), 'INVALID_UTF8');
  }
  assertInvalid(Buffer.concat([Buffer.from([0xef, 0xbb, 0xbf]), encode(batch())]), 'INVALID_UTF8');
});

test('duplicate keys, including escaped equivalents in nested documents, are rejected', () => {
  const raw = JSON.stringify(batch());
  assertInvalid(Buffer.from(raw.replace('{', '{"schema":"other",')), 'DUPLICATE_JSON_KEY');
  assertInvalid(Buffer.from(raw.replace('{', '{"sch\\u0065ma":"other",')), 'DUPLICATE_JSON_KEY');
  assertInvalid(Buffer.from(raw.replace('"paid_reconciliation":{',
    '"paid_reconciliation":{"batch_id":"injected",')), 'DUPLICATE_JSON_KEY');
  assertInvalid(Buffer.from(raw.replace('"funding_lots":[{',
    '"funding_lots":[{"lot_id":"injected",')), 'DUPLICATE_JSON_KEY');
});

test('dangerous keys and lone surrogate values are rejected at all input depths', () => {
  const raw = JSON.stringify(batch());
  for (const key of ['__proto__', '__pr\\u006fto__', 'constructor', 'prototype']) {
    assertInvalid(Buffer.from(raw.replace('{', `{"${key}":{},`)), 'UNSAFE_KEY');
    assertInvalid(Buffer.from(raw.replace('"funding_lots":[{',
      `"funding_lots":[{"${key}":{},`)), 'UNSAFE_KEY');
  }
  for (const value of ['\ud800', '\udfff', 'a\ud800b']) {
    const input = batch();
    input.policy_draft.decisions.funding_description = value;
    assertInvalid(encode(input), 'INVALID_VALUE');
  }
  assert.equal({}.polluted, undefined);
});

test('malformed JSON and excessive depth produce stable parser failures', () => {
  for (const raw of ['', ' ', '{', '{}{}', '{"a":NaN}', '{"a":Infinity}',
    '{"a":1e309}', '{"a":-0}', '{"a":2.0}', '{"a":9007199254740993}']) {
    assertInvalid(Buffer.from(raw), 'INVALID_JSON');
  }
  assertInvalid(Buffer.from('['.repeat(14) + 'null' + ']'.repeat(14)), 'INPUT_TOO_DEEP');
  for (const value of [null, [], true, 'ledger', 1]) assertInvalid(encode(value), 'INVALID_STRUCTURE');
});

test('amounts must be positive exact decimal strings bounded by u128', () => {
  for (const value of ['0', '-1', '+1', '01', '1.0', '1e3', '1\n', ' 1',
    (U128_MAX + 1n).toString(), '9'.repeat(40), 100, null]) {
    for (const target of ['lot', 'event']) {
      const input = batch();
      (target === 'lot' ? input.funding_lots[0] : input.journal[0]).amount_base_units = value;
      assertInvalid(encode(input), 'INVALID_AMOUNT');
    }
  }
});

test('identifiers, digests and payment references reject trailing newline and Unicode controls', () => {
  for (const suffix of ['\n', '\r', '\u2028', '\u2029', '\u0085', '\ud800']) {
    for (const target of ['lot_id', 'event_id', 'idempotency_key', 'policy_digest', 'external_payment_reference']) {
      const input = batch();
      const object = target === 'policy_digest' ? input.obligation_links[0]
        : ['event_id', 'idempotency_key'].includes(target) ? input.journal[0] : input.funding_lots[0];
      object[target] += suffix;
      assertInvalid(encode(input), 'INVALID_VALUE');
    }
  }
  const tooLong = batch();
  tooLong.funding_lots[0].lot_id = 'x'.repeat(81);
  assertInvalid(encode(tooLong), 'INVALID_VALUE');
  const longKey = batch();
  longKey.journal[0].idempotency_key = 'x'.repeat(129);
  assertInvalid(encode(longKey), 'INVALID_VALUE');
});

test('indices and event sequences require bounded integers with no lexical coercion', () => {
  for (const [field, values] of [['transfer_index', [-1, 2147483648, '0', null]],
    ['sequence', [0, 501, '1', null]]]) {
    for (const value of values) {
      const input = batch();
      (field === 'sequence' ? input.journal[0] : input.funding_lots[0])[field] = value;
      assertInvalid(encode(input), 'INVALID_VALUE');
    }
  }
  const raw = JSON.stringify(batch());
  for (const token of ['0.5', '0e0', '-0']) {
    assertInvalid(Buffer.from(raw.replace('"transfer_index":0', `"transfer_index":${token}`)), 'INVALID_JSON');
  }
});

test('unknown or missing fields and oversized collections fail before producing partial totals', () => {
  for (const select of [input => input, input => input.funding_lots[0],
    input => input.obligation_links[0], input => input.journal[0]]) {
    const input = batch();
    const object = select(input);
    object.currency = 'USD';
    assertInvalid(encode(input), 'INVALID_STRUCTURE');
    delete object.currency;
    delete object[Object.keys(object)[0]];
    assertInvalid(encode(input), 'INVALID_STRUCTURE');
  }
  for (const [field, count] of [['funding_lots', 201], ['obligation_links', 201], ['journal', 501]]) {
    const input = batch();
    input[field] = Array.from({ length: count }, () => input[field][0]);
    assertInvalid(encode(input), 'INVALID_STRUCTURE');
  }
});

test('nested policy and paid documents remain subject to their existing contracts', () => {
  const policy = batch();
  policy.policy_draft.confirmed_intent.policy_duration_years = 3;
  assertInvalid(encode(policy), 'INVALID_POLICY_DRAFT');
  const paid = batch();
  paid.paid_reconciliation.source.decimals = 19;
  assertInvalid(encode(paid), 'INVALID_PAID_RECONCILIATION');
  const override = batch();
  override.funding_lots[0].asset_symbol = 'BTC';
  assertInvalid(encode(override), 'INVALID_STRUCTURE');
});

test('canonical reports are order-stable, content-bound, isolated, and do not mutate input bytes', () => {
  const input = batch();
  const buffer = encode(input);
  const before = Buffer.from(buffer);
  const baseline = evaluateLedgerBuffer(buffer);
  assert.deepEqual(buffer, before);
  const reversed = Object.fromEntries(Object.entries(input).reverse());
  reversed.funding_lots = input.funding_lots.map(lot => Object.fromEntries(Object.entries(lot).reverse()));
  assert.deepEqual(evaluateLedgerBuffer(Buffer.from(JSON.stringify(reversed, null, 2))), baseline);
  input.journal[3].amount_base_units = '11';
  assert.notEqual(evaluate(input).input_digest, baseline.input_digest);
  baseline.issues.push('INJECTED');
  baseline.policy_missing_decisions.length = 0;
  assert.deepEqual(evaluateLedgerBuffer(buffer).issues, []);
  assert.equal(evaluateLedgerBuffer(buffer).policy_missing_decisions.length, 14);
});

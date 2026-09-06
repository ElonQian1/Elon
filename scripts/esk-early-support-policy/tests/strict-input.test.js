'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');
const { evaluatePolicyBuffer, MAX_INPUT_BYTES } = require('../contract');
const { fixture, encode, assertInvalid } = require('./helpers');

test('input must be a Buffer, including when an object resembles a valid draft', () => {
  for (const input of [undefined, null, fixture(), '{}', new Uint8Array([123, 125])]) {
    assertInvalid(input, 'INVALID_INPUT');
  }
});

test('the 64 KiB byte limit accepts the exact boundary and rejects one byte more', () => {
  assert.equal(MAX_INPUT_BYTES, 65536);
  const base = encode(fixture());
  const exact = Buffer.concat([base, Buffer.alloc(MAX_INPUT_BYTES - base.length, 0x20)]);
  assert.equal(evaluatePolicyBuffer(exact).input_digest, evaluatePolicyBuffer(base).input_digest);
  assertInvalid(Buffer.concat([exact, Buffer.from(' ')]), 'INPUT_TOO_LARGE');
});

test('malformed UTF-8 and BOM are rejected before decoding can replace bytes', () => {
  for (const bytes of [[0xc0, 0xaf], [0xed, 0xa0, 0x80], [0x80], [0xe2, 0x82]]) {
    assertInvalid(Buffer.from(bytes), 'INVALID_UTF8');
  }
  assertInvalid(Buffer.concat([Buffer.from([0xef, 0xbb, 0xbf]), encode(fixture())]), 'INVALID_UTF8');
});

test('repeated keys and escaped equivalents cannot silently select a new policy', () => {
  const raw = JSON.stringify(fixture());
  for (const duplicate of ['"schema"', '"sch\\u0065ma"']) {
    const input = raw.replace('{', `{${duplicate}:"other",`);
    assertInvalid(Buffer.from(input), 'DUPLICATE_JSON_KEY');
  }
  const nested = raw.replace('"decisions":{', '"decisions":{"protection_scope":null,');
  assertInvalid(Buffer.from(nested), 'DUPLICATE_JSON_KEY');
});

test('prototype-related keys are refused at every object depth without polluting objects', () => {
  const raw = JSON.stringify(fixture());
  for (const key of ['__proto__', 'constructor', 'prototype', '__pr\\u006fto__']) {
    assertInvalid(Buffer.from(raw.replace('{', `{"${key}":{"polluted":true},`)), 'UNSAFE_KEY');
    const nested = raw.replace('"decisions":{', `"decisions":{"${key}":{},`);
    assertInvalid(Buffer.from(nested), 'UNSAFE_KEY');
  }
  assert.equal({}.polluted, undefined);
});

test('empty, malformed and non-integral numeric JSON is rejected consistently', () => {
  for (const raw of ['', ' ', '{', '{}{}', '{"a":}', '{"a":1,}', '{"a":NaN}', '{"a":Infinity}',
    '{"a":1e309}', '{"a":2.0}', '{"a":9007199254740993}', '{"a":-0}', '{"a":"\\x20"}']) {
    assertInvalid(Buffer.from(raw), 'INVALID_JSON');
  }
});

test('JSON roots cannot be replaced with scalars or arrays', () => {
  for (const value of [null, true, 1, 'draft', [], [{}]]) {
    assertInvalid(encode(value), 'INVALID_STRUCTURE');
  }
});

test('excessive nesting is bounded even when the draft shape is already wrong', () => {
  assertInvalid(Buffer.from('['.repeat(14) + 'null' + ']'.repeat(14)), 'INPUT_TOO_DEEP');
});

test('escaped lone surrogates are invalid values rather than lossy canonical text', () => {
  for (const value of ['\ud800', '\udfff', 'x\ud800z']) {
    const input = fixture();
    input.decisions.funding_description = value;
    assertInvalid(encode(input), 'INVALID_VALUE');
  }
});

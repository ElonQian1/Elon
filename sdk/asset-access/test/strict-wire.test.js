import test from 'node:test';
import assert from 'node:assert/strict';
import { decodeStrictJson } from '../src/strict-json.js';
import { fixture, clone, reply, rawReply, environment, authorize } from './hardening-helper.js';

const bytes = value => new TextEncoder().encode(value);
const parse = raw => decodeStrictJson(bytes(raw));

test('duplicate escaped keys and nested balance keys fail instead of selecting a later value', async () => {
  const identity = JSON.stringify(fixture.valid.identity);
  for (const spelling of ['subject', 'su\\u0062ject']) {
    const env = environment();
    await authorize(env);
    env.queue.push(rawReply(identity.replace('{', `{"${spelling}":"wrong-subject",`)));
    await assert.rejects(env.client.identity(), { code: 'invalid_response' });
    assert.equal(env.client.state.status, 'unauthenticated');
  }
  const env = environment();
  await authorize(env);
  env.queue.push(reply(fixture.valid.asset_zero));
  await env.client.readAssets();
  const nested = JSON.stringify(fixture.valid.asset_zero)
    .replace('"balance":{', '"balance":{"total_base_units":"99",');
  env.queue.push(rawReply(nested));
  await assert.rejects(env.client.readAssets(), { code: 'invalid_response' });
  assert.equal(env.client.state.has_snapshot, false);
});

test('ambiguous token and revocation responses also fail closed', async () => {
  const tokenEnv = environment();
  const ambiguousToken = JSON.stringify(fixture.valid.token)
    .replace('{', '{"access_token":"aat_' + 'f'.repeat(64) + '",');
  await assert.rejects(authorize(tokenEnv, {}, rawReply(ambiguousToken)), { code: 'invalid_response' });
  assert.equal(tokenEnv.client.state.status, 'unauthenticated');
  const revokeEnv = environment();
  await authorize(revokeEnv);
  const ambiguousRevoke = JSON.stringify(fixture.valid.revoked).replace('{', '{"revoked":false,');
  revokeEnv.queue.push(rawReply(ambiguousRevoke));
  await assert.rejects(revokeEnv.client.revoke(), { code: 'invalid_response' });
  assert.equal(revokeEnv.client.state.status, 'unauthenticated');
});

test('dangerous object keys are rejected before any prototype-sensitive operation', () => {
  for (const key of ['__proto__', '__pr\\u006fto__', 'constructor', 'prototype']) {
    assert.throws(() => parse(`{"${key}":{"polluted":true}}`), { code: 'invalid_response' });
    assert.throws(() => parse(`{"nested":[{"${key}":{}}]}`), { code: 'invalid_response' });
  }
  assert.equal({}.polluted, undefined);
});

test('BOM and malformed UTF-8 fail through the real response transport', async () => {
  const identity = bytes(JSON.stringify(fixture.valid.identity));
  const bom = new Uint8Array(identity.length + 3);
  bom.set([0xef, 0xbb, 0xbf]); bom.set(identity, 3);
  for (const invalid of [bom, new Uint8Array([0xc0, 0xaf]), new Uint8Array([0xed, 0xa0, 0x80]),
    new Uint8Array([0xe2, 0x82])]) {
    const env = environment();
    await authorize(env);
    env.queue.push(rawReply(invalid));
    await assert.rejects(env.client.identity(), { code: 'invalid_response' });
    assert.equal(env.client.state.status, 'unauthenticated');
  }
});

test('lone surrogate keys and values fail while paired emoji and ordinary Unicode remain valid', async () => {
  for (const raw of ['"\\ud800"', '"\\udfff"', '{"\\ud800":1}', '{"name":"x\\udfffy"}']) {
    assert.throws(() => parse(raw), { code: 'invalid_response' });
  }
  assert.equal(parse('"\\ud83e\\uddea"'), '🧪');
  const env = environment();
  await authorize(env);
  env.queue.push(reply({ ...clone(fixture.valid.identity), nickname: '中文与 🧪' }));
  assert.equal((await env.client.identity()).nickname, '中文与 🧪');
  env.client.clear();
});

test('depth is limited to 32 containers and braces inside strings do not consume it', () => {
  const valid = '['.repeat(32) + '0' + ']'.repeat(32);
  let value = parse(valid);
  for (let depth = 0; depth < 32; depth += 1) value = value[0];
  assert.equal(value, 0);
  assert.throws(() => parse('['.repeat(33) + '0' + ']'.repeat(33)), { code: 'invalid_response' });
  const mixed = depth => {
    let raw = '0';
    for (let index = 0; index < depth; index += 1) raw = index % 2 ? `{"nested":${raw}}` : `[${raw}]`;
    return raw;
  };
  assert.doesNotThrow(() => parse(mixed(32)));
  assert.throws(() => parse(mixed(33)), { code: 'invalid_response' });
  assert.equal(parse(JSON.stringify('{'.repeat(100))), '{'.repeat(100));
});

test('normal JSON numeric syntax remains supported and malformed or non-finite values do not', () => {
  assert.deepEqual(parse('{"fraction":1.5,"exponent":2e3,"negative":-2}'),
    { fraction: 1.5, exponent: 2000, negative: -2 });
  assert.equal(Object.is(parse('-0'), -0), true);
  for (const raw of ['', '{}{}', '{"a":1,}', '[1,]', '{"a":NaN}', '{"a":Infinity}', '1e999', '01']) {
    assert.throws(() => parse(raw), { code: 'invalid_response' });
  }
});

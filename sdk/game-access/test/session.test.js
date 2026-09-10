import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { verifyObservation } from '../src/verify.js';
import { grantValues, hex, identifier, units } from '../src/contract.js';
import { signedFixture } from './fixture-builder.js';

function check(f, overrides = {}) {
  return verifyObservation({ expectedChallenge:f.expected_challenge, observation:f.observation,
    mainIssuer:f.main_issuer, keyId:f.key_id, publicKeyHex:f.public_key_hex, nowMs:f.now_ms, ...overrides });
}

test('fixed public vector is reproducible and yields the same evidence digest', () => {
  const f = JSON.parse(readFileSync(new URL('./fixtures/session-v1.json', import.meta.url)));
  assert.deepEqual(f, signedFixture());
  const result = check(f);
  assert.equal(result.authorization_digest, f.authorization_digest);
  assert.equal(result.main_user_id, 'synthetic-user-alice');
  assert.equal(result.wallet_bound, false);
  assert.equal(result.funds_moved, false);
});

test('nonce, credential digest and action cannot be replayed into a different request', () => {
  for (const mutate of [
    c => { c.nonce = '12'.repeat(32); }, c => { c.credential_digest = '34'.repeat(32); },
    c => { c.action.idempotency_key = 'other-request'; }, c => { c.action.quote_id = 'other-quote'; },
  ]) {
    const f = signedFixture(); mutate(f.expected_challenge);
    assert.throws(() => check(f), /challenge_mismatch/);
  }
});

test('user, grant revision and session substitution invalidate the signature', () => {
  for (const field of ['main_user_id', 'main_session_id', 'grant_id', 'revision']) {
    const f = signedFixture();
    f.observation.grant[field] = field === 'revision' ? '3' : 'other-subject';
    assert.throws(() => check(f), /signature_invalid/);
  }
});

test('unknown key and issuer are never learned from the reply', () => {
  assert.throws(() => check(signedFixture(), { keyId:'unknown-key' }), /authority_mismatch/);
  assert.throws(() => check(signedFixture(), { mainIssuer:'other-main' }), /authority_mismatch/);
  assert.throws(() => check(signedFixture(), { publicKeyHex:'00'.repeat(32) }), /signature_invalid/);
});

test('expiry is strict at the observation and grant boundary', () => {
  assert.throws(() => check(signedFixture(), { nowMs:'1700000011000' }), /observation_expired/);
  const end = signedFixture(o => { o.observed_at_ms = '1700000899000'; });
  assert.throws(() => check(end, { nowMs:'1700000900000' }), /observation_expired/);
  assert.throws(() => check(signedFixture(), { nowMs:'1699999999999' }), /observation_expired/);
});

test('future clock tolerance cannot extend validity or accept arbitrary future observations', () => {
  const future = signedFixture(o => { o.observed_at_ms = '1700000003501'; });
  assert.throws(() => check(future), /observation_expired/);
  const edge = signedFixture(o => { o.observed_at_ms = '1700000003500'; });
  assert.ok(check(edge));
});

test('inventory permission is not permission to redeem', () => {
  const f = signedFixture(o => { o.grant.scopes = ['play', 'inventory_read']; });
  assert.throws(() => check(f), /scope_missing/);
});

test('wallet binding needs its own permission and never proves wallet control or payment', () => {
  const denied = signedFixture(o => { o.challenge.action = { kind: 'wallet_bind' }; });
  assert.throws(() => check(denied), /scope_missing/);
  const allowed = signedFixture(o => {
    o.challenge.action = { kind: 'wallet_bind' };
    o.grant.scopes = ['play', 'inventory_read', 'wallet_bind'];
  });
  const result = check(allowed);
  assert.equal(result.wallet_bound, false);
  assert.equal(result.funds_moved, false);
  assert.throws(() => grantValues({ ...allowed.observation.grant, scopes: ['play', 'wallet_bind', 'inventory_read'] }));
});

test('principal withdrawal requires its own scope and binds its position and request', () => {
  const denied = signedFixture(o => { o.challenge.action = {kind:'principal_withdraw',position_id:'position-1',idempotency_key:'request-1'}; });
  assert.throws(() => check(denied), /scope_missing/);
  const allowed = signedFixture(o => {
    o.challenge.action = {kind:'principal_withdraw',position_id:'position-1',idempotency_key:'request-1'};
    o.grant.scopes.push('principal_withdraw');
  });
  assert.ok(check(allowed));
});

test('paper stage, arbitrary scope and extra fields fail closed', () => {
  const f = signedFixture(); f.observation.challenge.stage = 'paper';
  assert.throws(() => check(f), /invalid_contract/);
  const extra = signedFixture(); extra.observation.grant.admin = true;
  assert.throws(() => check(extra), /invalid_contract/);
  for (const scopes of [['play','redeem','inventory_read'],['play','redeem','redeem'],['play','admin']]) {
    assert.throws(() => grantValues({...signedFixture().observation.grant, scopes}), /invalid_contract/);
  }
});

test('integer strings and maximum grant duration are bounded', () => {
  for (const value of [1, '01', '-1', '1.5', '9223372036854775808']) assert.throws(() => units(value));
  assert.equal(units('9007199254740993'), 9007199254740993n);
  assert.throws(() => grantValues({...signedFixture().observation.grant,expires_at_ms:'1700000900001'}));
});

test('line terminators cannot bypass full-string canonical validation', () => {
  for (const suffix of ['\n', '\r', '\r\n', '\u2028', '\u2029', ' ', '\t']) {
    assert.throws(() => identifier(`session-id${suffix}`), /invalid_contract/);
    assert.throws(() => units(`1${suffix}`), /invalid_contract/);
    assert.throws(() => hex('a'.repeat(64 - suffix.length) + suffix, 32), /invalid_contract/);
  }
  assert.equal(identifier('session-id'), 'session-id');
  assert.equal(hex('a'.repeat(64), 32), 'a'.repeat(64));
  assert.equal(units('0'), 0n);
});

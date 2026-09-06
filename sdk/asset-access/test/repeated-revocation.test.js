import test from 'node:test';
import assert from 'node:assert/strict';
import { fixture, reply, environment, authorize } from './hardening-helper.js';

const settled = promise => promise.then(value => ({ value }), error => ({ error }));
const nextTurn = () => new Promise(resolve => setImmediate(resolve));
const revokeCalls = env => env.calls.filter(call => call.url.endsWith('/revoke'));

function deferredReply(env) {
  const gate = {};
  env.queue.push((_url, init) => new Promise(resolve => {
    gate.respond = resolve;
    gate.signal = init.signal;
  }));
  return gate;
}

test('a repeated revoke rejects without aborting or duplicating the first server request', async () => {
  const env = environment();
  await authorize(env);
  const gate = deferredReply(env);
  const first = settled(env.client.revoke());
  try {
    const second = await settled(env.client.revoke());
    const abortedBySecond = gate.signal.aborted;
    const requestsAfterSecond = revokeCalls(env).length;
    gate.respond(reply(fixture.valid.revoked));
    const firstResult = await first;
    assert.equal(second.error?.code, 'request_in_progress');
    assert.equal(abortedBySecond, false);
    assert.equal(requestsAfterSecond, 1);
    assert.equal(firstResult.error, undefined);
    assert.equal(firstResult.value.revoked, true);
    assert.equal(env.client.state.status, 'unauthenticated');
  } finally {
    gate.respond?.(reply(fixture.valid.revoked));
    env.client.clear();
    await first;
  }
});

test('clear and a new grant isolate late revocation results and permit a new guarded revoke', async () => {
  for (const lateStatus of [200, 503]) {
    const env = environment();
    await authorize(env);
    const oldGate = deferredReply(env);
    const oldRevoke = settled(env.client.revoke());
    let newGate, newRevoke;
    try {
      env.client.clear();
      await authorize(env, { grant_id: `aag_${'8'.repeat(32)}`, access_token: `aat_${'8'.repeat(64)}` });
      oldGate.respond(reply(lateStatus === 200 ? fixture.valid.revoked : { code: 'unavailable' }, lateStatus));
      assert.equal((await oldRevoke).error?.code, 'cleared');
      await nextTurn();
      assert.equal(env.client.state.status, 'authorized');
      newGate = deferredReply(env);
      newRevoke = settled(env.client.revoke());
      const duplicate = await settled(env.client.revoke());
      const newSignalAborted = newGate.signal.aborted;
      newGate.respond(reply(fixture.valid.revoked));
      const result = await newRevoke;
      assert.equal(duplicate.error?.code, 'request_in_progress');
      assert.equal(newSignalAborted, false);
      assert.equal(result.error, undefined);
      assert.equal(result.value.revoked, true);
      assert.equal(revokeCalls(env).length, 2);
      assert.equal(revokeCalls(env)[1].init.headers.Authorization, `Bearer aat_${'8'.repeat(64)}`);
    } finally {
      oldGate.respond?.(reply(fixture.valid.revoked));
      newGate?.respond?.(reply(fixture.valid.revoked));
      env.client.clear();
      await oldRevoke;
      if (newRevoke) await newRevoke;
    }
  }
});

test('a failed server revocation leaves no guard that blocks revoking a subsequent grant', async () => {
  const env = environment();
  try {
    await authorize(env);
    env.queue.push(reply({ code: 'unavailable' }, 503));
    await assert.rejects(env.client.revoke(), { code: 'request_failed' });
    assert.equal(env.client.state.status, 'unauthenticated');
    await authorize(env, { grant_id: `aag_${'9'.repeat(32)}`, access_token: `aat_${'9'.repeat(64)}` });
    env.queue.push(reply(fixture.valid.revoked));
    assert.equal((await env.client.revoke()).revoked, true);
    assert.equal(revokeCalls(env).length, 2);
    assert.equal(revokeCalls(env)[1].init.headers.Authorization, `Bearer aat_${'9'.repeat(64)}`);
  } finally { env.client.clear(); }
});

test('after explicit clear a new tokenless consent can still be withdrawn without HTTP', async () => {
  const env = environment();
  await authorize(env);
  const oldGate = deferredReply(env);
  const oldRevoke = settled(env.client.revoke());
  try {
    env.client.clear();
    const callsBeforeConsent = env.calls.length;
    const request = await env.client.authorizationRequest({
      redirectUri: 'https://assets.example/quant/asset-access/callback',
      scopes: fixture.valid.token.scopes, explicitConsent: true,
    });
    const withdrawal = await settled(env.client.revoke());
    const stateAfterWithdrawal = env.client.state;
    // A broken pending-consent cleanup can finish deterministically using this fake response.
    env.queue.push(reply(fixture.valid.token));
    const exchange = await settled(env.client.exchangeCode({
      schema: 'yilong.asset_access.authorization_code.v1', code: `aac_${'1'.repeat(64)}`,
      state: request.state, client_id: request.client_id, redirect_uri: request.redirect_uri,
      code_expires_at: '2030-01-01T00:01:00Z', grant_id: fixture.valid.token.grant_id,
      expires_at: fixture.valid.token.expires_at, scopes: request.scopes,
    }));
    oldGate.respond(reply(fixture.valid.revoked));
    assert.equal((await oldRevoke).error?.code, 'cleared');
    assert.equal(withdrawal.error?.code, 'authorization_required');
    assert.equal(stateAfterWithdrawal.status, 'unauthenticated');
    assert.equal(exchange.error?.code, 'authorization_required');
    assert.equal(env.calls.length, callsBeforeConsent);
    assert.equal(revokeCalls(env).length, 1);
  } finally {
    oldGate.respond?.(reply(fixture.valid.revoked));
    env.client.clear();
    await oldRevoke;
  }
});

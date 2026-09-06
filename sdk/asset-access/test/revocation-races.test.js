import test from 'node:test';
import assert from 'node:assert/strict';
import { fixture, reply, environment, authorize } from './hardening-helper.js';

const settled = promise => promise.then(value => ({ value }), error => ({ error }));
const nextTurn = () => new Promise(resolve => setImmediate(resolve));

async function pendingAuthorization(env) {
  const request = await env.client.authorizationRequest({
    redirectUri: 'https://assets.example/quant/asset-access/callback',
    scopes: fixture.valid.token.scopes, explicitConsent: true,
  });
  return {
    schema: 'yilong.asset_access.authorization_code.v1', code: `aac_${'1'.repeat(64)}`,
    state: request.state, client_id: request.client_id, redirect_uri: request.redirect_uri,
    code_expires_at: '2030-01-01T00:01:00Z', grant_id: fixture.valid.token.grant_id,
    expires_at: fixture.valid.token.expires_at, scopes: request.scopes,
  };
}

test('revoke cancels pending authorization even before a token exists', async () => {
  const env = environment();
  const response = await pendingAuthorization(env);
  const revoked = await settled(env.client.revoke());
  const stateAfterRevoke = env.client.state;
  // Supply a valid synthetic token so the pre-fix path can finish without a timeout.
  env.queue.push(reply(fixture.valid.token));
  const exchanged = await settled(env.client.exchangeCode(response));
  env.client.clear();
  assert.equal(revoked.error?.code, 'authorization_required');
  assert.equal(stateAfterRevoke.status, 'unauthenticated');
  assert.equal(exchanged.error?.code, 'authorization_required');
  assert.equal(env.calls.length, 0);
});

test('revoke cancels an in-flight exchange whose token is not available yet', async () => {
  const env = environment();
  const response = await pendingAuthorization(env);
  let answerToken;
  env.queue.push(() => new Promise(resolve => { answerToken = resolve; }));
  const exchange = settled(env.client.exchangeCode(response));
  const revoked = await settled(env.client.revoke());
  const stateAfterRevoke = env.client.state;
  answerToken(reply(fixture.valid.token));
  const exchanged = await exchange;
  const stateAfterToken = env.client.state;
  env.client.clear();
  assert.equal(revoked.error?.code, 'authorization_required');
  assert.equal(stateAfterRevoke.status, 'unauthenticated');
  assert.equal(exchanged.error?.code, 'cleared');
  assert.equal(stateAfterToken.status, 'unauthenticated');
  assert.equal(env.calls.filter(call => call.url.endsWith('/revoke')).length, 0);
});

test('local authorization and snapshot disappear before a slow server revocation answers', async () => {
  const env = environment();
  await authorize(env);
  env.queue.push(reply(fixture.valid.asset_zero));
  await env.client.readAssets();
  let answer;
  env.queue.push(() => new Promise(resolve => { answer = resolve; }));
  const pending = settled(env.client.revoke());
  try {
    assert.equal(env.client.state.status, 'unauthenticated');
    assert.equal(env.client.state.has_snapshot, false);
    await assert.rejects(env.client.identity(), { code: 'authorization_required' });
    assert.equal(env.calls.filter(call => call.url.endsWith('/revoke')).length, 1);
    answer(reply(fixture.valid.revoked));
    assert.equal((await pending).value.revoked, true);
  } finally { env.client.clear(); }
});

test('failed revocation also aborts an in-flight read and never restores its late response', async () => {
  const env = environment();
  await authorize(env);
  let answerRead;
  env.queue.push(() => new Promise(resolve => { answerRead = resolve; }));
  const oldRead = settled(env.client.readAssets());
  env.queue.push(reply({ code: 'PRIVATE_SERVER_TEXT', token: fixture.valid.token.access_token }, 503));
  try {
    await assert.rejects(env.client.revoke(), error => {
      assert.equal(error.code, 'request_failed');
      assert.equal(error.message.includes('PRIVATE_SERVER_TEXT'), false);
      assert.equal(JSON.stringify(error).includes(fixture.valid.token.access_token), false);
      return true;
    });
    answerRead(reply(fixture.valid.asset_zero));
    assert.equal((await oldRead).error.code, 'cleared');
    await nextTurn();
    assert.equal(env.client.state.status, 'unauthenticated');
    assert.equal(env.client.state.has_snapshot, false);
  } finally { env.client.clear(); }
});

for (const lateStatus of [200, 503]) {
  test(`late old revocation HTTP ${lateStatus} cannot clear a newly authorized grant`, async () => {
    const env = environment();
    await authorize(env);
    let answerRevoke;
    env.queue.push(() => new Promise(resolve => { answerRevoke = resolve; }));
    const oldRevoke = settled(env.client.revoke());
    try {
      await authorize(env, { grant_id: `aag_${'9'.repeat(32)}`, access_token: `aat_${'9'.repeat(64)}` });
      assert.equal(env.client.state.status, 'authorized');
      answerRevoke(reply(lateStatus === 200 ? fixture.valid.revoked : { code: 'unavailable' }, lateStatus));
      assert.equal((await oldRevoke).error.code, 'cleared');
      await nextTurn();
      assert.equal(env.client.state.status, 'authorized');
      env.queue.push(reply(fixture.valid.asset_zero));
      await env.client.readAssets();
      assert.equal(env.calls.at(-1).init.headers.Authorization, `Bearer aat_${'9'.repeat(64)}`);
      assert.equal(env.client.state.has_snapshot, true);
    } finally { env.client.clear(); }
  });
}

import test from 'node:test';
import assert from 'node:assert/strict';
import { fixture, clone, reply, rawReply, environment, authorize } from './hardening-helper.js';

test('revoke interrupts an in-flight read, clears locally, and still requests server revocation', async () => {
  const env = environment();
  await authorize(env);
  let completeRead;
  env.queue.push(() => new Promise(resolve => { completeRead = resolve; }));
  const oldRead = env.client.readAssets().then(value => ({ value }), error => ({ error }));
  env.queue.push(reply(fixture.valid.revoked));
  let result, failure;
  try { result = await env.client.revoke(); } catch (error) { failure = error; }
  const stateAfterRevoke = env.client.state;
  completeRead(reply(fixture.valid.asset_zero));
  const oldOutcome = await oldRead;
  env.client.clear();
  assert.equal(failure, undefined);
  assert.equal(result.revoked, true);
  assert.equal(stateAfterRevoke.status, 'unauthenticated');
  assert.equal(stateAfterRevoke.has_snapshot, false);
  assert.equal(oldOutcome.error?.code, 'cleared');
  assert.equal(env.calls.filter(call => new URL(call.url).pathname.endsWith('/revoke')).length, 1);
});

test('a request ID already returned on an earlier page cannot be returned again', async () => {
  const env = environment();
  await authorize(env);
  env.queue.push(reply(fixture.valid.asset_first));
  const first = await env.client.readAssets({ limit: 1 });
  const duplicate = clone(fixture.valid.asset_second);
  duplicate.progress.requests[0].request_id = first.page.progress.requests[0].request_id;
  env.queue.push(reply(duplicate));
  try {
    await assert.rejects(env.client.readAssets({ limit: 1, cursor: first.page.progress.next_cursor }),
      { code: 'invalid_response' });
    assert.equal(env.client.state.status, 'unauthenticated');
    assert.equal(env.client.state.has_snapshot, false);
  } finally { env.client.clear(); }
});

test('raw duplicate subject fields fail before the later JSON value can satisfy identity binding', async () => {
  const env = environment();
  await authorize(env);
  const raw = JSON.stringify(fixture.valid.identity).replace('{', '{"subject":"wrong-subject",');
  env.queue.push(rawReply(raw));
  try {
    await assert.rejects(env.client.identity(), { code: 'invalid_response' });
    assert.equal(env.client.state.status, 'unauthenticated');
    assert.equal(env.client.state.has_snapshot, false);
  } finally { env.client.clear(); }
});

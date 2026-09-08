'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');
const { fixture, flush } = require('./fixtures/chatgpt-runtime-stop');
const { attach } = require('./fixtures/chatgpt-runtime-bindings');

async function requestFixture(request) {
  const f = fixture();
  let active = true;
  f.setRequest(request);
  f.modules.shared.Fl = value => value === request && active;
  await attach(f.page, f.modules).api.load('shared');
  return { ...f, finish() { active = false; f.setState(false, null); f.publish(); } };
}

for (const client of ['guest:synthetic-thread', 'scope/synthetic.thread', '[synthetic scope]',
  'synthetic-' + 'x'.repeat(160)]) test('official composite request ID preserves its opaque client component: ' + client.length, async () => {
  const request = 'request-' + client + '-17', f = await requestFixture(request);
  const result = f.api.stop(f.command);
  assert.equal(result.handled, true);
  await flush();
  assert.deepEqual(f.calls, [['synthetic-client-thread', request, {
    clientInitiated: true, clientStopReason: 'user_stop_mouse'
  }]]);
  assert.equal(f.api.diagnostics().request, 'valid');
  f.settle(); await flush();
  assert.equal(f.api.state().pending, true);
  f.finish();
  assert.equal((await result.completion).code, 'stop_observed');
  assert.equal(f.calls.length, 1);
});

for (const request of ['request-scope/thread-1\n', 'request-scope\nthread-1',
  'request-scope\0thread-1', 'request-scope\x7fthread-1', 'request-scope/thread-no-counter',
  'request-' + 'x'.repeat(512) + '-1', { id: 'request-synthetic-1' }]) {
  test('non-protocol or unbounded request shape cannot reach the stop runtime: ' + JSON.stringify(request).length, async () => {
    const f = await requestFixture(request), result = f.api.stop(f.command);
    assert.equal(result.handled, false); assert.equal(f.api.state().code, 'request_unavailable');
    assert.equal(f.calls.length, 0);
  });
}

test('a composite successor cannot inherit a prepared stop', async () => {
  const f = await requestFixture('request-synthetic/thread-1'), result = f.api.stop(f.command);
  assert.equal(result.handled, true);
  f.setRequest('request-synthetic/thread-2');
  assert.equal((await result.completion).status, 'rejected');
  assert.equal(result.claimFallback(), false); assert.equal(f.calls.length, 0);
});

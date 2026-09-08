'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');
const { fixture, flush } = require('./fixtures/chatgpt-runtime-stop');
const { attach } = require('./fixtures/chatgpt-runtime-bindings');

async function cached(guest = false, options = {}) {
  const f = fixture(options);
  if (guest) {
    f.page.location.href = 'https://chatgpt.com/';
    f.page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders = () => null;
    f.modules.shared.R5 = () => ({ authStatus: 'logged_out' });
    f.modules.shared.F5 = () => null;
  }
  const bridge = attach(f.page, f.modules);
  await bridge.api.load('shared');
  return { ...f, bridge };
}

for (const guest of [false, true]) {
  for (const property of [null, 'previous-request']) {
    test(`cached official tree owns the stop when composer property is ${property}; guest=${guest}`, async () => {
      const f = await cached(guest);
      f.props.currentRequestId = property;
      const result = f.api.stop(f.command);
      assert.equal(result.handled, true);
      await flush();
      assert.equal(f.calls.length, 1);
      assert.equal(f.calls[0][1], 'synthetic-request');
      f.setState(false, null); f.settle();
      assert.deepEqual(await result.completion, { status: 'accepted', code: 'stop_observed' });
      assert.equal(f.timers.size, 0); assert.equal(f.listeners.size, 0);
    });
  }
}

test('cold missing request declines immediately without loading and capturing a later request', () => {
  const f = fixture();
  const bridge = attach(f.page, f.modules);
  f.props.currentRequestId = null;
  assert.equal(f.api.stop(f.command).handled, false);
  assert.equal(f.api.state().code, 'request_unavailable');
  assert.equal(f.calls.length, 0); assert.equal(f.loads.length, 0);
  assert.equal(bridge.loads.length, 0); assert.equal(f.timers.size, 0);
});

test('an empty official request is not replaced by a stale composer property', async () => {
  const f = await cached();
  f.setRequest(null); f.props.currentRequestId = 'synthetic-request';
  assert.equal(f.api.stop(f.command).handled, false);
  assert.equal(f.calls.length, 0); assert.equal(f.timers.size, 0);
});

test('a canonical request replaced during loading never stops its successor', async () => {
  const f = await cached(); f.props.currentRequestId = null;
  const result = f.api.stop(f.command);
  assert.equal(result.handled, true);
  f.setRequest('next-request'); f.props.currentRequestId = null;
  assert.equal((await result.completion).code, 'context_changed');
  assert.equal(result.claimFallback(), false);
  assert.equal(f.calls.length, 0); assert.equal(f.timers.size, 0);
});

test('request replacement during subscriptions is checked before the official stop', async () => {
  let f;
  f = await cached(false, { onSubscribe() { f.setRequest('next-request'); f.props.currentRequestId = null; } });
  f.props.currentRequestId = null;
  const result = f.api.stop(f.command);
  assert.equal(result.handled, true);
  assert.equal((await result.completion).code, 'context_changed');
  assert.equal(f.calls.length, 0); assert.equal(f.listeners.size, 0);
});

test('a request lookup failure cannot silently use the old composer request', async () => {
  const f = await cached();
  f.modules.shared.HM.getRequestId = () => { throw Error('synthetic lookup failure'); };
  assert.equal(f.api.stop(f.command).handled, false);
  assert.equal(f.calls.length, 0);
});

test('canonical request selection still excludes realtime voice', async () => {
  const f = await cached(true); f.props.currentRequestId = null; f.setState(true, 5);
  const result = f.api.stop(f.command);
  assert.equal(result.handled, true);
  assert.equal((await result.completion).code, 'voice_active');
  assert.equal(f.calls.length, 0); assert.equal(f.timers.size, 0);
});

test('request completion may remove its tree identity only after the stop invocation', async () => {
  const f = await cached(); f.props.currentRequestId = null;
  const result = f.api.stop(f.command); await flush();
  assert.equal(f.calls.length, 1);
  f.setRequest(null); f.setState(false, null); f.settle();
  assert.equal((await result.completion).code, 'stop_observed');
  assert.equal(f.timers.size, 0); assert.equal(f.listeners.size, 0);
});

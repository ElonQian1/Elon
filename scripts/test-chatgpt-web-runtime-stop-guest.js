'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const { fixture, flush } = require('./fixtures/chatgpt-runtime-stop');
const { attach } = require('./fixtures/chatgpt-runtime-bindings');

async function guest(withRequestIdentity = false) {
  const f = fixture();
  f.page.location.href = 'https://chatgpt.com/';
  if (!withRequestIdentity) f.page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders = () => null;
  const auth = { authStatus: 'logged_out' };
  f.modules.shared.R5 = () => auth;
  f.modules.shared.F5 = () => null;
  const bridge = attach(f.page, f.modules);
  await bridge.api.load('shared');
  return { ...f, bridge, auth };
}

for (const headers of [false, true]) test('guest stop reuses the verified current runtime identity: headers=' + headers, async () => {
  const f = await guest(headers), transaction = f.api.stop(f.command);
  assert.equal(transaction.handled, true); await flush();
  assert.equal(f.calls.length, 1);
  assert.equal(f.calls[0][0], f.props.conversation.id);
  f.setState(false, null); f.settle();
  assert.deepEqual(await transaction.completion, { status: 'accepted', code: 'stop_observed' });
  assert.equal(f.timers.size, 0); assert.equal(f.listeners.size, 0);
});

for (const headers of [false, true]) test('guest proof disappearing before invocation cancels the stop: headers=' + headers, async () => {
  const f = await guest(headers), transaction = f.api.stop(f.command);
  assert.equal(transaction.handled, true);
  f.props.conversation.serverId$ = () => null;
  f.auth.authStatus = 'logged_in';
  await flush();
  assert.equal(f.calls.length, 0);
  assert.equal((await transaction.completion).status, 'rejected');
  assert.equal(transaction.claimFallback(), false);
});

test('missing credentials are not sufficient evidence to stop as a guest', async () => {
  const f = await guest();
  f.page.__elonChatGptPrivateRuntimeBindings = { ...f.bridge.api, peek: () => null };
  assert.equal(f.api.stop(f.command).handled, false);
  assert.equal(f.calls.length, 0);
});

test('guest request replacement during preparation never stops the next generation', async () => {
  const f = await guest(), transaction = f.api.stop(f.command);
  assert.equal(transaction.handled, true); f.setRequest('next-request'); await flush();
  assert.equal((await transaction.completion).code, 'context_changed');
  assert.equal(f.calls.length, 0);
});

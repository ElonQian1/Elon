'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');
const { fixture, flush } = require('./fixtures/chatgpt-runtime-stop');
const { attach } = require('./fixtures/chatgpt-runtime-bindings');

async function scoped(options = {}) {
  const f = fixture(options);
  let leaf = 'synthetic-answer', turn = 'synthetic-turn', tree = {};
  f.setRequest(null); f.setState(false, 3);
  f.modules.shared.XM = key => { assert.equal(key, f.props.conversation.id); return tree; };
  f.modules.shared.HM.getCurrentLeafId = () => leaf;
  f.modules.shared.HM.getConversationLastTurn = () => ({ id: turn });
  if (options.guest) {
    f.page.location.href = 'https://chatgpt.com/';
    f.page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders = () => null;
    f.modules.shared.R5 = () => ({ authStatus: 'logged_out' });
    f.modules.shared.F5 = () => null;
  }
  const bridge = attach(f.page, f.modules);
  await bridge.api.load('shared');
  return { ...f, bridge, leaf: value => { leaf = value; }, turn: value => { turn = value; },
    noTree: () => { tree = null; } };
}

for (const guest of [false, true]) test('conversation-owned stop awaits official cleanup; guest=' + guest, async () => {
  const f = await scoped({ guest }), result = f.api.stop(f.command);
  assert.equal(result.handled, true); await flush();
  assert.deepEqual(f.calls, [['synthetic-client-thread', undefined, {
    clientInitiated: true, clientStopReason: 'user_stop_mouse'
  }]]);
  f.settle(); await flush();
  assert.equal(f.api.state().pending, true, 'HTTP settlement alone must not enable the next send');
  const duplicate = f.api.stop({ ...f.command, requestId: 'mcp_duplicate' });
  assert.equal(duplicate.completion, result.completion);
  f.setState(false, null); f.publish();
  assert.deepEqual(await result.completion, { status: 'accepted', code: 'stop_observed' });
  assert.equal(f.calls.length, 1); assert.equal(f.timers.size, 0); assert.equal(f.listeners.size, 0);
});

for (const [name, change] of Object.entries({
  leaf: f => f.leaf('next-answer'), turn: f => f.turn('next-turn'),
  request: f => f.setRequest('next-request'),
  status_generation: f => f.setState(false, 3),
  identity: f => f.setIdentity(),
  document: f => { f.page.__elonChatGptDocumentToken = 'doc_replaced'; },
  controller: f => { f.props.composerController = { conversation: f.props.conversation }; }
})) test(name + ' replacement before invocation never stops a successor', async () => {
  const f = await scoped(), result = f.api.stop(f.command);
  assert.equal(result.handled, true); change(f);
  assert.equal((await result.completion).status, 'rejected');
  assert.equal(result.claimFallback(), false); assert.equal(f.calls.length, 0);
});

test('subscription reentry cannot exchange the pinned no-request generation', async () => {
  let f;
  f = await scoped({ onSubscribe() { f.setState(false, 3); } });
  const result = f.api.stop(f.command);
  assert.equal(result.handled, true);
  assert.equal((await result.completion).status, 'rejected');
  assert.equal(f.calls.length, 0); assert.equal(f.listeners.size, 0);
});

for (const [name, change] of Object.entries({
  leaf: f => f.leaf('next-answer'), turn: f => f.turn('next-turn'),
  request: f => f.setRequest('next-request'), status_generation: f => f.setState(false, 3),
  voice: f => f.setState(false, 5)
})) test(name + ' replacement after invocation is unknown, never a second stop', async () => {
  const f = await scoped(), result = f.api.stop(f.command);
  assert.equal(result.handled, true); await flush(); change(f); f.settle();
  assert.equal((await result.completion).status, 'unknown');
  assert.equal(result.claimFallback(), false); assert.equal(f.calls.length, 1);
  assert.equal(f.timers.size, 0); assert.equal(f.listeners.size, 0);
});

for (const [name, change] of Object.entries({
  no_leaf: f => f.leaf(null), no_turn: f => f.turn(null),
  no_server: f => { f.props.conversation.serverId$ = () => null; },
  no_tree: f => f.noTree(),
  other_stop: f => { f.modules.shared.XM(f.props.conversation.id).interruptionInProgress = true; },
  invalid_leaf: f => f.leaf('not/a/leaf'), invalid_request: f => f.setRequest('request with spaces'),
  idle: f => f.setState(false, null), unread: f => f.setState(false, 4)
})) test(name + ' cannot manufacture a conversation stop owner', async () => {
  const f = await scoped({ guest: true }); change(f);
  assert.equal(f.api.stop(f.command).handled, false);
  assert.equal(f.calls.length, 0); assert.equal(f.timers.size, 0);
  assert.equal(f.bridge.loads.length, 1, 'no imports after failing to pin an owner');
});

for (const mode of [5, 6, 7]) test('no-request voice state ' + mode + ' rejects text stop without DOM fallback', async () => {
  const f = await scoped(); f.setState(false, mode);
  const result = f.api.stop(f.command);
  assert.equal(result.handled, true);
  assert.equal((await result.completion).code, 'voice_active');
  assert.equal(result.claimFallback?.() || false, false);
  assert.equal(f.calls.length, 0);
});

test('timeout retains unresolved owner; late official cleanup releases it', async () => {
  const f = await scoped(), result = f.api.stop(f.command);
  assert.equal(result.handled, true); await flush(); f.runTimer(15000);
  assert.equal((await result.completion).code, 'timeout');
  assert.equal(f.api.state().pending, true);
  f.setState(false, null); f.settle(); await flush();
  assert.equal(f.api.state().pending, false); assert.equal(f.calls.length, 1);
  assert.equal(f.timers.size, 0); assert.equal(f.listeners.size, 0);
});

test('unavailable module fallback is still bound to the exact captured generation', async () => {
  const f = await scoped({ loadRuntime: () => Promise.reject(Error('unavailable')) });
  const result = f.api.stop(f.command);
  assert.equal(result.handled, true);
  assert.equal((await result.completion).status, 'unavailable');
  f.turn('next-turn');
  assert.equal(result.claimFallback(), false); assert.equal(f.calls.length, 0);
});

test('diagnostics contain only captured shape and never read runtime state on demand', async () => {
  const f = await scoped(); f.leaf(null);
  assert.equal(f.api.stop(f.command).handled, false);
  const reads = f.modules.shared.HM.getRequestId;
  f.modules.shared.HM.getRequestId = () => assert.fail('diagnostic read must not inspect the page');
  assert.deepEqual(f.api.diagnostics(), {
    cached: true, request: 'missing', tree: true, generation: false, mode: 'streaming'
  });
  f.modules.shared.HM.getRequestId = reads;
  assert.doesNotMatch(JSON.stringify(f.api.diagnostics()), /synthetic|Bearer|doc_runtime|11111111/);
  f.page.__elonChatGptDocumentToken = 'doc_replaced';
  assert.equal(f.api.diagnostics().cached, false);
});

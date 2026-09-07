'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');
const { fixture, flush, id } = require('./fixtures/chatgpt-runtime-stop.js');

test('stop binds the current official request without requiring a ready composer or file store', async () => {
  const f = fixture(), result = f.api.stop(f.command);
  assert.equal(result.handled, true); await flush();
  assert.deepEqual(f.calls, [['synthetic-client-thread', 'synthetic-request', {
    clientInitiated: true, clientStopReason: 'user_stop_mouse'
  }]]);
  assert.equal(f.api.state().pending, true);
  f.settle(); await flush();
  assert.equal(f.api.state().pending, true, 'a successful stop response is not a stopped generation');
  f.setState(false, null); f.publish();
  assert.deepEqual(await result.completion, { status: 'accepted', code: 'stop_observed' });
  assert.equal(f.api.state().pending, false);
  assert.equal(f.timers.size, 0); assert.equal(f.listeners.size, 0);
});

test('completed-unread state is terminal, not a need to cancel again', async () => {
  const f = fixture(), result = f.api.stop(f.command); await flush();
  f.setState(false, 4); f.settle();
  assert.equal((await result.completion).status, 'accepted');
});

test('already finished before dispatch does not send a stop request', async () => {
  const f = fixture(); f.setState(false, null);
  assert.equal((await f.api.stop(f.command).completion).code, 'already_stopped');
  assert.equal(f.calls.length, 0);
});

test('duplicate stop commands share one in-flight transaction', async () => {
  const f = fixture(), first = f.api.stop(f.command);
  const second = f.api.stop({ ...f.command, requestId: 'mcp_repeat' }); await flush();
  assert.equal(first.completion, second.completion); assert.equal(f.calls.length, 1);
  f.setState(false, null); f.settle(); await first.completion;
});

test('a stop during official pre-dispatch streaming cancels the same pending request', async () => {
  const f = fixture(); f.setState(false, 3);
  const result = f.api.stop(f.command); await flush(); assert.equal(f.calls.length, 1);
  f.setState(false, null); f.settle(); assert.equal((await result.completion).status, 'accepted');
});

for (const mode of [5, 6, 7]) {
  test('voice mode ' + mode + ' is never stopped by a text-generation command', async () => {
    const f = fixture(); f.setState(true, mode);
    const receipt = await f.api.stop(f.command).completion;
    assert.deepEqual(receipt, { status: 'rejected', code: 'voice_active' });
    assert.equal(f.calls.length, 0); assert.equal(f.api.state().pending, false);
  });
}

for (const [name, change] of Object.entries({
  disabled: f => { f.page.__elonChatGptPrivateTextTransactionsEnabled = false; },
  detached: f => { f.node.isConnected = false; },
  uncommitted: f => { f.top.stateNode.current = {}; },
  wrong_route: f => { f.page.location.href = 'https://chatgpt.com/share/' + id; },
  unknown_runtime: f => { f.page.performance.getEntriesByName = () => []; },
  unknown_request: f => { f.props.currentRequestId = null; },
  structured_input: f => { f.props.structuredInputHost = {}; },
  unknown_document: f => { f.page.__elonChatGptDocumentToken = ''; }
})) {
  test(name + ' does not invoke an unbound stop', () => {
    const f = fixture(); change(f);
    assert.equal(f.api.stop(f.command).handled, false);
    assert.equal(f.calls.length, 0); assert.equal(f.loads.length, 0);
  });
}

for (const [name, change] of Object.entries({
  conversation: f => { f.page.location.href = 'https://chatgpt.com/c/' + id.replace(/^1/, '9'); },
  identity: f => f.setIdentity(),
  document: f => { f.page.__elonChatGptDocumentToken = 'doc_new_document'; },
  request: f => f.setRequest('new-request'),
  context: f => { f.props.composerController = { conversation: f.props.conversation }; }
})) {
  test(name + ' changing during module preparation cannot stop the next request', async () => {
    const f = fixture(), result = f.api.stop(f.command); change(f); await flush();
    assert.equal((await result.completion).status, 'rejected'); assert.equal(f.calls.length, 0);
  });
}

test('identity changing during subscription is rechecked before the official call', async () => {
  let f; f = fixture({ onSubscribe: () => f.setIdentity() });
  const result = f.api.stop(f.command); await flush();
  assert.equal((await result.completion).code, 'context_changed');
  assert.equal(f.calls.length, 0); assert.equal(f.listeners.size, 0);
});

test('voice state changing during subscription cannot be cancelled accidentally', async () => {
  let f; f = fixture({ onSubscribe: () => f.setState(true, 5) });
  const result = f.api.stop(f.command); await flush();
  assert.equal((await result.completion).status, 'rejected'); assert.equal(f.calls.length, 0);
});

test('a failed second subscription cleans up the first without invoking a stop', async () => {
  const f = fixture();
  f.page.__elonChatGptPrivateStreamTransport.subscribe = () => { throw Error('synthetic subscription error'); };
  const result = f.api.stop(f.command);
  assert.equal((await result.completion).status, 'rejected');
  assert.equal(f.calls.length, 0); assert.equal(f.listeners.size, 0);
});

test('missing runtime export allows fallback only before invocation', async () => {
  const f = fixture(); delete f.modules.conversation.FVt;
  assert.equal((await f.api.stop(f.command).completion).status, 'unavailable');
  assert.equal(f.calls.length, 0); assert.equal(f.api.state().pending, false);
});

test('one pre-dispatch fallback claim is shared by duplicate commands', async () => {
  const f = fixture(); delete f.modules.conversation.FVt;
  const first = f.api.stop(f.command), second = f.api.stop(f.command);
  await first.completion;
  assert.equal(first.claimFallback(), true);
  assert.equal(second.claimFallback(), false);
});

test('fallback scope is checked again after the asynchronous receipt reaches its caller', async () => {
  const f = fixture(); delete f.modules.conversation.FVt;
  const result = f.api.stop(f.command); await result.completion;
  f.setRequest('another-request');
  assert.equal(result.claimFallback(), false);
});

test('module load timeout cannot leave a late stop in the next conversation', async () => {
  const resolvers = [], f = fixture({ loadRuntime: () => new Promise(resolve => resolvers.push(resolve)) });
  const result = f.api.stop(f.command); await flush(); f.runTimer(1500); await flush();
  assert.equal((await result.completion).status, 'unavailable');
  resolvers[0](f.modules.shared); resolvers[1](f.modules.conversation); await flush();
  assert.equal(f.calls.length, 0); assert.equal(f.api.state().pending, false);
});

test('deadline retains an unresolved stop and coalesces retry until its request settles', async () => {
  const f = fixture(), first = f.api.stop(f.command); await flush(); f.runTimer(15000);
  assert.deepEqual(await first.completion, { status: 'unknown', code: 'timeout' });
  assert.equal(f.api.state().pending, true);
  assert.equal(f.api.stop(f.command).completion, first.completion); assert.equal(f.calls.length, 1);
  f.settle(); await flush();
  assert.equal(f.api.state().pending, false, 'expired, settled request must not retain a permanent writer lock');
  assert.equal(f.listeners.size, 0); assert.equal(f.timers.size, 0);
});

test('state checks have bounded retries instead of continuous DOM polling', async () => {
  const f = fixture(), result = f.api.stop(f.command); await flush(); f.settle(); await flush();
  let count = 0; while (f.runTimer(100)) { count++; assert.ok(count <= 30); }
  assert.equal(count, 30); f.runTimer(15000);
  assert.equal((await result.completion).code, 'timeout');
  assert.equal(f.api.state().pending, false); assert.equal(f.listeners.size, 0);
});

test('request failure reports unknown and releases subscriptions without a second write', async () => {
  const f = fixture(), result = f.api.stop(f.command); await flush(); f.reject();
  assert.deepEqual(await result.completion, { status: 'unknown', code: 'stop_failed' });
  assert.equal(f.calls.length, 1); assert.equal(f.listeners.size, 0); assert.equal(f.timers.size, 0);
});

test('new request after dispatch is not completed or stopped by the old receipt', async () => {
  const f = fixture(), result = f.api.stop(f.command); await flush();
  f.setRequest('another-request'); f.setState(false, null); f.settle();
  assert.equal((await result.completion).code, 'request_changed'); assert.equal(f.calls.length, 1);
});

test('document replacement releases the old owner without its late callback affecting a new stop', async () => {
  const f = fixture(), result = f.api.stop(f.command); await flush();
  f.page.__elonChatGptDocumentToken = 'doc_replaced';
  assert.equal(f.api.state().pending, false); assert.equal((await result.completion).code, 'document_changed');
  f.settle(); await flush(); assert.equal(f.listeners.size, 0);
});

test('the shared context continues to reject foreign conversations without requiring an input-ready state', () => {
  const f = fixture();
  assert.ok(f.page.__elonChatGptPrivateTextRuntimeSubmit.captureConversation(f.node));
  f.props.conversation.serverId$ = () => 'another-thread';
  assert.equal(f.page.__elonChatGptPrivateTextRuntimeSubmit.captureConversation(f.node), null);
});

'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture } = require('./fixtures/chatgpt-private-text-input');
const asset = name => require('../android/app/src/main/assets/chatgpt_web_fresh_text_' + name);
const tick = () => new Promise(resolve => setImmediate(resolve));

function coldFixture() {
  const f = fixture(), bindings = f.page.__elonChatGptPrivateRuntimeBindings;
  const peek = bindings.peek, load = bindings.load;
  let ready = false, promise, resolve, reject, imports = 0;
  bindings.peek = role => role === 'composer' && !ready ? null : peek(role);
  bindings.load = role => {
    if (role !== 'composer' || ready) return load(role);
    // Model the production binding loader's shared in-flight import.
    if (!promise) {
      imports++;
      promise = new Promise((yes, no) => { resolve = yes; reject = no; }).then(() => {
        ready = true; return f.editor;
      });
    }
    return promise;
  };
  const identity = value => {
    f.identity(value);
    f.page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders = () =>
      value ? { Authorization: 'Bearer synthetic-' + value } : null;
  };
  return { ...f, identity, bindings, resolve: () => resolve(), reject: () => reject(Error('runtime_timeout')),
    imports: () => imports };
}

test('first memory-owner capture joins a cold observed editor import without a second click', async () => {
  const f = coldFixture();
  const pending = f.context.capture(null);
  let done = false; void pending.then(() => { done = true; }, () => { done = true; });
  await tick();
  assert.equal(done, false);
  assert.equal(f.imports(), 1);
  f.resolve();
  const owner = await pending;
  assert.equal(owner.current(), true);
  assert.equal(owner.draft.read(), f.draft());
  assert.equal(f.view.dom.isConnected, false);
  assert.deepEqual(f.edits, []);
});

test('cold native input snapshots share the import and notify readiness once', async () => {
  const f = coldFixture(); let notifications = 0;
  for (let i = 0; i < 20; i++) assert.equal(f.api.snapshot(null, () => { notifications++; }).ready, false);
  await tick();
  assert.equal(notifications, 0); assert.equal(f.imports(), 1);
  f.resolve(); await tick();
  assert.equal(f.api.snapshot(null).ready, true); assert.equal(notifications, 1);
  assert.deepEqual(f.edits, []);
});

for (const [name, change] of [
  ['account', f => f.identity('another-fixture-account')],
  ['document', f => { f.page.document = {}; }],
  ['document token', f => { f.page.__elonChatGptDocumentToken = 'doc_replaced'; }],
  ['route', f => { f.page.location.href = 'https://chatgpt.com/'; }],
  ['profile', f => { f.bindings.state = () => ({ profile_id: 'web_20260915' }); }],
  ['controller', f => { f.props.composerController = { conversation: f.selected }; }],
  ['committed owner', f => { f.root.stateNode.current = {}; }],
]) {
  test(`a ${name} change while importing cannot capture a replacement draft`, async () => {
    const f = coldFixture(), pending = f.context.capture(null);
    const rejected = assert.rejects(pending, /context_changed/);
    await tick(); change(f); f.resolve(); await rejected;
    assert.deepEqual(f.edits, []);
  });
}

test('an unobserved module is not guessed or imported', async () => {
  const f = coldFixture(); f.bindings.observed = role => role !== 'composer';
  await assert.rejects(f.context.capture(null), /context_unavailable/);
  assert.equal(f.imports(), 0); assert.deepEqual(f.edits, []);
});

test('an absent account does not cause an import or deferred capture', async () => {
  const f = coldFixture(); f.identity(null);
  await assert.rejects(f.context.capture(null), /context_unavailable/);
  assert.equal(f.imports(), 0); assert.deepEqual(f.edits, []);
});

test('a failed import leaves the draft untouched and does not replay preparation', async () => {
  const f = coldFixture(), pending = f.context.capture(null);
  const rejected = assert.rejects(pending, /runtime_timeout/);
  await tick(); f.reject(); await rejected;
  assert.equal(f.imports(), 1); assert.deepEqual(f.edits, []);
});

test('native input ignores late import completion after its account changes', async () => {
  const f = coldFixture(); let notifications = 0;
  f.api.snapshot(null, () => { notifications++; }); await tick();
  f.identity(null); f.resolve(); await tick();
  assert.equal(f.api.snapshot(null).ready, false); assert.equal(notifications, 0);
  assert.deepEqual(f.edits, []);
});

test('the real sender performs zero writes while loading and one POST after owner confirmation', async () => {
  const f = coldFixture(), calls = [], page = f.page;
  f.draft('');
  const parentId = f.parent.id, conversationId = f.selected.serverId$();
  const responseId = '44444444-4444-4444-8444-444444444444';
  Object.assign(page, { crypto: require('node:crypto'), AbortController,
    __elonChatGptFreshTextStream: asset('stream'), __elonChatGptFreshTextRecovery: asset('recovery') });
  f.shared.textApi.safePost = async () => { calls.push('prepare'); return { conduit_token: 'synthetic' }; };
  f.shared.textSecurityHeaders = () => ({ 'OpenAI-Sentinel-Chat-Requirements-Token': 'synthetic' });
  f.shared.textTopic = () => { throw Error('unexpected_topic'); };
  f.conversation.textSecurity = () => ({ chatReq: { token: 'synthetic' } });
  let posted;
  f.conversation.textStream = async function* (_, options) {
    options.onBeforeRequestStart(); calls.push('post'); posted = options.body;
    yield { response: new Response(null, { headers: { 'content-type': 'text/event-stream' } }) };
    yield { data: { type: 'done' } };
  };
  page.__elonChatGptPrivateStreamTransport = { preparePrivateSend: () => true,
    beginPrivateStream: () => ({ push() {}, finish() {} }), finishPrivateSend: () => calls.push('finish') };
  f.conversation.textHydrateHistory = async (_, options) => {
    calls.push('history');
    const userId = posted.messages[0].id;
    const payload = { conversation_id: conversationId, gizmo_id: null, is_do_not_remember: false,
      current_node: responseId, async_status: null, mapping: {
        [userId]: { id: userId, parent: parentId, message: { id: userId, author: { role: 'user' } } },
        [responseId]: { id: responseId, parent: userId, message: { id: responseId,
          author: { role: 'assistant' }, status: 'finished_successfully', end_turn: true } }
      } };
    options.onConversationLoadedFromNetwork(payload);
    assert.equal(options.shouldApplyResponse(), true);
    f.parent.id = responseId;
    f.shared.HM.getNodeIfExists = (_, id) => payload.mapping[id];
    f.shared.HM.getParentNode = () => ({ id: parentId });
    f.shared.HM.getParentPromptNode = () => ({ id: userId });
  };
  const api = asset('transaction').create(page, { context: f.context, requests: asset('request'),
    receipts: asset('receipts'), reconciliation: asset('reconcile').create() });
  const command = { requestId: 'mcp_cold1', prompt: 'Synthetic cold draft', expectedDraft: '', composer: null,
    readDraft: () => f.draft(), clearDraft() {} };
  const sent = api.send(command);
  try {
    await tick();
    assert.deepEqual(calls, []); assert.equal(api.state().pending, true);
    assert.equal((await api.send({ ...command, requestId: 'mcp_cold2' }).completion).code, 'busy');
    f.resolve();
    assert.equal((await sent.completion).status, 'accepted');
    await tick();
    assert.deepEqual(calls, ['prepare', 'post', 'history', 'finish']);
    assert.equal(api.state().pending, false);
    assert.equal(f.imports(), 1); assert.equal(f.view.dom.isConnected, false);
  } finally { api.dispose(); }
});

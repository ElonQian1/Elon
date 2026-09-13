'use strict';
const test = require('node:test'), assert = require('node:assert/strict'), crypto = require('node:crypto');
const assets = '../android/app/src/main/assets/';
const stopModule = require(assets + 'chatgpt_web_fresh_text_stop');
const requestModule = require(assets + 'chatgpt_web_fresh_text_request');
const history = require(assets + 'chatgpt_web_fresh_text_reconcile').create();
const CID = '11111111-1111-4111-8111-111111111111', PID = '22222222-2222-4222-8222-222222222222';
const AID = '44444444-4444-4444-8444-444444444444';
const tick = () => new Promise(resolve => setImmediate(resolve));
function fixture(options = {}) {
  const page = { crypto, AbortController, setTimeout, clearTimeout };
  let current = true, applied = false;
  const calls = [], binding = { conversationId: CID, parentId: PID, model: 'fixture-model', historyDisabled: false,
    doNotRemember: false, canStop: () => current, canReconcile: () => current, reconciled: () => applied,
    current: () => current, owns: () => current,
    shared: { v7: { STREAMING: 3 }, $3: key => key === '3922476776' ? options.enabled !== false : options.exclude !== false,
      textSecurityHeaders: () => ({ 'OpenAI-Sentinel-Chat-Requirements-Token': 'fixture-security' }),
      textApi: { safePost: async (url, init) => {
        calls.push({ url, init });
        if (url === '/f/conversation/prepare') return { conduit_token: 'fixture-fresh-conduit' };
        if (options.post) return options.post();
        payload.async_status = null; payload.mapping[AID].message.status = 'finished_partial_completion';
      } } },
    runtime: { async textHydrateHistory(id, config) {
      assert.equal(id, CID); assert.equal(config.forceNetworkFetch, true);
      if (options.read) await options.read();
      config.onConversationLoadedFromNetwork(payload);
      if (config.shouldApplyResponse()) applied = true;
    } }
  };
  const request = requestModule.create(page).create(binding, { requestId: 'mcp_stop1', prompt: 'Synthetic stop fixture' });
  const consumed = request.consume({ conduit_token: 'fixture-conduit-only' }, { chatReq: { token: 'fixture-security' } },
    () => ({ 'OpenAI-Sentinel-Chat-Requirements-Token': 'fixture-security' }), () => true);
  const UID = request.userMessageId;
  const payload = { conversation_id: CID, current_node: AID, async_status: 3, mapping: {
    [UID]: { id: UID, parent: PID, message: { id: UID, author: { role: 'user' } } },
    [AID]: { id: AID, parent: UID, message: { id: AID, author: { role: 'assistant' }, status: 'in_progress', end_turn: false } }
  } };
  const boundary = new AbortController();
  const owner = { binding, request, stopBoundary: boundary.signal, stopCurrent: () => current,
    stopReading() { calls.push({ kind: 'stop_reader' }); } };
  const api = stopModule.create(page, { reconciliation: history, timeoutMs: options.timeoutMs || 1000 });
  return { api, owner, page, binding, payload, calls, consumed, boundary, current: value => { current = value; } };
}

test('fresh stop sends one source-shaped POST with this request conduit and reconciles partial history', async () => {
  const f = fixture(), result = await f.api.stop(f.owner);
  assert.deepEqual(result, { status: 'accepted', code: 'stopped' });
  const posts = f.calls.filter(c => c.url); assert.equal(posts.length, 1);
  assert.equal(posts[0].url, '/stop_conversation');
  assert.deepEqual(posts[0].init.requestBody, { conversation_id: CID, exclude_async_types: ['pro_mode'] });
  assert.deepEqual(posts[0].init.additionalHeaders, { 'x-conduit-token': f.consumed.headers['x-conduit-token'],
    'x-oai-turn-trace-id': f.owner.request.turnId });
  assert.equal(posts[0].init.disableAutomaticRetry, true);
  assert.equal(f.owner.stopConfirmed, true); assert.equal(f.binding.reconciled(), true);
  assert.equal(JSON.stringify(result).includes('fixture-conduit'), false);
  assert.throws(() => f.owner.request.consumeStop([], () => true), /stop_ownership_unavailable/);
});

test('same transaction stop is single-flight', async () => {
  let release;
  const wait = new Promise(resolve => { release = resolve; });
  const f = fixture({ read: () => wait });
  const first = f.api.stop(f.owner); assert.equal(f.api.stop(f.owner), first);
  release(); assert.equal((await first).status, 'accepted');
  assert.equal(f.calls.filter(c => c.url).length, 1);
});

test('known terminal same-turn history does not send another stop', async () => {
  const f = fixture({ enabled: false }); f.payload.async_status = null;
  f.payload.mapping[AID].message.status = 'finished_successfully'; f.payload.mapping[AID].message.end_turn = true;
  assert.equal((await f.api.stop(f.owner)).code, 'already_stopped');
  assert.equal(f.calls.filter(c => c.url).length, 0);
});

test('provider gates are observed and chime version controls exclusion', async () => {
  const disabled = fixture({ enabled: false });
  assert.equal((await disabled.api.stop(disabled.owner)).code, 'stop_unavailable');
  assert.equal(disabled.calls.filter(c => c.url).length, 0);
  for (const chime of [false, true]) {
    const f = fixture({ exclude: chime });
    if (chime) f.payload.mapping[AID].message.metadata = { chime_version: 'fixture' };
    await f.api.stop(f.owner);
    assert.deepEqual(f.calls.find(c => c.url).init.requestBody.exclude_async_types, []);
  }
});

for (const [name, mutate] of [
  ['wrong conversation', f => { f.payload.conversation_id = PID; }],
  ['wrong parent', f => { f.payload.mapping[f.owner.request.userMessageId].parent = AID; }],
  ['sibling branch', f => { f.payload.mapping[AID].parent = PID; }],
  ['cycle', f => { f.payload.mapping[AID].parent = AID; }],
  ['voice async status', f => { f.payload.async_status = 5; }],
  ['unknown async status', f => { delete f.payload.async_status; }],
  ['account changed', f => f.current(false)]
]) test(name + ' cannot stop another request', async () => {
  const f = fixture(); mutate(f);
  assert.equal((await f.api.stop(f.owner)).status, 'unknown');
  assert.equal(f.calls.filter(c => c.url).length, 0);
});

test('timeout before POST ignores a late history response', async () => {
  let release;
  const wait = new Promise(resolve => { release = resolve; });
  const f = fixture({ read: () => wait, timeoutMs: 15 });
  assert.equal((await f.api.stop(f.owner)).code, 'stop_timeout');
  release(); await tick(); assert.equal(f.calls.filter(c => c.url).length, 0);
});

test('document boundary aborts pending stop without a late write', async () => {
  let release;
  const wait = new Promise(resolve => { release = resolve; });
  const f = fixture({ read: () => wait });
  const done = f.api.stop(f.owner); f.current(false); f.boundary.abort();
  assert.equal((await done).code, 'context_changed');
  release(); await tick(); assert.equal(f.calls.filter(c => c.url).length, 0);
});

test('unknown stop POST is never repeated; next explicit stop only verifies history', async () => {
  const f = fixture({ post: async () => { throw Error('fixture network lost'); } });
  assert.equal((await f.api.stop(f.owner)).code, 'stop_unconfirmed');
  assert.equal(f.owner.stopAttempted, true); assert.equal(f.owner.stopConfirmed, undefined);
  f.payload.async_status = null; f.payload.mapping[AID].message.status = 'finished_partial_completion';
  assert.equal((await f.api.stop(f.owner)).status, 'accepted');
  assert.equal(f.calls.filter(c => c.url).length, 1);
});

test('successful stop response without terminal history remains unconfirmed', async () => {
  const f = fixture({ post: async () => {} });
  assert.equal((await f.api.stop(f.owner)).code, 'stop_reconciliation_pending');
  assert.equal(f.owner.stopAcknowledged, true); assert.equal(f.owner.stopConfirmed, undefined);
  assert.equal(f.binding.reconciled(), false); assert.equal(f.calls.filter(c => c.url).length, 1);
});

test('stop before any assistant text uses the owned user leaf and requires server acknowledgement', async () => {
  for (const confirmed of [false, true]) {
    const f = fixture({ post: async () => { f.payload.async_status = null; if (!confirmed) throw Error('lost response'); } });
    f.payload.current_node = f.owner.request.userMessageId; delete f.payload.mapping[AID];
    assert.equal((await f.api.stop(f.owner)).status, confirmed ? 'accepted' : 'unknown');
    assert.equal(f.calls.filter(c => c.url).length, 1);
    if (!confirmed) assert.equal((await f.api.stop(f.owner)).status, 'unknown');
  }
});

function delayedUserFixture(options = {}) {
  let reads = 0;
  const f = fixture({ ...options, read: async () => {
    reads++;
    if (reads === (options.visibleAt ?? 2)) {
      const uid = f.owner.request.userMessageId;
      f.payload.mapping[uid] = { id: uid, parent: PID, message: { id: uid, author: { role: 'user' } } };
      f.payload.current_node = uid;
    }
    await options.afterRead?.(f, reads);
  }, post: async () => {
    f.payload.async_status = null;
    if (options.loseStop) throw Error('synthetic lost stop acknowledgement');
  } });
  f.payload.current_node = PID;
  f.payload.mapping = { [PID]: { id: PID, parent: null, message: { id: PID,
    author: { role: 'assistant' }, status: 'finished_successfully', end_turn: true } } };
  return { ...f, reads: () => reads };
}

test('early stop waits briefly for this submitted user before one server stop', async () => {
  const f = delayedUserFixture();
  const first = f.api.stop(f.owner);
  assert.equal(f.api.stop(f.owner), first);
  assert.deepEqual(await first, { status: 'accepted', code: 'stopped' });
  assert.equal(f.calls.filter(c => c.url === '/stop_conversation').length, 1);
  assert.equal(f.reads(), 3);
  assert.equal(f.owner.stopAcknowledged, true);
  assert.equal(f.owner.stopConfirmed, true);
});

test('early stop leaves an absent submitted user unconfirmed after bounded reads', async () => {
  const f = delayedUserFixture({ visibleAt: 99 });
  assert.equal((await f.api.stop(f.owner)).code, 'stop_owner_unconfirmed');
  assert.equal(f.reads(), 3);
  assert.equal(f.calls.filter(c => c.url).length, 0);
});

test('an early stop with a lost acknowledgement cannot release the user-only writer', async () => {
  const f = delayedUserFixture({ loseStop: true });
  assert.equal((await f.api.stop(f.owner)).code, 'stop_unconfirmed');
  assert.equal((await f.api.stop(f.owner)).code, 'stop_reconciliation_pending');
  assert.equal(f.calls.filter(c => c.url).length, 1);
  assert.notEqual(f.owner.stopConfirmed, true);
});

for (const change of [
  f => { f.payload.conversation_id = PID; },
  f => { f.payload.current_node = AID; },
  f => { f.payload.mapping[PID].message.author.role = 'tool'; },
  f => { f.payload.mapping[PID].message.status = 'in_progress'; },
  f => { f.payload.async_status = 5; },
  f => { delete f.payload.async_status; },
  f => { f.binding.operation = 'regenerate'; }
]) test('unrelated or unsupported history is not an early-send propagation wait', async () => {
  const f = delayedUserFixture(); change(f);
  assert.equal((await f.api.stop(f.owner)).status, 'unknown');
  assert.equal(f.reads(), 1);
  assert.equal(f.calls.filter(c => c.url).length, 0);
});

test('owner change while awaiting the submitted user never sends a stop', async () => {
  const f = delayedUserFixture({ afterRead(f, count) { if (count === 2) f.current(false); } });
  assert.equal((await f.api.stop(f.owner)).code, 'context_changed');
  assert.equal(f.calls.filter(c => c.url).length, 0);
});

test('timeout during propagation leaves no late stop or released writer', async () => {
  const f = delayedUserFixture({ visibleAt: 99 });
  const api = stopModule.create(f.page, { reconciliation: history, timeoutMs: 20 });
  assert.equal((await api.stop(f.owner)).code, 'stop_timeout');
  await new Promise(resolve => setTimeout(resolve, 220));
  assert.equal(f.reads(), 1);
  assert.equal(f.calls.filter(c => c.url).length, 0);
  assert.notEqual(f.owner.stopConfirmed, true);
});

for (const mode of ['partial assistant', 'user-only', 'propagating user']) test('owned stop and follow-up cross the real transaction with ' + mode + ' history', async () => {
  const empty = mode !== 'partial assistant', delayed = mode === 'propagating user';
  let historyReads = 0, pendingUser;
  const transaction = require(assets + 'chatgpt_web_fresh_text_transaction');
  const f = fixture({ read: () => {
    if (delayed && ++historyReads === 2) {
      f.payload.mapping[pendingUser.id] = pendingUser;
      f.payload.current_node = pendingUser.id;
    }
  }, post: async () => {
    f.payload.async_status = null;
    if (empty) delete f.payload.mapping[AID];
    else f.payload.mapping[AID].message.status = 'finished_partial_completion';
  } });
  Object.assign(f.page, { document: {}, location: { href: 'https://chatgpt.com/c/' + CID },
    __elonChatGptDocumentToken: 'doc_fresh_stop', __elonChatGptPrivateTextTransactionsEnabled: true,
    __elonChatGptFreshTextDispatchEnabled: true,
    __elonChatGptFreshTextStream: require(assets + 'chatgpt_web_fresh_text_stream'),
    __elonChatGptPrivateStreamTransport: { preparePrivateSend: () => true,
      beginPrivateStream: () => ({ push() {}, finish() {} }) } });
  f.binding.shared.textTopic = () => { throw Error('unexpected_topic'); };
  let release, sentId, captureCount = 0, postCount = 0;
  const streamWait = new Promise(resolve => { release = resolve; });
  f.binding.runtime.textSecurity = () => ({ chatReq: { token: 'fixture-security' } });
  f.binding.runtime.textStream = (_, init) => (async function* () {
    init.onBeforeRequestStart();
    const uid = init.body.messages[0].id;
    if (++postCount === 2) {
      assert.equal(init.body.parent_message_id, empty ? sentId : AID);
      yield { response: new Response(null, { headers: { 'content-type': 'text/event-stream' } }) };
      return;
    }
    sentId = uid;
    pendingUser = { id: uid, parent: PID, message: { id: uid, author: { role: 'user' } } };
    if (delayed) {
      f.payload.mapping = { [PID]: { id: PID, message: { id: PID, author: { role: 'assistant' },
        status: 'finished_successfully', end_turn: true } } };
      f.payload.current_node = PID;
    } else f.payload.mapping[uid] = pendingUser;
    if (empty && !delayed) f.payload.current_node = uid;
    else if (!empty) f.payload.mapping[AID].parent = uid;
    yield { response: new Response(null, { headers: { 'content-type': 'text/event-stream' } }) };
    await streamWait;
  })();
  const api = transaction.create(f.page, { requests: requestModule, reconciliation: history, stopping: f.api,
    receipts: require(assets + 'chatgpt_web_fresh_text_receipts'),
    context: { capture: async (_, stoppedParent) => {
      if (++captureCount === 1) { assert.equal(stoppedParent, null); return f.binding; }
      assert.equal(stoppedParent.id, sentId); assert.equal(stoppedParent.current(), true);
      if (captureCount === 2) throw Error('runtime_unavailable');
      return { ...f.binding, parentId: empty ? sentId : AID, parentRole: empty ? 'user' : 'assistant' };
    }, stamp: () => 'fixture-owner' } });
  const sent = api.send({ requestId: 'mcp_lifecycle', prompt: 'fixture', expectedDraft: '', readDraft: () => '' });
  assert.equal((await sent.completion).status, 'accepted');
  const stop = api.stop(); assert.equal(api.state().pending, true);
  assert.equal((await stop.completion).status, 'accepted');
  await tick(); assert.equal(api.state().pending, false);
  assert.equal(f.calls.filter(c => c.url === '/stop_conversation').length, 1);
  assert.equal(f.calls.filter(c => c.url === '/f/conversation/prepare').length, 1);
  release(); await tick(); assert.equal(api.state().pending, false);
  const deferred = api.send({ requestId: 'mcp_deferred', prompt: 'Next fixture', expectedDraft: '', readDraft: () => '' });
  assert.equal((await deferred.completion).status, 'unavailable'); await tick();
  assert.equal(postCount, 1);
  const next = api.send({ requestId: 'mcp_next', prompt: 'Next fixture', expectedDraft: '', readDraft: () => '' });
  assert.equal((await next.completion).status, 'accepted');
  await tick(); assert.equal(postCount, 2);
  assert.equal(f.calls.filter(c => c.url === '/stop_conversation').length, 1);
  assert.equal(f.calls.filter(c => c.url === '/f/conversation/prepare').length, 2);
  assert.equal(api.trialControl('state').parent_role, empty ? 'user' : 'assistant');
});

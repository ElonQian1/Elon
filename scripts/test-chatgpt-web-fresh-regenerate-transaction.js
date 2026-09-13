'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const { fixture: contextFixture, CID, PID, UID, AID, OTHER } = require('./fixtures/chatgpt-fresh-regeneration');
const { install } = require('./fixtures/chatgpt-owned-stream-transport');
const load = name => require('../android/app/src/main/assets/chatgpt_web_fresh_text_' + name);
const tick = () => new Promise(resolve => setImmediate(resolve));
async function settle() { for (let i = 0; i < 12; i++) await tick(); }

function fixture(options = {}) {
  const f = contextFixture(), calls = [], { page, shared, conversation } = f;
  let draft = 'Synthetic unsent draft', body, stopped = false,
    historyId = options.historyId || AID, streamId = options.streamId || AID;
  Object.assign(page, { crypto, AbortController, setTimeout, clearTimeout,
    __elonChatGptPrivateTextTransactionsEnabled: true,
    __elonChatGptFreshRegenerationEnabled: options.enabled !== false, __elonChatGptFreshTextStream: load('stream') });
  page.document.visibilityState = 'visible';
  install(page);
  shared.textTopic = () => { throw Error('unexpected topic'); };
  shared.$3 = () => true;
  shared.textApi.safePost = async (path, request) => {
    if (path === '/stop_conversation') {
      calls.push({ kind: 'stop', path, request }); stopped = true; return {};
    }
    calls.push({ kind: 'prepare', path, request });
    await options.prepare?.(f);
    return { conduit_token: 'synthetic-fresh-conduit' };
  };
  shared.textSecurityHeaders = value => ({ 'openai-sentinel-chat-requirements-token': value.token });
  conversation.textSecurity = () => ({ chatReq: { token: 'synthetic-proof' } });
  conversation.textStream = (url, request) => (async function* () {
    request.onBeforeRequestStart(); body = structuredClone(request.body);
    calls.push({ kind: 'post', url });
    options.afterPost?.(f);
    if (options.responseLost) throw Error('synthetic-lost-response');
    yield { response: new Response(null, { headers: { 'content-type': 'text/event-stream' } }) };
    if (options.beforeData) await options.beforeData(f);
    yield { data: { conversation_id: CID, message: streamId === PID ? f.parent :
      f.reply(streamId, options.waitForStop ? 'in_progress' : 'finished_successfully') } };
    if (options.waitForStop) await new Promise((_, reject) => {
      request.signal.addEventListener('abort', () => reject(Error('cancelled')), { once: true });
    });
    if (options.afterDataLost) throw Error('synthetic-lost-tail');
    yield { data: { type: 'message_stream_complete' } };
  })();
  conversation.textHydrateHistory = async (_, request) => {
    calls.push({ kind: 'history' });
    if (options.history === false) return;
    const payload = f.history(historyId);
    if (options.waitForStop) {
      payload.mapping[historyId].message = f.reply(historyId, stopped ? 'finished_partial_completion' : 'in_progress');
      payload.async_status = stopped ? null : 3;
    }
    request.onConversationLoadedFromNetwork(payload);
    if (request.shouldApplyResponse()) f.apply(payload);
  };
  const reconciliation = load('reconcile').create();
  const recovery = load('recovery').create(page, { reconciliation, delays: [0], timeoutMs: 1000, cooldownMs: 0 });
  const api = load('transaction').create(page, { context: f.api, requests: load('request'),
    receipts: load('receipts'), recovery, reconciliation,
    stopping: load('stop').create(page, { reconciliation, timeoutMs: 1000 }),
    prepareTimeoutMs: 1000, openTimeoutMs: 1000, streamTimeoutMs: 1000 });
  const command = { ...f.command, expectedDraft: draft, readDraft: () => draft,
    clearDraft: () => { draft = ''; calls.push({ kind: 'clear_draft' }); } };
  return Object.assign(f, { api, command, calls, body: () => body, draft: value => {
    if (value !== undefined) draft = value; return draft;
  }, historyId: value => { historyId = value; },
  close() { page.__elonChatGptDocumentToken = 'new-document'; api.state(); assert.equal(api.dispose(), true);
    page.__elonChatGptPrivateStreamTransport.dispose(); } });
}

test('private regeneration posts one variant, streams natively and reconciles the original user branch', async () => {
  const f = fixture(), sent = f.api.regenerate(f.command);
  assert.equal((await sent.completion).status, 'accepted');
  await settle();
  assert.equal(f.body().action, 'variant'); assert.equal('messages' in f.body(), false);
  assert.equal(f.body().parent_message_id, UID); assert.equal(f.body().thinking_effort, 'high');
  assert.equal(f.calls.filter(c => c.kind === 'post').length, 1);
  assert.equal(f.retry.calls.length, 0); assert.equal(f.draft(), 'Synthetic unsent draft');
  assert.equal(f.api.state().pending, false); assert.equal(f.tree.leaf, AID);
  assert.equal(f.api.trialControl('state').operation, 'regenerate');
  assert.equal(Object.values(f.tree.nodes).filter(n => n.message?.author?.role === 'user').length, 1);
  assert.equal(f.api.regenerate(f.command), sent);
  assert.equal((await f.api.send({ ...f.command, prompt: f.draft() }).completion).code, 'request_id_conflict');
  f.close();
});

test('default-off leaves the accepted retry alone; a one-command trial does not enable the default', async () => {
  const f = fixture({ enabled: false });
  assert.equal(f.api.regenerate(f.command).handled, false); assert.equal(f.calls.length, 0);
  assert.equal(f.api.trialControl('start').armed, true);
  assert.equal((await f.api.regenerate(f.command).completion).status, 'accepted');
  await settle();
  assert.equal(f.api.trialControl('state').armed, false);
  assert.equal(f.page.__elonChatGptFreshRegenerationEnabled, false);
  f.close();
});

test('a selected-file scope gap may fall back once before preparation', async () => {
  const f = fixture(); f.files.files$ = () => [{ id: 'synthetic-pending' }];
  const sent = f.api.regenerate(f.command);
  assert.equal((await sent.completion).status, 'unavailable');
  assert.equal(f.calls.length, 0); assert.equal(sent.claimFallback(), true);
  assert.equal(sent.claimFallback(), false); f.close();
});

test('a changed draft during fresh preparation rejects without posting or erasing it', async () => {
  let f;
  f = fixture({ prepare: () => f.draft('New unsent draft') });
  const sent = f.api.regenerate(f.command);
  assert.equal((await sent.completion).status, 'rejected');
  assert.equal(sent.claimFallback(), false); assert.equal(f.calls.filter(c => c.kind === 'post').length, 0);
  assert.equal(f.draft(), 'New unsent draft'); f.close();
});

test('lost response cannot mistake an old or another-device variant for delivery and cannot replay', async () => {
  for (const historyId of [PID, OTHER]) {
    const f = fixture({ responseLost: true, historyId }), sent = f.api.regenerate(f.command);
    assert.equal((await sent.completion).status, 'unknown'); await settle();
    assert.equal(f.api.state().pending, true); assert.equal(f.tree.leaf, PID);
    assert.equal(sent.claimFallback(), false); assert.equal(f.api.regenerate(f.command), sent);
    assert.equal((await f.api.regenerate({ ...f.command, requestId: 'mcp_regen2' }).completion).code, 'busy');
    assert.equal((await f.api.send({ ...f.command, requestId: 'mcp_send2', prompt: f.draft() }).completion).code, 'busy');
    assert.equal(f.calls.filter(c => c.kind === 'post').length, 1); f.close();
  }
});

test('a lost stream tail can recover from the exact observed reply without a second POST', async () => {
  const f = fixture({ afterDataLost: true });
  assert.equal((await f.api.regenerate(f.command).completion).status, 'accepted');
  await settle();
  assert.equal(f.api.state().pending, false); assert.equal(f.tree.leaf, AID);
  assert.equal(f.calls.filter(c => c.kind === 'post').length, 1); f.close();
});

test('old reply frames are rejected before projection, not just after history reconciliation', async () => {
  const f = fixture({ streamId: PID, historyId: PID });
  f.page.document.visibilityState = 'hidden';
  await f.api.regenerate(f.command).completion; await settle();
  assert.equal(f.api.state().pending, true); assert.equal(f.api.state().code, 'stream_owner_changed');
  assert.equal(f.page.__elonChatGptPrivateStreamTransport.current('/c/' + CID), null);
  f.close();
});

test('another device history is not applied; later matching history recovers the owned variant', async () => {
  const f = fixture({ historyId: OTHER });
  await f.api.regenerate(f.command).completion; await settle();
  assert.equal(f.tree.leaf, PID); assert.equal(f.api.state().pending, true);
  assert.equal(f.page.__elonChatGptPrivateStreamTransport.current('/c/' + CID).id, AID);
  f.historyId(AID);
  assert.equal((await f.api.recover().completion).status, 'accepted'); await settle();
  assert.equal(f.tree.leaf, AID); assert.equal(f.api.state().pending, false);
  assert.equal(f.calls.filter(c => c.kind === 'post').length, 1); f.close();
});

test('leaving the conversation prevents late stream projection and history application', async () => {
  const f = fixture({ afterPost: f => { f.page.location.href = 'https://chatgpt.com/c/' + OTHER; } });
  await f.api.regenerate(f.command).completion; await settle();
  assert.equal(f.page.__elonChatGptPrivateStreamTransport.current('/c/' + CID), null);
  assert.equal(f.tree.leaf, PID); assert.equal(f.calls.filter(c => c.kind === 'history').length, 0);
  f.close();
});

test('native stop uses this variant preparation and preserves the owned partial answer', async () => {
  const f = fixture({ waitForStop: true });
  assert.equal((await f.api.regenerate(f.command).completion).status, 'accepted');
  await settle();
  assert.equal(f.page.__elonChatGptPrivateStreamTransport.current('/c/' + CID).id, AID);
  const stop = f.api.stop();
  assert.equal((await stop.completion).status, 'accepted');
  await settle();
  const request = f.calls.find(c => c.kind === 'stop').request;
  assert.equal(request.requestBody.conversation_id, CID);
  assert.equal(request.additionalHeaders['x-conduit-token'], 'synthetic-fresh-conduit');
  assert.equal(f.tree.leaf, AID); assert.equal(f.tree.nodes[AID].message.status, 'finished_partial_completion');
  assert.equal(f.api.state().pending, false);
  assert.equal(f.draft(), 'Synthetic unsent draft');
  assert.equal(f.calls.filter(c => c.kind === 'post').length, 1);
  assert.equal(f.calls.filter(c => c.kind === 'stop').length, 1);
  f.close();
});

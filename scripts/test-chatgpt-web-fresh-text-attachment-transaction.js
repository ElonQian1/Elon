'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const path = require('node:path');
const { fixture: attachmentsFixture, CID, PID } = require('./fixtures/chatgpt-fresh-text-attachments');
const asset = name => require(path.join('..', 'android/app/src/main/assets', 'chatgpt_web_fresh_text_' + name));
const turn = () => new Promise(resolve => setImmediate(resolve));
const settled = async () => { for (let i = 0; i < 8; i++) await turn(); };
const command = { requestId: 'mcp_attach1', prompt: 'Synthetic attachment question', expectedDraft: '' };

async function fixture(options = {}) {
  const f = await attachmentsFixture(), calls = [];
  const { page, shared, conversation, files } = f;
  Object.assign(page, { crypto, AbortController, setTimeout, clearTimeout,
    __elonChatGptPrivateTextTransactionsEnabled: true, __elonChatGptFreshTextAttachmentsEnabled: options.enabled === true });
  page.document.visibilityState = 'visible';
  page.__elonChatGptFreshTextStream = asset('stream');
  let posted, applied = null, missingReference = false;
  shared.textTopic = () => { throw Error('unexpected_topic'); };
  shared.textApi.safePost = async (url, value) => {
    calls.push({ kind: 'prepare', url, value });
    if (options.prepare) await options.prepare(f);
    return { conduit_token: 'synthetic-conduit' };
  };
  shared.textSecurityHeaders = value => ({ 'openai-sentinel-chat-requirements-token': value.token });
  conversation.textSecurity = () => ({ chatReq: { token: 'synthetic-proof' } });
  conversation.textStream = (url, request) => (async function* () {
    request.onBeforeRequestStart();
    posted = structuredClone(request.body);
    calls.push({ kind: 'post', url, request });
    options.afterPost?.(f);
    if (options.responseLost) throw Error('synthetic-response-lost');
    yield { response: new Response(null, { headers: { 'content-type': 'text/event-stream' } }) };
    yield { data: { type: 'message_stream_complete' } };
  })();
  function payload() {
    const user = structuredClone(posted.messages[0]), id = crypto.randomUUID();
    if (missingReference) user.metadata.attachments.shift();
    return { conversation_id: CID, current_node: id, mapping: {
      [PID]: { id: PID, parent: null, message: f.parent },
      [user.id]: { id: user.id, parent: PID, message: user },
      [id]: { id, parent: user.id, message: { id, author: { role: 'assistant' },
        status: 'finished_successfully', end_turn: true, content: { content_type: 'text', parts: ['Synthetic reply'] } } }
    } };
  }
  shared.HM.getCurrentMessage = () => applied ? applied.mapping[applied.current_node].message : f.parent;
  shared.HM.getNodeIfExists = (_, id) => applied?.mapping[id];
  shared.HM.getParentNode = (_, id) => applied?.mapping[applied.mapping[id]?.parent];
  shared.HM.getParentPromptNode = (_, id) => applied?.mapping[applied.mapping[id]?.parent];
  conversation.textHydrateHistory = async (id, value) => {
    calls.push({ kind: 'history', id });
    if (options.history === false) return;
    const data = payload(); value.onConversationLoadedFromNetwork(data);
    if (value.shouldApplyResponse()) applied = data;
  };
  page.__elonChatGptPrivateStreamTransport = {
    preparePrivateSend: () => true, beginPrivateStream: () => ({ push() {}, finish() {} }), finishPrivateSend() {}
  };
  const reconciliation = asset('reconcile').create();
  const recovery = asset('recovery').create(page, { reconciliation, delays: [0], timeoutMs: 1000 });
  const api = asset('transaction').create(page, { context: f.api, requests: asset('request'),
    receipts: asset('receipts'), recovery, reconciliation,
    prepareTimeoutMs: 1000, openTimeoutMs: 1000, streamTimeoutMs: 1000 });
  return Object.assign(f, { api, calls, files, send: input => api.send({ ...command, composer: f.node,
    readDraft: () => '', ...input }), posted: () => posted, missing: value => { missingReference = value; } });
}

test('attachment extension is opt-in and a one-command trial uses the actual private lease', async () => {
  const f = await fixture();
  const before = f.send();
  assert.equal((await before.completion).status, 'unavailable');
  assert.equal(before.claimFallback(), true);
  assert.equal(f.calls.length, 0); assert.equal(f.files.files$().length, 3);
  await settled();
  assert.equal(f.api.trialControl('start').armed, true);
  const trial = f.send({ requestId: 'mcp_attach2' });
  assert.equal((await trial.completion).status, 'accepted');
  await settled();
  assert.equal(f.api.state().pending, false);
  assert.equal(f.api.trialControl('state').reconciled, true);
  assert.equal(f.page.__elonChatGptFreshTextAttachmentsEnabled, false);
  assert.equal(f.api.trialControl('state').armed, false);
  assert.deepEqual(f.calls.find(call => call.kind === 'prepare').value.requestBody.attachment_mime_types,
    ['text/plain', 'application/pdf', 'image/png']);
  assert.equal(f.posted().messages[0].metadata.attachments.length, 3);
  assert.equal(f.files.files$().length, 0);
  assert.equal(f.send({ requestId: 'mcp_attach2' }), trial);
  assert.equal(f.calls.filter(call => call.kind === 'post').length, 1);
  assert.equal(f.api.dispose(), true);
});

test('HTTP acceptance retires submitted entries but preserves a file added after dispatch', async () => {
  const later = { tempId: 'later', status: 'uploading' };
  const f = await fixture({ enabled: true, afterPost: f => f.files.files$.set([...f.files.files$(), later]) });
  assert.equal((await f.send().completion).status, 'accepted');
  assert.deepEqual(f.files.files$(), [later]);
  await settled();
  assert.equal(f.api.state().pending, false);
  assert.deepEqual(f.files.files$(), [later]);
  assert.equal(f.api.dispose(), true);
});

test('changed attachment selection during preparation rejects without posting or clearing anything', async () => {
  const later = { tempId: 'later', status: 'uploading' };
  const f = await fixture({ enabled: true, prepare: f => f.files.files$.set([...f.files.files$(), later]) });
  const sent = f.send();
  assert.equal((await sent.completion).status, 'rejected');
  assert.equal(sent.claimFallback(), false);
  await settled();
  assert.equal(f.calls.filter(call => call.kind === 'post').length, 0);
  assert.equal(f.files.files$().length, 4); assert.equal(f.api.state().pending, false);
  assert.equal(f.api.dispose(), true);
});

test('lost HTTP response retains attachments until exact history proves acceptance, never resends', async () => {
  const f = await fixture({ enabled: true, responseLost: true, history: false });
  const sent = f.send();
  assert.equal((await sent.completion).status, 'unknown');
  await settled();
  assert.equal(f.files.files$().length, 3); assert.equal(f.api.state().pending, true);
  assert.equal(sent.claimFallback(), false); assert.equal(f.send(), sent);
  assert.equal((await f.send({ requestId: 'mcp_blocked' }).completion).code, 'busy');
  assert.equal(f.calls.filter(call => call.kind === 'post').length, 1);
  f.page.__elonChatGptDocumentToken = 'new-document'; f.api.state();
  assert.equal(f.api.dispose(), true);

  const g = await fixture({ enabled: true, responseLost: true });
  assert.equal((await g.send().completion).status, 'unknown');
  await settled();
  assert.equal(g.api.state().pending, false);
  assert.equal(g.files.files$().length, 0);
  assert.equal(g.api.trialControl('state').reconciled, true);
  assert.equal(g.calls.filter(call => call.kind === 'post').length, 1);
  assert.equal(g.api.dispose(), true);
});

test('cleanup failure holds the completed writer until local cleanup succeeds without network replay', async () => {
  let original;
  const f = await fixture({ enabled: true, afterPost: f => {
    original = f.files.files$.set; f.files.files$.set = () => {};
  } });
  assert.equal((await f.send().completion).status, 'accepted');
  await settled();
  assert.equal(f.api.state().pending, true);
  assert.equal(f.api.state().code, 'attachment_cleanup_pending');
  assert.equal(f.api.hasCurrentWriter(), false);
  assert.equal(f.files.files$().length, 3);
  assert.equal((await f.send({ requestId: 'mcp_cleanup2' }).completion).code, 'busy');
  const reads = f.calls.filter(call => call.kind === 'history').length;
  f.files.files$.set = original;
  assert.equal(f.api.state().pending, false);
  assert.equal(f.files.files$().length, 0);
  assert.equal(f.calls.filter(call => call.kind === 'history').length, reads);
  assert.equal(f.calls.filter(call => call.kind === 'post').length, 1);
  assert.equal(f.api.dispose(), true);
});

test('matching text IDs without all file references cannot reconcile or release an uncertain writer', async () => {
  const f = await fixture({ enabled: true, responseLost: true }); f.missing(true);
  assert.equal((await f.send().completion).status, 'unknown');
  await settled();
  assert.equal(f.api.state().pending, true); assert.equal(f.files.files$().length, 3);
  assert.equal(f.api.trialControl('state').history, 'branch_mismatch');
  f.missing(false);
  assert.equal((await f.api.recover().completion).status, 'accepted');
  await settled();
  assert.equal(f.api.state().pending, false); assert.equal(f.files.files$().length, 0);
  assert.equal(f.calls.filter(call => call.kind === 'post').length, 1);
  assert.equal(f.api.dispose(), true);
});

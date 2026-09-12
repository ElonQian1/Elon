'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const path = require('node:path');
const asset = name => require(path.join('..', 'android/app/src/main/assets', name));
const requestModule = asset('chatgpt_web_fresh_text_request');
const transactionModule = asset('chatgpt_web_fresh_text_transaction');
const CID = '11111111-1111-4111-8111-111111111111';
const PARENT = '22222222-2222-4222-8222-222222222222';
const base = { conversationId: CID, parentId: PARENT, model: 'fixture-model',
  effort: 'high', serviceTier: 'standard', historyDisabled: false, doNotRemember: false };
const security = { chatReq: { token: 'fixture-fresh-only' }, proofToken: 'fixture-proof' };
const headers = req => ({ 'OpenAI-Sentinel-Chat-Requirements-Token': req.token });
const command = { requestId: 'mcp_fresh1', prompt: 'Synthetic fixture prompt', expectedDraft: '' };
const turn = () => new Promise(resolve => setImmediate(resolve));
const deferred = () => { let resolve, reject; const promise = new Promise((a, b) => { resolve = a; reject = b; }); return { promise, resolve, reject }; };

function fixture(options = {}) {
  const calls = [], page = { document: {}, __elonChatGptDocumentToken: 'doc_fresh_test',
    location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/' + CID },
    __elonChatGptPrivateTextTransactionsEnabled: true, __elonChatGptFreshTextDispatchEnabled: true,
    crypto, AbortController, setTimeout, clearTimeout };
  let current = true, reconciled = false, draft = '';
  const binding = { ...base, token: page.__elonChatGptDocumentToken,
    current: () => current, owns: () => current, reconciled: () => reconciled,
    shared: { textApi: { safePost: async (url, value) => {
      calls.push({ kind: 'prepare', url, value });
      return options.prepare ? options.prepare() : { conduit_token: 'fixture-conduit' };
    } }, textSecurityHeaders: headers },
    runtime: {
      textSecurity: () => { calls.push({ kind: 'security' }); return options.security ? options.security() : security; },
      textStream: (url, value) => {
        calls.push({ kind: 'stream_setup', url, value });
        if (options.stream) return options.stream(value);
        return (async function* () {
          value.onBeforeRequestStart();
          calls.push({ kind: 'post' });
          yield { response: new Response(null, { headers: { 'content-type': 'text/event-stream' } }) };
          yield { data: { type: 'done' } };
        })();
      }
    }
  };
  page.__elonChatGptPrivateStreamTransport = { preparePrivateSend(prompt, id) {
    calls.push({ kind: 'native_stream', prompt, id }); return true;
  } };
  const api = transactionModule.create(page, { requests: requestModule,
    reconciliation: { reconcile: options.reconciliation || (async () => false) },
    context: { capture: options.capture || (async () => binding), stamp: () => current ? 'fixture-stamp' : 'changed' },
    prepareTimeoutMs: options.prepareTimeoutMs || 1000, openTimeoutMs: options.openTimeoutMs || 1000,
    streamTimeoutMs: options.streamTimeoutMs || 1000, reconcileTimeoutMs: options.reconcileTimeoutMs || 1000 });
  function send(input = {}) {
    return api.send({ ...command, readDraft: () => draft, clearDraft: () => { draft = ''; }, ...input });
  }
  return { api, page, calls, binding, send, setCurrent: value => { current = value; },
    reconcile: () => { reconciled = true; }, draft: value => { if (value !== undefined) draft = value; return draft; } };
}

test('fresh request projects current plain text, not captured message/proof state', () => {
  const builder = requestModule.create({ crypto });
  const first = builder.create(base, command), second = builder.create(base, { ...command, requestId: 'mcp_fresh2' });
  assert.notEqual(first.userMessageId, second.userMessageId);
  assert.notEqual(first.turnId, second.turnId);
  const preparation = first.preparationBody();
  assert.equal(preparation.messages, undefined);
  assert.equal(preparation.parent_message_id, PARENT);
  assert.equal(preparation.model, 'fixture-model');
  preparation.parent_message_id = 'wrong';
  const request = first.consume({ conduit_token: 'fixture-conduit' }, security, headers, () => true);
  assert.equal(request.body.parent_message_id, PARENT);
  assert.equal(request.body.messages.length, 1);
  assert.equal(request.body.messages[0].id, first.userMessageId);
  assert.deepEqual(request.body.messages[0].content, { content_type: 'text', parts: [command.prompt] });
  assert.equal(request.body.thinking_effort, 'high');
  assert.equal(request.body.service_tier, 'standard');
  assert.equal(request.body.force_use_search, undefined);
  assert.equal(request.body.history_and_training_disabled, undefined);
  assert.throws(() => first.consume({ conduit_token: 'other' }, security, headers, () => true), /consumed/);
});

test('fresh builder preserves privacy and refuses login enforcement or stale ownership', () => {
  const builder = requestModule.create({ crypto });
  const make = () => builder.create({ ...base, historyDisabled: true, doNotRemember: true }, command);
  const request = make().consume({ conduit_token: 'fixture' }, security, headers, () => true);
  assert.equal(request.body.history_and_training_disabled, true);
  assert.equal(request.body.is_do_not_remember, true);
  assert.throws(() => make().consume({ conduit_token: 'fixture' }, { chatReq: { token: 'fixture', force_login: true } }, headers, () => true), /login_required/);
  assert.throws(() => make().consume({ conduit_token: 'fixture' }, security, headers, () => false), /context_changed/);
  assert.throws(() => make().consume({}, security, headers, () => true), /prepare_unconfirmed/);
  assert.throws(() => make().consume({ conduit_token: 'fixture' }, security, () => ({ authorization: 'fixture' }), () => true), /security_invalid/);
});

test('independent transaction prepares once, obtains fresh security, then posts once', async () => {
  const f = fixture();
  const sent = f.send();
  assert.equal(sent.handled, true);
  assert.equal((await sent.completion).status, 'accepted');
  await turn();
  assert.deepEqual(f.calls.map(x => x.kind), ['prepare', 'security', 'stream_setup', 'native_stream', 'post']);
  const prepare = f.calls[0];
  assert.equal(prepare.url, '/f/conversation/prepare');
  assert.equal(prepare.value.disableAutomaticRetry, true);
  const post = f.calls.find(x => x.kind === 'stream_setup');
  assert.equal(post.url, 'https://chatgpt.com/backend-api/f/conversation');
  assert.equal(post.value.method, 'POST');
  assert.equal(post.value.shouldRetry, undefined);
  assert.equal(f.api.state().phase, 'reconciling');
  assert.equal(f.api.state().pending, true);
  assert.equal(sent.claimFallback(), false);
  assert.equal(f.send(), sent);
  assert.equal((await f.send({ prompt: 'different' }).completion).code, 'request_id_conflict');
  assert.equal((await f.send({ requestId: 'mcp_second' }).completion).code, 'busy');
  f.reconcile();
  assert.equal(f.api.state().pending, false);
});

test('trial disabled does not load runtime or intercept accepted sender', () => {
  const f = fixture(); f.page.__elonChatGptFreshTextDispatchEnabled = false;
  assert.equal(f.send().handled, false); assert.equal(f.calls.length, 0);
});

test('successful authoritative reconciliation releases the next native send', async () => {
  const f = fixture({ reconciliation: async () => { f.reconcile(); return true; } });
  assert.equal((await f.send().completion).status, 'accepted'); await turn();
  assert.equal(f.api.state().pending, false);
  assert.equal((await f.send({ requestId: 'mcp_second' }).completion).status, 'accepted'); await turn();
  assert.equal(f.calls.filter(c => c.kind === 'post').length, 2);
});

test('history timeout does not repeat an accepted message or release its branch barrier', async () => {
  const pending = deferred(), f = fixture({ reconcileTimeoutMs: 10, reconciliation: () => pending.promise });
  assert.equal((await f.send().completion).status, 'accepted');
  await new Promise(resolve => setTimeout(resolve, 30));
  assert.equal(f.api.state().code, 'reconciliation_timeout');
  assert.equal(f.api.state().pending, true);
  pending.resolve(false); await turn();
  assert.equal(f.calls.filter(c => c.kind === 'post').length, 1);
});

test('duplicate command during preparation is single-flight', async () => {
  const pending = deferred(), f = fixture({ prepare: () => pending.promise });
  const one = f.send(); assert.equal(f.send(), one);
  await turn(); assert.equal(f.calls.length, 1);
  assert.equal((await f.send({ requestId: 'mcp_second' }).completion).code, 'busy');
  pending.resolve({ conduit_token: 'fixture' });
  assert.equal((await one.completion).status, 'accepted'); await turn();
  assert.equal(f.calls.filter(x => x.kind === 'post').length, 1);
});

test('owner changes while preparing cannot dispatch or fall back', async () => {
  const pending = deferred(), f = fixture({ prepare: () => pending.promise });
  const result = f.send(); await turn(); f.setCurrent(false); pending.resolve({ conduit_token: 'fixture' });
  assert.equal((await result.completion).code, 'context_changed'); await turn();
  assert.equal(f.calls.some(x => x.kind === 'post'), false); assert.equal(result.claimFallback(), false);
});

test('draft changes while fresh security is pending cannot send to stale draft', async () => {
  const pending = deferred(), f = fixture({ security: () => pending.promise });
  const result = f.send(); await turn(); f.draft('new draft'); pending.resolve(security);
  assert.equal((await result.completion).status, 'rejected');
  assert.equal(f.calls.some(x => x.kind === 'post'), false); assert.equal(f.draft(), 'new draft');
});

test('preparation timeout aborts and a late response never sends', async () => {
  const pending = deferred(), f = fixture({ prepare: () => pending.promise, prepareTimeoutMs: 15 });
  const result = f.send(); assert.equal((await result.completion).code, 'preparation_timeout'); await turn();
  pending.resolve({ conduit_token: 'fixture' }); await turn();
  assert.equal(f.calls.filter(x => x.kind === 'prepare').length, 1);
  assert.equal(f.calls[0].value.signal.aborted, true);
  assert.equal(f.calls.some(x => x.kind === 'post'), false); assert.equal(result.claimFallback(), false);
});

test('security force_login does not issue a conversation POST', async () => {
  const f = fixture({ security: () => ({ chatReq: { token: 'fixture', force_login: true } }) });
  const result = f.send(); assert.equal((await result.completion).code, 'login_required');
  assert.equal(result.claimFallback(), false); assert.equal(f.calls.some(x => x.kind === 'post'), false);
});

test('synchronous failure after dispatch hook is unknown and retains the write barrier', async () => {
  const f = fixture({ stream: value => (async function* () {
    value.onBeforeRequestStart(); throw Error('fixture fetch failed');
  })() });
  const result = f.send(); assert.equal((await result.completion).status, 'unknown'); await turn();
  assert.equal(f.api.state().pending, true); assert.equal(f.api.state().phase, 'uncertain');
  f.page.__elonChatGptFreshTextDispatchEnabled = false;
  assert.equal((await f.send({ requestId: 'mcp_next' }).completion).code, 'busy');
  assert.equal(result.claimFallback(), false);
});

test('headers are acceptance only; an interrupted stream never clears ownership', async () => {
  const f = fixture({ stream: value => (async function* () {
    value.onBeforeRequestStart();
    yield { response: new Response(null, { headers: { 'content-type': 'text/event-stream' } }) };
    throw Error('fixture missing terminal event');
  })() });
  assert.equal((await f.send().completion).status, 'accepted'); await turn();
  assert.equal(f.api.state().phase, 'uncertain'); assert.equal(f.api.state().pending, true);
});

test('cancel before POST cannot start on late preparation', async () => {
  const pending = deferred(), f = fixture({ prepare: () => pending.promise });
  const result = f.send(); await turn(); assert.equal(f.api.cancel(), true);
  assert.equal((await result.completion).code, 'cancelled'); await turn();
  pending.resolve({ conduit_token: 'fixture' }); await turn();
  assert.equal(f.api.state().pending, false); assert.equal(f.calls.some(x => x.kind === 'post'), false);
});

test('cancel an open request retains unknown server status; no automatic stop success', async () => {
  const wait = deferred(), f = fixture({ stream: value => (async function* () {
    value.onBeforeRequestStart();
    yield { response: new Response(null, { headers: { 'content-type': 'text/event-stream' } }) };
    await wait.promise;
  })() });
  assert.equal((await f.send().completion).status, 'accepted'); assert.equal(f.api.cancel(), true); await turn();
  assert.equal(f.api.state().pending, true); assert.equal(f.api.state().phase, 'uncertain');
  wait.resolve(); await turn(); assert.equal(f.api.state().pending, true);
});

test('fresh document retires local reader and prevents late work from reaching HTTP', async () => {
  const pending = deferred(), f = fixture({ prepare: () => pending.promise });
  const result = f.send(); await turn();
  f.page.document = {}; f.page.__elonChatGptDocumentToken = 'doc_replaced';
  assert.equal(f.api.state().pending, false);
  await result.completion; pending.resolve({ conduit_token: 'fixture' }); await turn();
  assert.equal(f.calls.some(x => x.kind === 'post'), false);
});

test('new native text entered after accepted HTTP is never cleared', async () => {
  const response = deferred(), f = fixture({ stream: value => (async function* () {
    value.onBeforeRequestStart(); await response.promise;
    yield { response: new Response(null, { headers: { 'content-type': 'text/event-stream' } }) };
  })() });
  f.draft(command.prompt);
  const result = f.send({ expectedDraft: command.prompt }); await turn(); f.draft('next question'); response.resolve();
  assert.equal((await result.completion).status, 'accepted'); await turn(); assert.equal(f.draft(), 'next question');
});

test('response-header deadline is separate from the longer accepted stream deadline', async () => {
  const pending = deferred();
  const f = fixture({ openTimeoutMs: 15, streamTimeoutMs: 1000, stream: value => (async function* () {
    value.onBeforeRequestStart(); await pending.promise;
    yield { response: new Response(null, { headers: { 'content-type': 'text/event-stream' } }) };
  })() });
  const result = f.send(); assert.equal((await result.completion).code, 'stream_open_timeout');
  pending.resolve(); await turn();
  assert.equal(f.api.state().accepted, false); assert.equal(f.api.state().pending, true);
});

test('pre-dispatch fallback is once-only and cannot cross an account change', async () => {
  const f = fixture({ capture: async () => { throw Error('runtime_unavailable'); } });
  const result = f.send(); assert.equal((await result.completion).status, 'unavailable'); await turn();
  f.setCurrent(false); assert.equal(result.claimFallback(), false);
  f.setCurrent(true); assert.equal(result.claimFallback(), true); assert.equal(result.claimFallback(), false);
  assert.equal(f.calls.length, 0);
});

test('unsupported plain-text context preserves the accepted sender, but another writer does not', async () => {
  for (const code of ['attachments_active', 'tools_active', 'parent_unavailable', 'identity_unavailable', 'conversation_busy']) {
    const f = fixture({ capture: async () => { throw Error(code); } });
    const result = f.send(), allowed = code !== 'conversation_busy';
    assert.equal((await result.completion).status, allowed ? 'unavailable' : 'rejected'); await turn();
    assert.equal(result.claimFallback(), allowed);
    assert.equal(f.calls.length, 0);
  }
});

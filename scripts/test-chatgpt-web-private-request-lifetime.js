'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const assets = path.join(__dirname, '../android/app/src/main/assets');
const transport = require(path.join(assets, 'chatgpt_web_private_json_request.js'));
const authModule = require(path.join(assets, 'chatgpt_web_private_auth_context.js'));
const mutationModule = require(path.join(assets, 'chatgpt_web_private_conversation_mutation.js'));
const flush = () => new Promise(resolve => setImmediate(resolve));
const json = value => new Response(JSON.stringify(value), { headers: { 'content-type': 'application/json' } });
function deferred() {
  let resolve;
  const promise = new Promise(done => { resolve = done; });
  return { promise, resolve };
}
function runtime(fetch) {
  const timers = new Map();
  let id = 0;
  const root = {
    fetch, AbortController,
    location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/', pathname: '/' },
    __elonChatGptPrivateJsonRequest: transport,
    setTimeout: (fn, delay) => { timers.set(++id, { fn, delay }); return id; },
    clearTimeout: id => timers.delete(id),
  };
  return { root, timers, expire: () => {
    assert.equal(timers.size, 1);
    timers.values().next().value.fn();
  } };
}
function hangingBody() {
  let cancelled = 0;
  const response = new Response(new ReadableStream({ cancel() { cancelled++; } }));
  return { response, cancelled: () => cancelled };
}

test('JSON is streamed and multibyte characters survive split chunks', async () => {
  const bytes = new TextEncoder().encode(JSON.stringify({ text: '\u4f60\u597d' }));
  const response = new Response(new ReadableStream({ start(controller) {
    for (const byte of bytes) controller.enqueue(new Uint8Array([byte]));
    controller.close();
  } }));
  const value = runtime(async () => response);
  const result = await transport.request(value.root, '/fixture', {}, { maxBytes: bytes.length });
  assert.deepEqual(result.payload, { text: '\u4f60\u597d' });
  assert.equal(value.timers.size, 0);
});

test('body deadline rejects and releases the stream after successful headers', async () => {
  const body = hangingBody();
  let signal;
  const value = runtime(async (_, init) => { signal = init.signal; return body.response; });
  const pending = transport.request(value.root, '/fixture', {}, { timeoutMs: 25 });
  const rejected = assert.rejects(pending, /^Error: timeout$/);
  await flush();
  assert.equal(value.timers.values().next().value.delay, 25);
  value.expire();
  await rejected;
  await flush();
  assert.equal(signal.aborted, true);
  assert.equal(body.cancelled(), 1);
  assert.equal(value.timers.size, 0);
});

test('ignored fetch abort still settles and late responses are discarded', async () => {
  const delayed = deferred();
  const value = runtime(() => delayed.promise);
  const pending = transport.request(value.root, '/fixture', {}, {});
  const rejected = assert.rejects(pending, /timeout/);
  value.expire();
  await rejected;
  const body = hangingBody();
  delayed.resolve(body.response);
  await flush();
  assert.equal(body.cancelled(), 1);
  assert.equal(value.timers.size, 0);
});

test('deadline also works without AbortController or streaming body support', async () => {
  const delayed = deferred();
  const value = runtime(async () => ({ ok: true, status: 200, text: () => delayed.promise }));
  value.root.AbortController = undefined;
  const pending = transport.request(value.root, '/fixture', {}, {});
  const rejected = assert.rejects(pending, /timeout/);
  await flush(); value.expire(); await rejected;
  delayed.resolve('{"late":true}'); await flush();
  assert.equal(value.timers.size, 0);
});

test('declared and actual oversized bodies are rejected and cancelled', async () => {
  for (const declared of [undefined, '1', '9999']) {
    let cancelled = 0;
    const response = new Response(new ReadableStream({ start(controller) {
      controller.enqueue(new Uint8Array(32));
    }, cancel() { cancelled++; } }), { headers: declared ? { 'content-length': declared } : {} });
    const value = runtime(async () => response);
    await assert.rejects(transport.request(value.root, '/fixture', {}, { maxBytes: 16 }), /response_too_large/);
    assert.equal(cancelled, 1);
    assert.equal(value.timers.size, 0);
  }
});

test('fallback body limit counts UTF-8 bytes rather than characters', async () => {
  const value = runtime(async () => ({ ok: true, status: 200, text: async () => '"\u4f60"' }));
  await assert.rejects(transport.request(value.root, '/fixture', {}, { maxBytes: 4 }), /response_too_large/);
});

test('malformed JSON and malformed UTF-8 do not produce successful empty payloads', async () => {
  for (const body of ['{broken', '', new Uint8Array([0xff])]) {
    const value = runtime(async () => new Response(body));
    await assert.rejects(transport.request(value.root, '/fixture', {}, {}));
    assert.equal(value.timers.size, 0);
  }
});

test('HTTP errors are preserved without consuming an error response', async () => {
  let cancelled = 0;
  const value = runtime(async () => new Response(new ReadableStream({ cancel() { cancelled++; } }), { status: 403 }));
  await assert.rejects(transport.request(value.root, '/fixture', {}, {}), /http_403/);
  assert.equal(cancelled, 1);
});

test('write acknowledgement releases an unused body without waiting for it', async () => {
  const body = hangingBody();
  let requests = 0;
  const value = runtime(async () => { requests++; return body.response; });
  assert.deepEqual(await transport.request(value.root, '/fixture', { method: 'PATCH' }, { mode: 'none' }),
    { ok: true, status: 200 });
  assert.equal(requests, 1);
  assert.equal(body.cancelled(), 1);
  assert.equal(value.timers.size, 0);
});

test('caller cancellation prevents dispatch or cancels an ongoing body', async () => {
  for (const early of [true, false]) {
    const body = hangingBody();
    let requests = 0;
    const value = runtime(async () => { requests++; return body.response; });
    const controller = new AbortController();
    if (early) controller.abort();
    const pending = transport.request(value.root, '/fixture', { signal: controller.signal }, {});
    const rejected = assert.rejects(pending, /cancelled/);
    if (!early) { await flush(); controller.abort(); }
    await rejected;
    assert.equal(requests, early ? 0 : 1);
    assert.equal(value.timers.size, 0);
  }
});

test('auth body timeout releases single-flight and late credentials are ignored', async () => {
  const late = deferred();
  let requests = 0;
  const value = runtime(async () => ++requests === 1
    ? { ok: true, status: 200, text: () => late.promise }
    : json({ accessToken: 'synthetic-valid-token' }));
  value.root.__elonChatGptPrivateAuthContextEnabled = true;
  const auth = authModule.create(value.root);
  const pending = auth.prewarm();
  assert.equal(pending, auth.prewarm());
  const rejected = assert.rejects(pending, /timeout/);
  await flush(); value.expire(); await rejected;
  assert.equal(auth.state().loading, false);
  assert.equal(auth.state().lastOutcome, 'timeout');
  late.resolve(JSON.stringify({ accessToken: 'synthetic-stale-token' }));
  await flush();
  assert.equal(auth.copyRequestHeaders(), null);
  auth.invalidate('explicit_retry');
  await auth.prewarm();
  assert.equal(auth.state().ready, true);
  assert.equal(requests, 2);
});

test('mutation reconciliation timeout releases busy state and never replays PATCH', async () => {
  const body = hangingBody();
  const methods = [];
  const accepted = [];
  const value = runtime(async (_, init) => {
    methods.push(init.method);
    return init.method === 'PATCH' ? json({}) : body.response;
  });
  const mutation = mutationModule.create(value.root, {
    enabled: true,
    privateTransport: { acquireSameOriginRequestHeaders: async () => ({ Authorization: 'Bearer synthetic-auth' }) },
    directory: { acceptTitleState: (id, title) => accepted.push({ id, title }) },
  });
  const pending = mutation.rename('/c/fixture-conversation', 'Fixture title');
  await flush();
  assert.deepEqual(methods, ['PATCH', 'GET']);
  assert.equal(mutation.state().state, 'busy');
  value.expire();
  const result = await pending;
  assert.equal(result.code, 'mutation_server_acknowledged');
  assert.equal(result.reconciled, false);
  assert.equal(mutation.state().state, 'ready');
  assert.equal(accepted.length, 1);
  assert.deepEqual(methods, ['PATCH', 'GET']);
  assert.equal(body.cancelled(), 1);
});

test('timed-out project refresh cannot replace a later successful directory', async () => {
  const late = deferred();
  let calls = 0;
  const value = runtime(async () => ++calls === 1
    ? { ok: true, status: 200, text: () => late.promise }
    : json({ items: [{ id: 'new-record', title: 'New fixture' }] }));
  const context = { window: value.root, location: value.root.location, URL };
  vm.runInNewContext(fs.readFileSync(path.join(assets, 'chatgpt_web_private_conversation_directory.js'), 'utf8'), context);
  const directory = value.root.__elonChatGptPrivateConversationDirectory;
  const pending = directory.refreshProject('g-p-fixture');
  assert.equal(pending, directory.refreshProject('g-p-fixture'));
  await flush(); value.expire(); assert.equal(await pending, false);
  assert.equal(await directory.refreshProject('g-p-fixture'), true);
  const revision = directory.snapshot().revision;
  late.resolve(JSON.stringify({ items: [{ id: 'stale-record', title: 'Stale fixture' }] }));
  await flush();
  assert.equal(directory.snapshot().revision, revision);
  assert.deepEqual(Array.from(directory.snapshot().conversations, x => x.id), ['new-record']);
  assert.equal(value.timers.size, 0);
});

test('Android early identity and desktop early directory load the request owner first', () => {
  const root = path.join(__dirname, '..');
  const android = fs.readFileSync(path.join(root,
    'android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt'), 'utf8');
  const desktop = fs.readFileSync(path.join(root,
    'desktop-shell/src-tauri/src/local_ai_browser/chatgpt_adapter_bootstrap.rs'), 'utf8');
  assert.match(android, /privateAuthContextScript = listOf\(\s*"chatgpt_web_private_json_request.js", PRIVATE_AUTH_CONTEXT_ASSET/);
  assert.ok(desktop.indexOf('  __PRIVATE_JSON_REQUEST__') < desktop.indexOf('  __PRIVATE_CONVERSATION_DIRECTORY__'));
  assert.ok(desktop.includes('.replace("__PRIVATE_JSON_REQUEST__", PRIVATE_JSON_REQUEST)'));
  assert.ok(desktop.includes('chatgpt_web_private_json_request.js'));
});

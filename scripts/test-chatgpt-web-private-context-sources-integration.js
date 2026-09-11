'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const { webcrypto } = require('node:crypto');
const base = '../android/app/src/main/assets/';
const sources = require(base + 'chatgpt_web_private_context_sources.js');
const projectionModule = require(base + 'chatgpt_web_private_history_projection.js');
const stream = require(base + 'chatgpt_web_private_stream_policy.js');
const jsonRequest = require(base + 'chatgpt_web_private_json_request.js');
const downloadModule = require(base + 'chatgpt_web_private_file_download.js');
const projection = projectionModule.create({});
const file = (id = 'cited', extra = {}) => ({ type: 'file', file_id: 'file-' + id,
  citation_uuid: id, name: id + '.txt', url: 'file://library/file-' + id, retrieval_origin: 'pca', ...extra });
const source = item => ({ type: 'conversation_context_source', item });
const sse = events => events.map(event => 'data: ' + JSON.stringify(event) + '\n\n').join('');
const message = (id = 'answer', status = 'seeded', inline = []) => ({ id, author: { role: 'assistant' },
  content: { parts: ['synthetic answer'] }, metadata: {
    conversation_context_citation_metadata_status: status,
    conversation_context_citation_metadata: inline.map(citation => ({ citation }))
  } });
const deferred = () => { let resolve; return { promise: new Promise(r => { resolve = r; }), resolve }; };

function fixture(handler = () => new Response(sse([source(file())]))) {
  const calls = [], queued = [], receipts = [];
  let account = 'Bearer synthetic-context-source';
  const payload = { messages: [message()] };
  const headers = () => ({ Authorization: account });
  const bridge = { onmessage: null, postMessage(raw) {
    const value = JSON.parse(raw);
    if (value.cancel) return;
    queued.push(value);
    queueMicrotask(() => bridge.onmessage?.({ data: JSON.stringify({ leaseId: value.leaseId, state: 'queued' }) }));
  } };
  const root = {
    location: { origin: 'https://chatgpt.com', pathname: '/c/visible', href: 'https://chatgpt.com/c/visible' },
    __elonChatGptDocumentToken: 'doc_context_1',
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: headers, acquireSameOriginRequestHeaders: async () => headers() },
    __elonChatGptPrivateHistoryProjection: projectionModule, __elonChatGptPrivateStreamPolicy: stream,
    __elonChatGptPrivateJsonRequest: jsonRequest,
    AbortController, crypto: webcrypto, setTimeout, clearTimeout, elonChatGptFileDownload: bridge,
    get document() { throw new Error('Native inventory must not inspect DOM'); },
    fetch: async (url, init) => {
      const parsed = new URL(url, root.location.href);
      calls.push({ url: parsed, init });
      if (parsed.pathname.endsWith('/simple')) return Response.json({ file_id: 'file-cited', is_library_file: false });
      if (parsed.pathname.includes('/files/download/')) return Response.json({ status: 'success', file_id: 'file-cited',
        download_url: 'https://files.oaiusercontent.com/synthetic-file' });
      if (parsed.pathname.includes('/conversations/')) return Response.json(payload);
      return handler(parsed, init);
    }
  };
  const api = sources.create(root), downloads = downloadModule.create(root);
  root.__elonChatGptPrivateContextSources = api;
  root.__elonChatGptPrivateFileDownload = downloads;
  const rows = () => downloads.register('/c/source', payload, projection.files(payload));
  const run = row => downloads.start(JSON.stringify({ version: 1,
    leaseId: '00000000-0000-4000-8000-000000000001', documentToken: root.__elonChatGptDocumentToken,
    href: root.location.href, path: '/c/source', name: row.name, downloadHandle: row.downloadHandle }),
  (...args) => receipts.push(args));
  return { root, payload, api, rows, run, calls, queued, receipts, headers,
    setAccount(value) { account = value; } };
}

test('actual source owner -> projection -> opaque native handle -> existing scoped download; no DOM', async () => {
  const f = fixture();
  assert.equal(projection.files(f.payload).truncated, true);
  assert.equal((await f.api.enrich(f.payload, 'source')).partial, false);
  const rows = f.rows();
  assert.equal(rows.length, 1);
  assert.match(rows[0].downloadHandle, /^download_[a-f0-9]{32}$/);
  assert.doesNotMatch(JSON.stringify(rows), /Bearer|file-cited|file:\/\/|retrieval_origin/);
  await f.run(rows[0]);
  assert.equal(f.calls.length, 3);
  const read = f.calls[0];
  assert.equal(read.url.pathname, '/backend-api/sidebar/conversation_context_sources');
  assert.equal(read.init.credentials, 'include');
  assert.equal(read.init.method, 'POST');
  assert.deepEqual(JSON.parse(read.init.body), { conversation_id: 'source', message_id: 'answer', expand_partial_inline: false });
  assert.equal(f.calls[2].url.searchParams.get('check_context_scopes_for_conversation_id'), 'source');
  assert.equal(f.calls[2].url.searchParams.get('download_intent'), 'true');
  assert.equal(f.queued.length, 1);
  assert.equal(f.receipts.at(-1)[2], 'download_queued');
  assert.equal(f.root.location.pathname, '/c/visible');
});

test('single flight and warm cache attach to fresh payload objects without another source POST', async () => {
  const gate = deferred();
  const f = fixture(async () => { await gate.promise; return new Response(sse([source(file())])); });
  const other = structuredClone(f.payload);
  const first = f.api.enrich(f.payload, 'source'), second = f.api.enrich(other, 'source');
  await new Promise(r => setImmediate(r));
  assert.equal(f.calls.length, 1);
  gate.resolve(); await Promise.all([first, second]);
  assert.equal(projection.files(other).files.length, 1);
  const fresh = structuredClone(f.payload);
  f.api.applyCached(fresh, 'source');
  assert.equal(projection.files(fresh).files.length, 1);
  await f.api.enrich(fresh, 'source');
  assert.equal(f.calls.length, 1);
});

test('same ID in another conversation and changed seeds have separate cache ownership', async () => {
  const f = fixture();
  await f.api.enrich(f.payload, 'source');
  await f.api.enrich(structuredClone(f.payload), 'another');
  f.payload.messages[0].metadata.conversation_context_citation_metadata.push({ citation: file('seed') });
  await f.api.enrich(f.payload, 'source');
  assert.equal(f.calls.length, 3);
});

test('outer deletion invalidates warm sources and revokes the prior native download handle', async () => {
  const f = fixture();
  await f.api.enrich(f.payload, 'source');
  const previous = f.rows()[0];
  const metadata = f.payload.messages[0].metadata;
  metadata.conversation_context_citation_metadata = [{ citation: file(), deleted: true }];
  metadata.conversation_context_citation_metadata_status = 'complete';
  f.api.applyCached(f.payload, 'source');
  assert.equal(projection.files(f.payload).files.length, 0);
  assert.equal(projection.files(f.payload).truncated, false);
  assert.equal(f.rows().length, 0);
  await f.api.enrich(f.payload, 'source');
  await f.run(previous);
  assert.equal(f.calls.length, 1);
  assert.equal(f.queued.length, 0);
  assert.notEqual(f.receipts.at(-1)?.[2], 'download_queued');
});

for (const change of ['account', 'document', 'navigation']) test('late ' + change + ' changes reject results and old approvals', async () => {
  const gate = deferred();
  const f = fixture(async () => { await gate.promise; return new Response(sse([source(file())])); });
  const pending = f.api.enrich(f.payload, 'source');
  await new Promise(r => setImmediate(r));
  if (change === 'account') f.setAccount('Bearer another-synthetic-account');
  if (change === 'document') f.root.__elonChatGptDocumentToken = 'doc_replaced';
  if (change === 'navigation') f.root.location.href = 'https://chatgpt.com/c/other';
  gate.resolve();
  assert.equal((await pending).stale, true);
  assert.equal(f.rows().length, 0);
  assert.equal(f.calls.length, 1);
});

test('inline complete is local; partial inline expands; pending and unknown remain partial without requests', async () => {
  const f = fixture();
  f.payload.messages = [message('ready', 'complete', [file()]), message('waiting', 'pending_inline_finalize'),
    message('unknown', 'future-status'), message('partial', 'complete_inline_only')];
  const result = await f.api.enrich(f.payload, 'source');
  assert.equal(result.partial, true);
  assert.equal(f.calls.length, 1);
  assert.equal(JSON.parse(f.calls[0].init.body).expand_partial_inline, true);
  assert.equal(projection.files(f.payload).files.length, 2);
});

test('only selected visible branch messages are fetched, not regeneration alternatives or analysis', async () => {
  const f = fixture();
  f.payload.messages = undefined;
  f.payload.current_node = 'chosen';
  f.payload.mapping = { root: { message: { ...message('internal'), channel: 'analysis' } },
    chosen: { parent: 'root', message: message('chosen') }, other: { parent: 'root', message: message('other') } };
  await f.api.enrich(f.payload, 'source');
  assert.deepEqual(f.calls.map(call => JSON.parse(call.init.body).message_id), ['chosen']);
});

test('eight-message batch, two concurrent readers, then cached progress on explicit refresh', async () => {
  let active = 0, peak = 0;
  const f = fixture(async () => { active++; peak = Math.max(peak, active);
    await new Promise(r => setTimeout(r, 5)); active--; return new Response(sse([source(file())])); });
  f.payload.messages = Array.from({ length: 10 }, (_, i) => message('m' + i));
  assert.equal((await f.api.enrich(f.payload, 'source')).partial, true);
  assert.equal(f.calls.length, 8); assert.equal(peak, 2);
  assert.equal((await f.api.enrich(f.payload, 'source')).partial, false);
  assert.equal(f.calls.length, 10);
});

for (const response of ['auth', 'malformed', 'oversized']) test(response + ' source failure retains ordinary attachments and partial status', async () => {
  const f = fixture(() => response === 'auth' ? new Response('', { status: 401 }) :
    new Response(response === 'malformed' ? sse([source(file())]) + 'data: {bad}\n\n' : 'x'.repeat(1048577)));
  f.payload.messages[0].metadata.attachments = [{ id: 'file-uploaded', name: 'existing.txt' }];
  const result = await f.api.enrich(f.payload, 'source');
  assert.equal(result.partial, true);
  assert.deepEqual(projection.files(f.payload).files.map(row => row.name), ['existing.txt']);
  assert.equal(projection.files(f.payload).truncated, true);
  assert.equal(f.calls.length, 1);
});

test('deadline includes an uncooperative response body, without a retry or fallback send', async () => {
  const f = fixture(() => ({ status: 200, ok: true, text: () => new Promise(() => {}) }));
  const started = Date.now();
  assert.equal((await f.api.enrich(f.payload, 'source')).partial, true);
  assert.ok(Date.now() - started < 4800);
  assert.equal(f.calls.length, 1);
});

function installTransport(f) {
  f.root.__elonChatGptPrivateConversationPrefetchEnabled = true;
  f.root.__elonChatGptPrivateAuthContext = { copyRequestHeaders: f.headers };
  delete f.root.__elonChatGptPrivateTransport;
  const context = { window: f.root, location: f.root.location, URL, AbortController, Date, setTimeout, clearTimeout };
  for (const name of ['chatgpt_web_private_transport_policy.js', 'chatgpt_web_private_transport.js']) {
    vm.runInNewContext(fs.readFileSync(require.resolve(base + name), 'utf8'), context, { filename: name });
  }
  return f.root.__elonChatGptPrivateTransport;
}

test('production list command emits base then enriched snapshots and settles exactly once', async () => {
  const gate = deferred();
  const f = fixture(async () => { await gate.promise; return new Response(sse([source(file())])); });
  f.payload.messages[0].metadata.attachments = [{ id: 'file-uploaded', name: 'existing.txt' }];
  const transport = installTransport(f), events = [], replies = [];
  const task = transport.listConversationFiles('/c/source', 'mcp_sources', e => events.push(e), (...args) => replies.push(args));
  await new Promise(r => setTimeout(r, 20));
  assert.equal(events.length, 1); assert.equal(replies.length, 0);
  assert.equal(events[0].files[0].name, 'existing.txt');
  gate.resolve(); await task;
  assert.equal(events.length, 2); assert.equal(replies.length, 1);
  assert.equal(events[1].files.length, 2);
  assert.equal(events[1].truncated, false);
  assert.equal(replies[0][1], true);
  assert.equal(f.calls.length, 2);
});

test('account switch during history fetch cannot rebind old content to a new source request', async () => {
  const f = fixture(), gate = deferred(), original = f.root.fetch;
  f.root.fetch = async (...args) => { const result = await original(...args);
    if (String(args[0]).includes('/conversations/')) await gate.promise;
    return result; };
  const transport = installTransport(f), events = [], replies = [];
  const task = transport.listConversationFiles('/c/source', 'mcp_stale', e => events.push(e), (...args) => replies.push(args));
  await new Promise(r => setTimeout(r, 20));
  f.setAccount('Bearer new-synthetic-account'); gate.resolve(); await task;
  assert.equal(events.length, 0); assert.equal(f.calls.length, 1);
  assert.equal(replies[0][1], false);
});

test('ordinary inventory keeps one GET and one snapshot, no source request', async () => {
  const f = fixture(); f.payload.messages[0].metadata = { attachments: [{ id: 'file-uploaded', name: 'existing.txt' }] };
  const transport = installTransport(f), events = [], replies = [];
  await transport.listConversationFiles('/c/source', 'mcp_plain', e => events.push(e), (...args) => replies.push(args));
  assert.equal(events.length, 1); assert.equal(replies[0][1], true); assert.equal(f.calls.length, 1);
});

test('native polling waits for command completion after intermediate snapshots', () => {
  const code = fs.readFileSync(require.resolve('../android/app/src/main/kotlin/com/elon/app/WebChatConversationFilesCoordinator.kt'), 'utf8');
  assert.match(code, /if \(matched\) index = result/);
  assert.match(code, /status != WebChatConsumerCommandStatus\.SUCCEEDED \|\| !matched/);
  assert.match(code, /if \(matched\) sheet\?\.updateItems\(WebChatConversationFilesPresentation\.rows\(index, true, false\)\)/);
  assert.ok(code.indexOf('pollTask = null', code.indexOf('val matched')) > code.indexOf('val timedOut'));
});

test('upgrading then reinjecting assets replaces old captures and retains valid resolved downloads', async () => {
  const f = fixture();
  const oldPolicy = { version: 1 }, oldOwner = { version: 1 };
  let disposed = 0;
  f.root.__elonChatGptPrivateContextSourcesPolicy = oldPolicy;
  f.root.__elonChatGptPrivateContextSources = oldOwner;
  f.root.__elonChatGptPrivateFileDownload = { version: 28, dispose() { disposed++; } };
  const context = { window: f.root, URL, Date, setTimeout, clearTimeout };
  const assets = ['chatgpt_web_private_context_sources_policy.js', 'chatgpt_web_private_file_citation.js',
    'chatgpt_web_private_history_projection.js', 'chatgpt_web_private_context_sources.js',
    'chatgpt_web_private_image_pointer.js', 'chatgpt_web_private_file_download.js'];
  const inject = () => { for (const name of assets) vm.runInNewContext(
    fs.readFileSync(require.resolve(base + name), 'utf8'), context, { filename: name }); };
  inject();
  const api = f.root.__elonChatGptPrivateContextSources, policy = f.root.__elonChatGptPrivateContextSourcesPolicy;
  assert.notEqual(api, oldOwner);
  assert.notEqual(policy, oldPolicy);
  assert.equal(api.version, 2);
  assert.equal(policy.version, 2);
  assert.equal(disposed, 1);
  await api.enrich(f.payload, 'source');
  inject();
  assert.equal(f.root.__elonChatGptPrivateContextSources, api);
  assert.equal(f.root.__elonChatGptPrivateContextSourcesPolicy, policy);
  assert.equal(disposed, 1);
  const projected = f.root.__elonChatGptPrivateHistoryProjection.create({}).files(f.payload);
  const rows = f.root.__elonChatGptPrivateFileDownload.register('/c/source', f.payload, projected);
  assert.equal(rows.length, 1);
  assert.match(rows[0].downloadHandle, /^download_[a-f0-9]{32}$/);
  assert.equal(f.calls.length, 1);
  assert.equal(f.root.__elonChatGptPrivateFileDownload.version, 29);
  const metadata = f.payload.messages[0].metadata;
  metadata.conversation_context_citation_metadata = [{ citation: file(), deleted: true }];
  metadata.conversation_context_citation_metadata_status = 'complete';
  api.applyCached(f.payload, 'source');
  const updated = f.root.__elonChatGptPrivateHistoryProjection.create({}).files(f.payload);
  assert.equal(updated.files.length, 0);
  assert.equal(updated.truncated, false);
  assert.equal(f.root.__elonChatGptPrivateFileDownload.register('/c/source', f.payload, updated).length, 0);
});

'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const assets = path.join(__dirname, '../android/app/src/main/assets');
const bytes = require(path.join(assets, 'chatgpt_web_private_attachment_bytes.js'));
const protocol = require(path.join(assets, 'chatgpt_web_private_attachment_protocol.js'));
const transport = require(path.join(assets, 'chatgpt_web_private_attachment_transport.js'));
const json = require(path.join(assets, 'chatgpt_web_private_json_request.js'));
const blobUrl = 'https://uploads.oaiusercontent.com/synthetic?sig=fixture-only&other=value';
const input = () => new File(['abcdefghijk'], 'fixture.txt', { type: 'text/plain' });
const context = () => ({ useCase: 'ace_upload', storeInLibrary: false,
  libraryPersistenceMode: 'required', indexForRetrieval: false });
const prepared = (strategy) => ({ status: 'success', file_id: 'file-synthetic', upload_url: blobUrl,
  ...(strategy === undefined ? {} : { direct_library_upload_strategy: strategy }) });
const multipart = () => ({ kind: 'direct_azure_multipart', part_size_bytes: 4, part_count: 3, max_part_concurrency: 9 });
const flush = async () => { for (let i = 0; i < 30; i++) await Promise.resolve(); };

function fixture(options = {}) {
  const calls = [], stages = [], binding = {};
  let clock = 0, valid = true;
  const root = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/synthetic' },
    AbortController, FormData, btoa, setTimeout, clearTimeout, performance: { now: () => clock },
    __elonChatGptPrivateAttachmentBytes: options.bytes === false ? undefined : bytes,
    fetch: async (url, init) => {
      const call = { url, init }; calls.push(call);
      if (options.fetch) { const response = await options.fetch(call, calls.length); if (response) return response; }
      if (url === '/backend-api/files') return Response.json(options.prepared || prepared());
      if (url === '/backend-api/files/process_upload_stream') return new Response(JSON.stringify({
        file_id: 'file-synthetic', event: 'file.processing.completed', progress: 100,
        extra: { mime_type: options.mimeType || 'text/plain' },
      }) + '\n');
      return new Response(null, { status: 201 });
    } };
  const api = transport.create(root, { protocol, request: json.request, isCurrent: value => value === binding && valid,
    acquireHeaders: async () => ({ Authorization: 'Bearer page-local-synthetic', Cookie: 'excluded',
      'ChatGPT-Account-ID': 'synthetic-account', 'openai-sentinel-proof-token': 'excluded',
      'oai-device-id': 'synthetic-device' }), onProgress: value => stages.push(value.stage) });
  return { root, api, calls, stages, binding, advance(ms) { clock += ms; }, invalidate() { valid = false; },
    upload: (file = input(), scope = context()) => api.upload(file, scope, binding) };
}

test('byte module advertises multipart only when integrated; legacy whole-file transfer is retained', async () => {
  const f = fixture(), file = input();
  assert.equal((await f.upload(file)).ok, true);
  assert.equal(JSON.parse(f.calls[0].init.body).supports_direct_azure_multipart, true);
  assert.equal(f.calls[1].init.body, file);
  assert.equal(f.calls[1].init.headers['x-ms-blob-type'], 'BlockBlob');
  assert.equal(f.calls[1].init.credentials, 'omit');
  assert.deepEqual(f.calls.map(c => c.init.method), ['POST', 'PUT', 'POST']);
});

test('multipart uploads exact nonoverlapping bytes, commits ordered block ids, then processes once', async () => {
  const f = fixture({ prepared: prepared(multipart()) });
  const result = await f.upload();
  assert.equal(result.ok, true);
  assert.equal(result.associated, false);
  assert.equal(f.calls.length, 6);
  const parts = f.calls.slice(1, 4), commit = f.calls[4];
  assert.deepEqual(await Promise.all(parts.map(c => c.init.body.text())), ['abcd', 'efgh', 'ijk']);
  const ids = parts.map(c => new URL(c.url).searchParams.get('blockid'));
  assert.deepEqual(ids.map(atob), ['00000000', '00000001', '00000002']);
  for (const part of parts) {
    assert.equal(new URL(part.url).searchParams.get('comp'), 'block');
    assert.equal(new URL(part.url).searchParams.get('sig'), 'fixture-only');
    assert.equal(part.init.method, 'PUT');
    assert.equal(part.init.credentials, 'omit');
    assert.deepEqual(part.init.headers, { 'Content-Type': 'text/plain', 'x-ms-version': '2020-04-08' });
  }
  assert.equal(new URL(commit.url).searchParams.get('comp'), 'blocklist');
  assert.equal(new URL(commit.url).searchParams.has('blockid'), false);
  assert.equal(commit.init.body, '<?xml version="1.0" encoding="utf-8"?>\n<BlockList>\n' +
    ids.map(id => '  <Latest>' + id + '</Latest>').join('\n') + '\n</BlockList>');
  assert.deepEqual(commit.init.headers, { 'Content-Type': 'application/xml', 'x-ms-version': '2020-04-08',
    'x-ms-blob-content-type': 'text/plain' });
  assert.equal(commit.init.credentials, 'omit');
  assert.equal(f.calls[5].url, '/backend-api/files/process_upload_stream');
  assert.deepEqual(f.stages, ['preparing', 'uploading', 'processing', 'processed']);
  for (const c of f.calls) {
    assert.equal(c.init.redirect, 'error');
    assert.equal(c.init.headers.Cookie, undefined);
    assert.equal(c.init.headers['openai-sentinel-proof-token'], undefined);
    if (c.url.startsWith('https://uploads.')) {
      assert.equal(c.init.headers.authorization, undefined);
      assert.equal(c.init.headers['chatgpt-account-id'], undefined);
    }
  }
});

test('bounded workers do not exceed server concurrency or two local uploads and wait before commit', async () => {
  for (const serverConcurrency of [1, 20]) {
    const pending = [], active = new Set(); let peak = 0;
    const f = fixture({ prepared: prepared({ ...multipart(), max_part_concurrency: serverConcurrency }),
      fetch: c => {
        if (new URL(c.url, 'https://chatgpt.com').searchParams.get('comp') !== 'block') return;
        active.add(c); peak = Math.max(peak, active.size);
        return new Promise(resolve => pending.push(() => { active.delete(c); resolve(new Response(null, { status: 201 })); }));
      } });
    const result = f.upload(); await flush();
    assert.equal(active.size, Math.min(2, serverConcurrency));
    while (pending.length) { pending.shift()(); await flush(); }
    assert.equal((await result).ok, true);
    assert.equal(peak, Math.min(2, serverConcurrency));
    assert.equal(active.size, 0);
  }
});

for (const [name, patch] of Object.entries({
  wrong_kind: { kind: 'other' }, zero_size: { part_size_bytes: 0 },
  bad_count: { part_count: 2 }, noninteger_count: { part_count: 3.1 },
  one_part: { part_count: 1, part_size_bytes: 11 }, too_many: { part_size_bytes: 1, part_count: 129 },
  zero_concurrency: { max_part_concurrency: 0 }, infinite_concurrency: { max_part_concurrency: Infinity },
  unknown_strategy_field: { invented: true },
})) test(name + ' is rejected without uploading or processing the created entry', async () => {
  const f = fixture({ prepared: prepared({ ...multipart(), ...patch }) });
  const result = await f.upload();
  assert.equal(result.code, 'unsupported_upload_route');
  assert.equal(result.ok, false); assert.equal(result.mayHaveSideEffects, true);
  assert.equal(f.calls.length, 1);
});

test('AWS cannot accidentally take the Azure multipart route', () => {
  assert.throws(() => bytes.plan({ ...prepared(multipart()), upload_url: blobUrl + '&X-Amz-Algorithm=test' }, input(), protocol));
  const normal = bytes.plan({ ...prepared(), upload_url: blobUrl + '&X-Amz-Algorithm=test' }, input(), protocol);
  assert.equal(normal.kind, 'single');
  assert.deepEqual(normal.headers, { 'Content-Type': 'text/plain' });
});

test('a failed part cancels sibling requests, never commits, and never silently retries', async () => {
  let release;
  const f = fixture({ prepared: prepared(multipart()), fetch: c => {
    if (new URL(c.url, 'https://chatgpt.com').searchParams.get('comp') !== 'block') return;
    if (new URL(c.url).searchParams.get('blockid') === btoa('00000000')) return new Response(null, { status: 503 });
    return new Promise(resolve => { release = resolve; });
  } });
  const result = await f.upload();
  assert.equal(result.code, 'http_503'); assert.equal(result.mayHaveSideEffects, true);
  assert.equal(f.calls.length, 3); assert.equal(f.calls[2].init.signal.aborted, true);
  release(new Response(null, { status: 201 })); await flush();
  assert.equal(f.calls.length, 3);
  assert.equal((await f.upload()).code, 'cooldown');
});

for (const mode of ['cancel', 'context']) test(mode + ' while uploading cannot commit or attach stale bytes', async () => {
  const pending = [];
  const f = fixture({ prepared: prepared(multipart()), fetch: c => {
    if (new URL(c.url, 'https://chatgpt.com').searchParams.get('comp') === 'block') {
      return new Promise(resolve => pending.push(resolve));
    }
  } });
  const result = f.upload(); await flush();
  if (mode === 'cancel') f.api.cancel(); else f.invalidate();
  for (const resolve of pending) resolve(new Response(null, { status: 201 }));
  assert.equal((await result).code, mode === 'cancel' ? 'cancelled' : 'context_changed');
  assert.equal(f.calls.length, 3);
  assert.equal(f.api.snapshot().stage, 'idle');
});

test('multipart commit failure cannot advance to processing or whole-file fallback', async () => {
  const f = fixture({ prepared: prepared(multipart()), fetch: c => {
    if (new URL(c.url, 'https://chatgpt.com').searchParams.get('comp') === 'blocklist') return new Response(null, { status: 403 });
  } });
  assert.equal((await f.upload()).code, 'http_403');
  assert.equal(f.calls.length, 5);
  assert.equal(f.stages.includes('processing'), false);
});

test('one total byte-stage deadline prevents a fresh thirty seconds for every part', async () => {
  const f = fixture({ prepared: prepared(multipart()), fetch: c => {
    if (new URL(c.url, 'https://chatgpt.com').searchParams.get('comp') === 'block') f.advance(16000);
  } });
  const result = await f.upload();
  assert.equal(result.code, 'timeout');
  assert.equal(f.calls.length, 3);
  assert.equal(f.calls.some(c => c.url.includes('comp=blocklist')), false);
});

for (const prefix of ['/backend-api', '/api']) test('same-origin Estuary FormData ' + prefix, async () => {
  const url = prefix + '/estuary/upload_content_bytes?upload_url=' + encodeURIComponent(blobUrl);
  const f = fixture({ prepared: { ...prepared(), upload_url: url } });
  const file = input(), result = await f.upload(file);
  assert.equal(result.ok, true); assert.equal(result.associated, false);
  assert.equal(f.calls.length, 3);
  const sent = f.calls[1];
  assert.equal(sent.url, 'https://chatgpt.com' + url);
  assert.equal(sent.init.method, 'POST'); assert.equal(sent.init.credentials, 'include');
  assert.equal(sent.init.headers.authorization, 'Bearer page-local-synthetic');
  assert.equal(sent.init.headers['Content-Type'], undefined);
  assert.deepEqual([...sent.init.body.keys()], ['file', 'upload_url']);
  assert.equal(sent.init.body.get('file').name, 'fixture.txt');
  assert.equal(await sent.init.body.get('file').text(), await file.text());
  assert.equal(sent.init.body.get('upload_url'), blobUrl);
  assert.equal(f.calls[2].url, '/backend-api/files/process_upload_stream');
});

test('Estuary without a nested URL preserves the official destination form field', async () => {
  const url = '/backend-api/estuary/upload_content_bytes';
  const f = fixture({ prepared: { ...prepared(), upload_url: url } });
  assert.equal((await f.upload()).ok, true);
  assert.equal(f.calls[1].init.body.get('upload_url'), url);
});

for (const url of ['https://evil.test/backend-api/estuary/upload_content_bytes',
  'https://chatgpt.com.evil.test/backend-api/estuary/upload_content_bytes',
  'http://chatgpt.com/backend-api/estuary/upload_content_bytes',
  'https://name:pass@chatgpt.com/backend-api/estuary/upload_content_bytes',
  '/backend-api/estuary/unknown', '/backend-api/estuary/upload_content_bytes#secret',
  '/backend-api/estuary/upload_content_bytes?upload_url=https%3A%2F%2Flocalhost%2Ffile',
  '/backend-api/estuary/upload_content_bytes?upload_url=x&upload_url=y',
]) test('unsafe or unknown Estuary destination stays rejected: ' + url, async () => {
  const f = fixture({ prepared: { ...prepared(), upload_url: url } });
  assert.equal((await f.upload()).ok, false);
  assert.equal(f.calls.length, 1);
});

test('mixed strategies and custom forwarding headers are not inferred', () => {
  for (const extra of [{ direct_library_upload_strategy: multipart() }, { upload_headers: { Authorization: 'forbidden' } }]) {
    assert.throws(() => bytes.plan({ ...prepared(), upload_url: '/backend-api/estuary/upload_content_bytes', ...extra }, input(), protocol));
  }
});

test('Estuary failure does not claim processing or retry another route', async () => {
  const f = fixture({ prepared: { ...prepared(), upload_url: '/backend-api/estuary/upload_content_bytes' },
    fetch: c => { if (c.url.includes('/estuary/')) return new Response(null, { status: 429 }); } });
  const result = await f.upload();
  assert.equal(result.code, 'http_429'); assert.equal(result.mayHaveSideEffects, true);
  assert.equal(f.calls.length, 2);
});

test('both byte routes preserve existing image, PDF, document, temporary and project contracts', async () => {
  const projectId = 'g-p-' + 'a'.repeat(32);
  const origins = { origination_thread_id: '11111111-2222-3333-4444-555555555555',
    origination_message_id: '22222222-2222-3333-4444-555555555555' };
  const project = { gizmo_id: projectId, is_project: true, should_upload_to_project: true, ...origins };
  const cases = [
    { mimeType: 'text/plain', scope: context() },
    { mimeType: 'image/png', scope: { ...context(), useCase: 'multimodal', isTemporaryChat: true,
      libraryPersistenceMode: undefined, imageDimensions: { width: 16, height: 16 } } },
    { mimeType: 'application/pdf', scope: { ...context(), modelSlug: 'synthetic-model', isProjectThread: true,
      projectScopeId: projectId, libraryFileInfo: origins } },
    { mimeType: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
      scope: { ...context(), useCase: 'gizmo', isProjectThread: true, gizmoId: projectId,
        libraryFileInfo: project, indexForRetrieval: true } },
    { mimeType: 'image/png', scope: { ...context(), useCase: 'multimodal', isProjectThread: true,
      libraryFileInfo: project, indexForRetrieval: true, imageDimensions: { width: 16, height: 16 } } },
  ];
  for (const item of cases) {
    const file = new File(['abcdefghijk'], item.mimeType === 'application/pdf' ? 'fixture.pdf' : 'fixture', { type: item.mimeType });
    const legacy = fixture({ bytes: false, mimeType: item.mimeType });
    const { binding: originalBinding, ...expected } = await legacy.upload(file, item.scope);
    assert.equal(expected.ok, true);
    for (const response of [prepared(multipart()), { ...prepared(), upload_url: '/backend-api/estuary/upload_content_bytes' }]) {
      const f = fixture({ prepared: response, mimeType: item.mimeType });
      const { binding: newBinding, ...actual } = await f.upload(file, item.scope);
      assert.equal(newBinding, f.binding);
      assert.deepEqual(actual, expected);
      const originalCreation = JSON.parse(legacy.calls[0].init.body), creation = JSON.parse(f.calls[0].init.body);
      assert.deepEqual(creation, { ...originalCreation, supports_direct_azure_multipart: true });
      assert.deepEqual(JSON.parse(f.calls.at(-1).init.body), JSON.parse(legacy.calls.at(-1).init.body));
      assert.equal(f.calls.filter(c => c.url === '/backend-api/files/process_upload_stream').length, 1);
      for (const c of f.calls.slice(1, -1)) assert.equal(c.init.headers['x-oai-model-slug'], undefined);
    }
  }
});

test('production assets load byte routing before the attachment transport and parse together', () => {
  const adapter = require('./chatgpt-web-adapter-assembly').readAdapterSource();
  const names = [...adapter.split('private val ADAPTER_ASSETS = listOf(')[1].split(')')[0].matchAll(/"([a-z0-9_]+\.js)"/g)].map(m => m[1]);
  assert.ok(names.indexOf('chatgpt_web_private_attachment_bytes.js') >= 0);
  assert.ok(names.indexOf('chatgpt_web_private_attachment_bytes.js') < names.indexOf('chatgpt_web_private_attachment_transport.js'));
  new vm.Script(names.map(name => fs.readFileSync(path.join(assets, name), 'utf8')).join('\n'));
  const root = { location: { origin: 'https://chatgpt.com' } };
  vm.runInNewContext(fs.readFileSync(path.join(assets, 'chatgpt_web_private_attachment_bytes.js'), 'utf8'), { window: root });
  assert.equal(root.__elonChatGptPrivateAttachmentBytes.version, 1);
});

'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const { webcrypto } = require('node:crypto');
const projection = require('../android/app/src/main/assets/chatgpt_web_private_history_projection.js');
const download = require('../android/app/src/main/assets/chatgpt_web_private_file_download.js');
const request = require('../android/app/src/main/assets/chatgpt_web_private_json_request.js');
const PROJECT = 'g-p-' + 'a'.repeat(32);
const LIBRARY = 'libfile_synthetic';
const SOURCE = 'https://drive.example.test/fixture';
const SIGNED = 'https://files.oaiusercontent.com/fixture?sig=synthetic';

function fixture({ image = false, project = false, library = false, info } = {}) {
  let identity = 'Bearer synthetic-page-identity';
  const calls = [], queued = [], cancelled = [], receipts = [];
  const attachment = { id: 'file-synthetic', name: image ? 'fixture.png' : 'fixture.txt',
    mime_type: image ? 'image/png' : 'text/plain', source: 'connector',
    context_connector_info: info || { context_connector: 'google_drive', source_url: SOURCE, type: 'file', synthetic_extension: 'txt' },
    ...(library ? { library_file_id: LIBRARY } : {}) };
  const message = { id: 'message-synthetic', author: { role: 'user' },
    content: { parts: image ? [{ content_type: 'image_asset_pointer', asset_pointer: 'sediment://file-synthetic' }] : ['fixture'] },
    metadata: { attachments: [attachment] } };
  const payload = { ...(project ? { gizmo_id: PROJECT } : {}), messages: [message] };
  const bridge = { onmessage: null, postMessage(raw) {
    const value = JSON.parse(raw);
    if (value.cancel) { cancelled.push(value); return; }
    queued.push(value);
    queueMicrotask(() => bridge.onmessage?.({ data: JSON.stringify({ leaseId: value.leaseId, state: 'queued' }) }));
  } };
  const root = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/current' },
    __elonChatGptDocumentToken: 'doc_connector_download',
    __elonChatGptPrivateHistoryProjection: projection, __elonChatGptPrivateJsonRequest: request,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: identity }) },
    elonChatGptFileDownload: bridge, AbortController, crypto: webcrypto, setTimeout, clearTimeout,
    fetch: async (url, init) => {
      const parsed = new URL(url); calls.push({ url: parsed, init });
      assert.equal(parsed.origin, 'https://chatgpt.com', 'never fetch an external connector source');
      if (parsed.pathname.endsWith('/simple')) return Response.json({ is_library_file: true, library_file_id: LIBRARY,
        file_id: 'file-synthetic', is_project: project, ...(project ? { gizmo_id: PROJECT } : {}) });
      return Response.json({ status: 'success', file_id: 'file-synthetic', download_url: SIGNED });
    } };
  const api = download.create(root), privateProjection = projection.create({});
  const register = () => api.register('/c/selected', payload, privateProjection.files(payload));
  const descriptor = row => ({ version: 1, leaseId: '00000000-0000-4000-8000-000000000001',
    documentToken: root.__elonChatGptDocumentToken, href: root.location.href,
    path: '/c/selected', name: row.name, downloadHandle: row.downloadHandle });
  const run = row => api.start(JSON.stringify(descriptor(row)), (...args) => receipts.push(args));
  return { root, api, privateProjection, payload, attachment, message, calls, queued, cancelled, receipts, register, run,
    setIdentity: value => { identity = value; } };
}

for (const config of [{}, { image: true }, { project: true }, { library: true },
  { image: true, project: true, library: true }]) {
  test('connector copy retains conversation/project/library authorization: ' + JSON.stringify(config), async () => {
    const f = fixture(config), rows = f.register();
    assert.match(rows[0].downloadHandle || '', /^download_[a-f0-9]{32}$/);
    await f.run(rows[0]);
    assert.equal(f.calls.length, config.library ? 2 : 1);
    const { url, init } = f.calls.at(-1);
    assert.equal(url.pathname, '/backend-api/files/download/file-synthetic');
    assert.equal(url.searchParams.get(config.image || config.library || config.project
      ? 'check_context_scopes_for_conversation_id' : 'conversation_id'), 'selected');
    assert.equal(url.searchParams.get('gizmo_id'), config.project ? PROJECT : null);
    assert.equal(url.searchParams.get('download_intent'), 'true');
    assert.equal(init.method, 'GET'); assert.equal(init.credentials, 'same-origin');
    assert.equal(init.redirect, 'error');
    assert.equal(f.queued.length, 1);
    assert.deepEqual(Object.keys(f.queued[0]).sort(), ['documentToken', 'leaseId', 'url']);
    assert.equal(f.queued[0].url, SIGNED);
    assert.deepEqual(f.receipts, [['download_conversation_file', true, 'download_queued']]);
    const observable = JSON.stringify({ rows, messages: f.privateProjection.project(f.payload),
      requests: f.calls.map(call => call.url.href), queued: f.queued, receipts: f.receipts });
    assert.ok(!observable.includes(SOURCE));
    assert.ok(!observable.includes('context_connector_info'));
    assert.equal(f.root.location.href, 'https://chatgpt.com/c/current');
  });
}

test('recognized attribution does not need a source URL or change a saved library copy route', async () => {
  for (const source of [undefined, 'connector', 'library', 'local']) {
    const f = fixture({ info: { context_connector: 'synthetic_provider', source_url: null,
      synthetic_extension: null, type: null } });
    f.attachment.source = source;
    const rows = f.register();
    assert.ok(rows[0].downloadHandle);
    await f.run(rows[0]);
    assert.equal(f.calls.length, 1); assert.equal(f.queued.length, 1);
  }
});

for (const info of [false, '', [], {}, { source_url: SOURCE },
  { context_connector: false }, { context_connector: ' ' },
  { context_connector: 'google_drive', source_url: {} },
  { context_connector: 'google_drive', source_url: 'javascript:alert(1)' },
  { context_connector: 'google_drive', source_url: 'https://user:password@drive.example.test/file' },
  { context_connector: 'google_drive', source_url: 'https://drive.example.test/\nfile' },
  { context_connector: 'google_drive', type: {} },
  { context_connector: 'google_drive', synthetic_extension: ['txt'] },
  { context_connector: 'google_drive', gizmo_id: PROJECT },
  { context_connector: 'google_drive', connector_link_id: 'other' },
]) {
  test('unrecognized connector attribution is not treated as an ordinary downloaded copy: ' + JSON.stringify(info), () => {
    const f = fixture(); f.attachment.context_connector_info = info;
    assert.equal(f.register()[0].downloadHandle, undefined);
    assert.equal(f.calls.length, 0);
  });
}

for (const fields of [{ shared_library_file_id: LIBRARY }, { library_download_id: LIBRARY },
  { mounted_library_file_id: LIBRARY }, { shared_library_file_reference: { library_file_id: LIBRARY } },
  { preview_file: { id: 'file-another' } }, { context_scopes: ['other'] },
  { connector_id: 'external-id' }, { source_url: SOURCE }, { source: 'unrecognized' },
  { id: undefined }, { id: 'https://drive.example.test/file' }]) {
  test('different reference types remain visible but do not acquire a copy download: ' + JSON.stringify(fields), () => {
    const f = fixture(); Object.assign(f.attachment, fields);
    const rows = f.register();
    assert.equal(rows.length, 1);
    assert.equal(rows[0].name, 'fixture.txt');
    assert.equal(rows[0].downloadHandle, undefined);
  });
}

test('selection snapshots the copy rather than following later connector or file changes', async () => {
  const f = fixture(), rows = f.register();
  f.attachment.id = 'file-another'; f.attachment.context_connector_info.source_url = 'https://another.example.test/file';
  await f.run(rows[0]);
  assert.equal(f.calls[0].url.pathname, '/backend-api/files/download/file-synthetic');
  assert.equal(f.queued.length, 1);
});

test('copy authorization failure never falls back to the external source URL', async () => {
  const f = fixture(), rows = f.register();
  f.root.fetch = async (url, init) => { f.calls.push({ url: new URL(url), init }); return new Response('', { status: 404 }); };
  await f.run(rows[0]);
  assert.equal(f.calls.length, 1);
  assert.equal(f.queued.length, 0);
  assert.deepEqual(f.receipts, [['download_conversation_file', false, 'download_file_unavailable']]);
  assert.equal(f.cancelled.length, 1);
});

test('identity, document, route and cancellation checks span connector copy authorization', async () => {
  for (const mutate of [f => f.setIdentity('Bearer another-account'),
    f => { f.root.__elonChatGptDocumentToken = 'doc_another'; },
    f => { f.root.location.href += '-another'; }, f => f.api.cancel()]) {
    const f = fixture(), rows = f.register(), fetch = f.root.fetch;
    f.root.fetch = async (...args) => { const result = await fetch(...args); mutate(f); return result; };
    await f.run(rows[0]);
    assert.equal(f.calls.length, 1); assert.equal(f.queued.length, 0);
    assert.equal(f.receipts[0][1], false);
  }
});

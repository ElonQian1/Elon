'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const { webcrypto } = require('node:crypto');
const assets = '../android/app/src/main/assets/chatgpt_web_private_';
const download = require(assets + 'file_download.js');
const library = require(assets + 'library_download.js');
const json = require(assets + 'json_request.js');
const node = extra => ({ kind: 'file', id: 'libfile_generated', file_id: 'file-generated',
  library_artifact_type: 'image_gen', name: 'generated.png', mime_type: 'image/png', ...extra });
const PNG = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jRZkAAAAASUVORK5CYII=', 'base64');

function fixture(options = {}) {
  const calls = [], packets = [], chunks = [], receipts = [];
  let account = 'Bearer synthetic-generated-image', saved = false;
  const bridge = { onmessage: null, postMessage(raw) {
    const value = JSON.parse(raw); packets.push(value);
    if (value.cancel) return;
    if (value.byteOperation === 'chunk') chunks.push(Buffer.from(value.data, 'base64'));
    if (value.byteOperation === 'commit') saved = true;
    queueMicrotask(() => bridge.onmessage?.({ data: JSON.stringify({ leaseId: value.leaseId,
      byteOperation: value.byteOperation, sequence: value.sequence,
      state: { begin: 'ready', chunk: 'written', commit: 'saved' }[value.byteOperation] }) }));
  } };
  const root = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/unrelated' },
    __elonChatGptDocumentToken: 'doc_generated_image',
    __elonChatGptPrivateLibraryDownload: library, __elonChatGptPrivateJsonRequest: json,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: account }) },
    elonChatGptFileDownload: bridge, crypto: webcrypto, AbortController, setTimeout, clearTimeout, btoa,
    fetch: async (url, init) => {
      calls.push({ url, init });
      if (url === 'https://chatgpt.com/backend-api/files/file-generated/simple') {
        options.afterMetadata?.(f);
        return Response.json({ is_library_file: true, library_file_id: 'libfile_generated',
          file_id: 'file-generated', is_project: false, ...options.info });
      }
      assert.equal(url, 'https://chatgpt.com/api/library/files/libfile_generated/download');
      const response = new Response(PNG, { status: options.status || 200,
        headers: { 'Content-Type': options.mime || 'image/png', 'Content-Length': String(PNG.length) } });
      Object.defineProperty(response, 'url', { value: url });
      return response;
    } };
  Object.defineProperty(root, 'document', { get() { throw new Error('must_not_read_dom'); } });
  const api = download.create(root);
  const run = (file = node(), handle = api.registerLibraryFile(file)) => api.start(JSON.stringify({
    version: 1, byteTransferVersion: 1, leaseId: '00000000-0000-4000-8000-000000000001',
    documentToken: root.__elonChatGptDocumentToken, href: root.location.href,
    path: '/library', name: file.name, downloadHandle: handle,
  }), (...args) => receipts.push(args));
  const f = { root, api, calls, packets, chunks, receipts, run, get saved() { return saved; },
    changeIdentity() { account = 'Bearer synthetic-another-identity'; } };
  return f;
}

test('generated library image confirms ownership then saves original bytes without page navigation', async () => {
  const f = fixture(), file = node({ thumbnail_url: 'https://untrusted.test/thumbnail.png' });
  const handle = f.api.registerLibraryFile(file);
  assert.match(handle, /^download_[a-f0-9]{32}$/);
  Object.assign(file, { id: 'libfile_other', file_id: 'file-other' });
  await f.run(file, handle);
  assert.equal(f.saved, true);
  assert.deepEqual(Buffer.concat(f.chunks), PNG);
  assert.equal(f.calls.length, 2);
  assert.ok(f.calls.every(call => call.init.method === 'GET'));
  assert.equal(f.root.location.href, 'https://chatgpt.com/c/unrelated');
  assert.deepEqual(f.receipts, [['download_conversation_file', true, 'download_saved']]);
  assert.doesNotMatch(JSON.stringify(f.packets), /Bearer|https:|libfile|file-generated/);
  await f.run({ ...file, name: 'retargeted.png' }, handle);
  assert.equal(f.calls.length, 2, 'retargeted selection cannot trigger a request');
});

test('only bounded image_gen nodes qualify; unrelated artifacts and scopes remain unclaimed', () => {
  const f = fixture();
  for (const extra of [{ library_artifact_type: 'saved_entity' }, { library_artifact_type: 'unknown' },
    { library_artifact_type: 'deep_research_report' }, { library_artifact_type: 'flashcards' },
    { file_id: null }, { file_id: 'file-bad?scope=other' }, { mime_type: 'text/html' },
    { is_project: true }, { gizmo_id: 'g-p-' + 'a'.repeat(32) }, { project_id: 'other' },
    { context_scopes: [] }, { preview_file: {} }, { library_file_id: 'libfile_other' },
    { mounted_library_file_id: 'external-gdrive:file:synthetic' }, { saved_entity: {} },
    { trashed_at: 'now' }, { external_account: {} }, { cloud_doc_url: 'https://example.test' },
    { shared_library_file_id: 'libfile_other' }, { library_download_id: 'libfile_other' },
    { context_connector_info: {} }, { library_provider: 'google_drive' }]) {
    assert.equal(f.api.registerLibraryFile(node(extra)), '', JSON.stringify(extra));
  }
  assert.equal(f.calls.length, 0);
});

test('unconfirmed or changed image ownership never starts binary transfer', async () => {
  for (const info of [{ file_id: 'file-other' }, { library_file_id: 'libfile_other' },
    { is_library_file: false }, { is_project: true }, { is_project: 'false' },
    { gizmo_id: 'g-p-' + 'a'.repeat(32) }]) {
    const f = fixture({ info }); await f.run();
    assert.equal(f.saved, false); assert.equal(f.calls.length, 1);
    assert.equal(f.packets.some(p => p.byteOperation), false);
    assert.equal(f.receipts.at(-1)[1], false);
  }
});

test('identity changes, navigation and cancellation invalidate in-flight image downloads', async () => {
  for (const afterMetadata of [f => f.changeIdentity(), f => f.api.cancel(),
    f => { f.root.location.href += '-changed'; }, f => { f.root.__elonChatGptDocumentToken = 'doc_changed'; }]) {
    const f = fixture({ afterMetadata }); await f.run();
    assert.equal(f.saved, false); assert.equal(f.calls.length, 1);
    assert.equal(f.receipts.at(-1)[1], false);
  }
});

test('server denial or non-image error response does not replay through another path', async () => {
  for (const options of [{ status: 403 }, { mime: 'text/html' }]) {
    const f = fixture(options); await f.run();
    assert.equal(f.saved, false); assert.equal(f.calls.length, 2);
    assert.equal(f.receipts.at(-1)[1], false);
  }
});

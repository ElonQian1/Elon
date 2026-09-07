'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const { webcrypto } = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const projection = require('../android/app/src/main/assets/chatgpt_web_private_history_projection.js');
const download = require('../android/app/src/main/assets/chatgpt_web_private_file_download.js');
const library = require('../android/app/src/main/assets/chatgpt_web_private_library_download.js');
const request = require('../android/app/src/main/assets/chatgpt_web_private_json_request.js');
const PROJECT = 'g-p-' + 'a'.repeat(32);
const MOUNTED = 'external-gdrive:file:synthetic_file';
const MATERIALIZE = '/backend-api/files/library/mounted/materialize';
const CONTENT = '/backend-api/estuary/content?id=file-materialized&sig=synthetic';

function fixture(options = {}) {
  const calls = [], packets = [], receipts = [], stored = [];
  let identity = 'Bearer synthetic-mounted-identity', saved = false;
  const reference = { mounted_library_file_id: options.id || MOUNTED, name: 'fixture.txt' };
  const attachment = { ...reference, source: 'library', id: reference.mounted_library_file_id,
    mime_type: 'text/plain' };
  const metadata = options.attachment ? { attachments: [attachment] }
    : { mounted_library_file_references: [reference] };
  const payload = { ...(options.project ? { gizmo_id: PROJECT } : {}), messages: [{ id: 'message-fixture',
    author: { role: 'user' }, content: { parts: ['fixture'] }, metadata }] };
  const bridge = { postMessage(raw) {
    const p = JSON.parse(raw); packets.push(p);
    if (p.cancel) { if (!saved) stored.length = 0; return; }
    let state;
    if (p.byteOperation === 'begin') state = options.rejectMetadata ? 'failed' : 'ready';
    if (p.url) state = 'queued';
    if (p.byteOperation === 'chunk') { stored.push(Buffer.from(p.data, 'base64')); state = 'written'; }
    if (p.byteOperation === 'commit') { saved = true; state = 'saved'; }
    queueMicrotask(() => bridge.onmessage?.({ data: JSON.stringify({ leaseId: p.leaseId,
      byteOperation: p.byteOperation, sequence: p.sequence, state }) }));
  } };
  const root = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/current' },
    __elonChatGptDocumentToken: 'doc_mounted_download', crypto: webcrypto, AbortController, btoa,
    setTimeout: (fn, ms) => setTimeout(fn, options.fastTimeout ? Math.min(ms, 30) : ms), clearTimeout,
    elonChatGptFileDownload: bridge, __elonChatGptPrivateJsonRequest: request,
    __elonChatGptPrivateLibraryDownload: library, __elonChatGptPrivateHistoryProjection: projection,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: identity }) },
    fetch: async (url, init) => {
      calls.push({ url, init });
      const path = new URL(url).pathname;
      if (path === MATERIALIZE) {
        if (options.hang) return new Promise(() => {});
        options.onMaterialize?.(f);
        if (options.status) return new Response('', { status: options.status });
        return Response.json(options.response || { file_id: 'file-materialized', file_name: 'fixture.txt',
          mime_type: 'text/plain', file_size_bytes: 13 });
      }
      if (path === '/backend-api/files/download/file-materialized') {
        options.onAuthorization?.(f);
        return Response.json({ status: 'success', file_id: options.authorizationId || 'file-materialized',
          download_url: options.signed ? 'https://files.oaiusercontent.com/export?sig=synthetic' : CONTENT });
      }
      assert.equal(url, 'https://chatgpt.com' + CONTENT);
      const response = new Response('mounted bytes', { headers: { 'content-type': options.contentType || 'text/plain' } });
      Object.defineProperty(response, 'url', { value: url });
      return response;
    } };
  const api = download.create(root), history = projection.create({});
  const register = () => api.register('/c/selected', payload, history.files(payload));
  const run = row => api.start(JSON.stringify({ version: 1, byteTransferVersion: 1,
    resolvedFileVersion: options.resolvedFileVersion,
    leaseId: '00000000-0000-4000-8000-000000000001', documentToken: root.__elonChatGptDocumentToken,
    href: root.location.href, path: '/c/selected', name: row.name, downloadHandle: row.downloadHandle }),
  (...args) => receipts.push(args));
  const f = { root, api, reference, attachment, payload, metadata, history, register, run, calls, packets, receipts,
    stored, get saved() { return saved; }, setIdentity: value => { identity = value; } };
  return f;
}

test('mounted-only metadata appears in the existing native file index without private identifiers', () => {
  const f = fixture();
  assert.equal(f.history.files(f.payload).files[0].name, 'fixture.txt');
  assert.deepEqual(f.history.fileSource(f.payload, 'message-fixture:0').mountedLibraryReference, f.reference);
  assert.doesNotMatch(JSON.stringify(f.history.project(f.payload)), /external-gdrive|mounted_library|synthetic_file/);
  assert.doesNotMatch(JSON.stringify(f.history.files(f.payload)), /external-gdrive|mounted_library|synthetic_file/);
});

test('mounted references append after existing parts and deduplicate matching attachments', () => {
  const f = fixture();
  f.metadata.attachments = [{ name: 'ordinary.txt', id: 'file-ordinary' }, { ...f.attachment }];
  f.metadata.mounted_library_file_references.push({ ...f.reference },
    { mounted_library_file_id: 'external-box:file:12345', name: 'other.txt' });
  const rows = f.history.files(f.payload).files;
  assert.deepEqual(rows.map(row => row.name), ['ordinary.txt', 'fixture.txt', 'other.txt']);
  assert.equal(f.history.fileSource(f.payload, rows[1].id).attachment.id, MOUNTED);
  assert.equal(f.history.fileSource(f.payload, rows[2].id).mountedLibraryReference.name, 'other.txt');
});

test('mounted bounds, old-message indexing and hidden/alternative branch rules are preserved', () => {
  const f = fixture();
  f.metadata.mounted_library_file_references = Array.from({ length: 22 }, (_, i) =>
    ({ mounted_library_file_id: 'external-box:file:' + (i + 1), name: 'fixture-' + i + '.txt' }));
  f.payload.messages.push(...Array.from({ length: 81 }, (_, i) => ({ id: 'later-' + i,
    author: { role: 'assistant' }, content: { parts: ['later'] } })));
  assert.equal(f.history.project(f.payload).length, 80);
  assert.equal(f.history.files(f.payload).files.length, 20);
  assert.equal(f.history.files(f.payload).truncated, true);
  const source = f.payload.messages[0];
  assert.equal(f.history.files({ messages: [{ ...source, channel: 'analysis', author: { role: 'assistant' } }] }).files.length, 0);
  assert.equal(f.history.files({ mapping: { chosen: { message: f.payload.messages[1] },
    other: { message: source } }, current_node: 'chosen' }).files.length, 0);
});

for (const id of [MOUNTED, 'external-gdrive:account:synthetic_account:file:synthetic_file',
  'external-box:file:12345', 'external-dropbox:file:id:synthetic']) {
  test('confirmed mounted file materializes once then reuses native saved bytes: ' + id.split(':')[0], async () => {
    for (const project of [false, true]) {
      const f = fixture({ id, project });
      const row = f.register()[0];
      assert.match(row.downloadHandle, /^download_[a-f0-9]{32}$/);
      f.reference.mounted_library_file_id = 'external-box:file:999';
      await f.run(row);
      assert.deepEqual(f.receipts.at(-1).slice(1), [true, 'download_saved']);
      assert.equal(Buffer.concat(f.stored).toString(), 'mounted bytes');
      assert.equal(f.calls.length, 3);
      const first = f.calls[0];
      assert.equal(new URL(first.url).pathname, MATERIALIZE);
      assert.equal(first.init.method, 'POST');
      assert.equal(first.init.credentials, 'same-origin');
      assert.equal(first.init.redirect, 'error');
      assert.deepEqual(JSON.parse(first.init.body), { file_id: id, name: 'fixture.txt',
        mime_type: null, index_for_retrieval: false });
      const authorization = new URL(f.calls[1].url);
      assert.equal(authorization.searchParams.get(project ? 'check_context_scopes_for_conversation_id' : 'conversation_id'), 'selected');
      assert.equal(authorization.searchParams.get('gizmo_id'), project ? PROJECT : null);
      assert.equal(f.calls[2].init.headers, undefined);
      assert.doesNotMatch(JSON.stringify(f.packets), /external-|Bearer|mounted\/materialize/);
      await f.run(row);
      assert.equal(f.receipts.at(-1)[2], 'download_selection_expired');
      assert.equal(f.calls.length, 3, 'a consumed selection cannot materialize twice');
    }
  });
}

test('a library attachment with a mounted identity uses the same transaction and retains MIME', async () => {
  const f = fixture({ attachment: true });
  await f.run(f.register()[0]);
  assert.equal(f.receipts.at(-1)[2], 'download_saved');
  assert.equal(JSON.parse(f.calls[0].init.body).mime_type, 'text/plain');
});

test('unrecognized mounted descriptors never become ordinary downloads or arbitrary external requests', () => {
  for (const patch of [{ mounted_library_file_id: 'external-unknown:file:abc' },
    { mounted_library_file_id: 'external-box:file:0' }, { mounted_library_file_id: 'external-box:folder:123' },
    { mounted_library_file_id: 'external-dropbox:file:abc' }, { mounted_library_file_id: 'external-gdrive:file:a' },
    { mounted_library_file_id: MOUNTED + '?other=1' }, { mounted_library_file_id: ['external-box:file:123'] },
    { source_url: 'https://example.com/private' }, { id: 'file-other' }, { context_scopes: [] },
    { mime_type: 'text/plain' }, { name: 'bad\nname' }]) {
    const f = fixture(); Object.assign(f.reference, patch);
    assert.equal(f.register()[0]?.downloadHandle, undefined);
    assert.equal(f.calls.length, 0);
  }
  for (const patch of [{ id: 'file-copy' }, { library_file_id: 'libfile_other' },
    { library_provider: 'box' }, { mime_type: 'application/vnd.google-apps.folder' },
    { source: 'connector' }, { preview_file: { file_id: 'file-other' } }]) {
    const f = fixture({ attachment: true }); Object.assign(f.attachment, patch);
    assert.equal(f.register()[0]?.downloadHandle, undefined);
  }
  const box = fixture({ attachment: true, id: 'external-box:file:123' }); box.attachment.name = 'fixture.gdoc';
  assert.equal(box.register()[0]?.downloadHandle, undefined);
});

test('failed or ambiguous materialization is not replayed and never enters authorization', async () => {
  for (const options of [{ status: 403 }, { status: 500 }, { hang: true, fastTimeout: true },
    { response: {} }, { response: { file_id: MOUNTED, file_name: 'fixture.txt' } },
    { response: { file_id: 'file-ok', file_name: 'fixture.txt', file_size_bytes: 512 * 1024 * 1024 + 1 } },
    { response: { file_id: 'file-ok', file_name: 'fixture.txt', file_size_bytes: -1 } },
    { response: { file_id: 'file-ok', file_name: 'fixture.txt', mime_type: 'not mime' } }]) {
    const f = fixture(options), row = f.register()[0];
    await f.run(row);
    assert.equal(f.calls.length, 1);
    assert.equal(f.receipts.at(-1)[1], false);
    assert.equal(f.saved, false);
    await f.run(row);
    assert.equal(f.calls.length, 1);
    assert.equal(f.receipts.at(-1)[2], 'download_selection_expired');
  }
});

test('exported names and MIME cannot silently replace the pre-existing native save contract', async () => {
  for (const response of [{ file_id: 'file-ok', file_name: 'export.docx', mime_type: 'application/octet-stream' },
    { file_id: 'file-ok', file_name: 'fixture.txt', mime_type: 'application/pdf' }]) {
    const f = fixture({ attachment: true, response });
    await f.run(f.register()[0]);
    assert.equal(f.calls.length, 1);
    assert.equal(f.saved, false);
    assert.equal(f.receipts.at(-1)[2], 'download_source_unsupported');
  }
});

const EXPORTS = [
  ['document', 'docx', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'],
  ['spreadsheet', 'xlsx', 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'],
  ['presentation', 'pptx', 'application/vnd.openxmlformats-officedocument.presentationml.presentation'],
];
for (const [kind, extension, mime] of EXPORTS) {
  test('cloud ' + kind + ' exports through the same byte and signed native download owners', async () => {
    for (const signed of [false, true]) {
      const name = 'Exported fixture.' + extension;
      const f = fixture({ attachment: true, resolvedFileVersion: 1, signed, contentType: mime,
        response: { file_id: 'file-materialized', file_name: name, mime_type: mime } });
      f.attachment.mime_type = 'application/vnd.google-apps.' + kind;
      const row = f.register()[0];
      assert.match(row.downloadHandle, /^download_/);
      await f.run(row);
      assert.equal(f.receipts.at(-1)[2], signed ? 'download_queued' : 'download_saved');
      const packet = f.packets.find(p => signed ? p.url : p.byteOperation === 'begin');
      assert.deepEqual(packet.resolvedFile, { version: 1, name, mediaType: mime });
      assert.equal(JSON.parse(f.calls[0].init.body).mime_type, f.attachment.mime_type);
      assert.equal(JSON.parse(f.calls[0].init.body).index_for_retrieval, false);
      assert.doesNotMatch(JSON.stringify(f.packets), /external-gdrive|Bearer|mounted_library/);
      if (signed) assert.equal(f.saved, false, 'queued is not saved');
      else assert.equal(Buffer.concat(f.stored).toString(), 'mounted bytes');
    }
  });
}

test('mounted source MIME takes precedence over a prepared attachment MIME', async () => {
  const f = fixture({ attachment: true, resolvedFileVersion: 1,
    response: { file_id: 'file-materialized', file_name: 'resolved.docx' } });
  f.attachment.mime_type = EXPORTS[0][2];
  f.attachment.mounted_library_mime_type = 'application/vnd.google-apps.document';
  await f.run(f.register()[0]);
  assert.equal(f.receipts.at(-1)[2], 'download_saved');
  assert.equal(JSON.parse(f.calls[0].init.body).mime_type, 'application/vnd.google-apps.document');
  assert.equal(f.packets.find(p => p.byteOperation === 'begin').resolvedFile.mediaType, EXPORTS[0][2]);
});

test('metadata-only references accept the server-resolved format without exposing source identity', async () => {
  const name = 'a'.repeat(250) + '.xlsx';
  const f = fixture({ resolvedFileVersion: 1,
    response: { file_id: 'file-materialized', file_name: name, mime_type: EXPORTS[1][2] } });
  await f.run(f.register()[0]);
  assert.equal(f.receipts.at(-1)[2], 'download_saved');
  assert.deepEqual(f.packets.find(p => p.byteOperation === 'begin').resolvedFile,
    { version: 1, name, mediaType: EXPORTS[1][2] });
});

test('unsupported native metadata versions reject known cloud exports before the POST', async () => {
  for (const version of [undefined, 0, 2, '1']) {
    const f = fixture({ attachment: true, resolvedFileVersion: version });
    f.attachment.mime_type = 'application/vnd.google-apps.document';
    const row = f.register()[0];
    assert.match(row.downloadHandle, /^download_/);
    await f.run(row);
    assert.equal(f.calls.length, 0);
    assert.equal(f.saved, false);
  }
});

test('unsupported cloud formats and providers do not become materialization candidates', () => {
  for (const id of ['external-box:file:123', 'external-dropbox:file:id:synthetic', MOUNTED]) {
    const f = fixture({ attachment: true, id, resolvedFileVersion: 1 });
    f.attachment.mounted_library_mime_type = id === MOUNTED
      ? 'application/vnd.google-apps.drawing' : 'application/vnd.google-apps.document';
    assert.equal(f.register()[0]?.downloadHandle, undefined);
  }
});

test('mismatched exported formats and rejected native metadata never write file bytes', async () => {
  for (const patch of [{ file_name: 'wrong.xlsx' }, { mime_type: 'text/html' },
    { file_name: 'a'.repeat(1025) }, { file_name: 'bad\n.docx' }]) {
    const f = fixture({ attachment: true, resolvedFileVersion: 1, response: {
      file_id: 'file-materialized', file_name: 'fixture.docx', mime_type: EXPORTS[0][2], ...patch } });
    f.attachment.mime_type = 'application/vnd.google-apps.document';
    await f.run(f.register()[0]);
    assert.equal(f.calls.length, 1);
    assert.equal(f.saved, false);
    assert.equal(f.packets.some(p => p.byteOperation === 'begin'), false);
  }
  const f = fixture({ attachment: true, resolvedFileVersion: 1, rejectMetadata: true });
  await f.run(f.register()[0]);
  assert.equal(f.receipts.at(-1)[2], 'download_storage_failed');
  assert.equal(f.packets.some(p => p.byteOperation === 'chunk'), false);
});

test('a second click while materialization is pending does not cancel or replay the first POST', async () => {
  const f = fixture({ hang: true, fastTimeout: true }), row = f.register()[0];
  const first = f.run(row);
  await f.run(row);
  assert.equal(f.receipts.at(-1)[2], 'download_busy');
  assert.equal(f.calls.length, 1);
  await first;
  assert.equal(f.calls.length, 1);
  assert.equal(f.saved, false);
});

test('project conflicts and extra scope reject selection before a materializing write', () => {
  for (const mutate of [f => { f.payload.context_scopes = ['unexpected']; },
    f => { f.payload.project_id = 'g-p-' + 'b'.repeat(32); }]) {
    const f = fixture({ project: true }); mutate(f);
    assert.equal(f.register()[0].downloadHandle, undefined);
    assert.equal(f.calls.length, 0);
  }
});

test('cancellation, identity, route and document changes stop after materialization', async () => {
  for (const mutate of [f => f.api.cancel(), f => f.setIdentity('Bearer different-account'),
    f => { f.root.location.href = 'https://chatgpt.com/c/other'; },
    f => { f.root.__elonChatGptDocumentToken = 'doc_replaced'; }, f => f.api.dispose()]) {
    const f = fixture({ onMaterialize: mutate });
    await f.run(f.register()[0]);
    assert.equal(f.calls.length, 1);
    assert.equal(f.saved, false);
    assert.equal(f.receipts.at(-1)[1], false);
  }
});

test('a mismatched authorization cannot save the wrong materialized file', async () => {
  const f = fixture({ authorizationId: 'file-unrelated' });
  await f.run(f.register()[0]);
  assert.equal(f.calls.length, 2);
  assert.equal(f.saved, false);
  assert.equal(f.receipts.at(-1)[1], false);
});

test('installed bridges upgrade once and load mounted modules without replacing identity or audio', () => {
  const assets = path.join(__dirname, '../android/app/src/main/assets');
  const adapter = fs.readFileSync(path.join(assets, '../kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt'), 'utf8');
  const version = Number(/ADAPTER_VERSION = (\d+)/.exec(adapter)[1]);
  const bootstrap = fs.readFileSync(path.join(assets, 'chatgpt_web_adapter_bootstrap.js'), 'utf8');
  for (const previous of [294, 295, 296]) {
    const identity = {}, audio = {};
    let disposed = 0, retired = 0;
    const window = { location: { origin: 'https://chatgpt.com' },
      __elonChatGptAdapterVersion: previous, __elonChatGptAdapterTargetVersion: version,
      __elonChatGptBridge: { dispose() { disposed++; } },
      __elonChatGptPrivateAuthContext: identity, __elonChatGptPrivateRealtimeVoice: audio,
      __elonChatGptPrivateFileDownload: { version: 10, dispose() { retired++; } } };
    const context = { window, location: window.location };
    vm.runInNewContext(bootstrap, context);
    assert.equal(disposed, 1);
    for (const filename of ['chatgpt_web_private_history_projection.js',
      'chatgpt_web_private_library_download.js', 'chatgpt_web_private_file_download.js']) {
      vm.runInNewContext(fs.readFileSync(path.join(assets, filename), 'utf8'), context);
    }
    assert.equal(window.__elonChatGptPrivateHistoryProjection.version, 6);
    assert.equal(window.__elonChatGptPrivateLibraryDownload.version, 6);
    assert.equal(window.__elonChatGptPrivateFileDownload.version, 12);
    assert.equal(retired, 1);
    assert.equal(window.__elonChatGptPrivateAuthContext, identity);
    assert.equal(window.__elonChatGptPrivateRealtimeVoice, audio);
    vm.runInNewContext(bootstrap, context);
    vm.runInNewContext(fs.readFileSync(path.join(assets, 'chatgpt_web_private_file_download.js'), 'utf8'), context);
    assert.equal(disposed, 1); assert.equal(retired, 1);
  }
});

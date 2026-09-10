'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const { webcrypto } = require('node:crypto');
const catalog = require('../android/app/src/main/assets/chatgpt_web_private_library_catalog.js');
const download = require('../android/app/src/main/assets/chatgpt_web_private_file_download.js');
const library = require('../android/app/src/main/assets/chatgpt_web_private_library_download.js');
const mutations = require('../android/app/src/main/assets/chatgpt_web_private_library_mutations.js');
const request = require('../android/app/src/main/assets/chatgpt_web_private_json_request.js');
const DRIVE = 'external-gdrive:file:synthetic_file';
const SHAREPOINT = 'external-sharepoint:file:v1:item:' +
  Buffer.from('b!synthetic-drive').toString('base64url') + ':' + Buffer.from('synthetic-item').toString('base64url');
const MATERIALIZE = '/backend-api/files/library/mounted/materialize';
const CONTENT = '/backend-api/estuary/content?id=file-materialized&sig=synthetic';

function fixture(options = {}) {
  const node = { kind: 'file', id: options.id || DRIVE, name: 'fixture.txt',
    mime_type: 'text/plain', file_size_bytes: 13, ...options.node };
  const calls = [], events = [], packets = [], receipts = [], stored = [];
  let identity = 'Bearer synthetic-catalog-download', sequence = 0, saved = false;
  const bridge = { postMessage(raw) {
    const p = JSON.parse(raw); packets.push(p);
    if (p.cancel) { if (!saved) stored.length = 0; return; }
    let state;
    if (p.byteOperation === 'begin') state = 'ready';
    if (p.byteOperation === 'chunk') { stored.push(Buffer.from(p.data, 'base64')); state = 'written'; }
    if (p.byteOperation === 'commit') { saved = true; state = 'saved'; }
    queueMicrotask(() => bridge.onmessage?.({ data: JSON.stringify({ leaseId: p.leaseId,
      byteOperation: p.byteOperation, sequence: p.sequence, state }) }));
  } };
  const root = {
    location: { origin: 'https://chatgpt.com', href: options.href || 'https://chatgpt.com/' },
    __elonChatGptDocumentToken: 'doc_mounted_catalog', crypto: webcrypto, AbortController, btoa,
    setTimeout: (fn, ms) => setTimeout(fn, options.fastTimeout ? Math.min(ms, 25) : ms), clearTimeout,
    __elonChatGptPrivateLibraryDownload: library,
    __elonChatGptPrivateJsonRequest: request,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: identity }) },
    elonChatGptFileDownload: bridge,
    get document() { throw new Error('standalone download must not read the composer or DOM'); },
    fetch: async (url, init) => {
      calls.push({ url, init });
      const path = new URL(url).pathname;
      if (path === '/backend-api/files/library/nodes') return Response.json({ items: [node] });
      if (path === MATERIALIZE) {
        await options.onMaterialize?.(f);
        if (options.hang) return new Promise(() => {});
        if (options.status) return new Response('', { status: options.status });
        return Response.json(options.response || { file_id: 'file-materialized', file_name: 'fixture.txt',
          mime_type: 'text/plain', file_size_bytes: 13 });
      }
      if (path === '/backend-api/files/download/file-materialized') return Response.json({ status: 'success',
        file_id: 'file-materialized', download_url: CONTENT });
      assert.equal(url, 'https://chatgpt.com' + CONTENT);
      const response = new Response('mounted bytes', { headers: { 'content-type': options.mime || 'text/plain' } });
      Object.defineProperty(response, 'url', { value: url });
      return response;
    },
  };
  const owner = download.create(root);
  root.__elonChatGptPrivateFileDownload = owner;
  root.__elonChatGptPrivateLibraryMutations = mutations.create(root);
  const browser = catalog.create(root);
  const list = (operation = 'open') => browser.list({ requestId: 'mcp_' + ++sequence,
    value: JSON.stringify({ operation }) }, event => events.push(event), (...args) => receipts.push(args));
  const descriptor = row => ({ version: 1, byteTransferVersion: 1, resolvedFileVersion: 1,
    path: '/library', name: row.name, href: root.location.href, documentToken: root.__elonChatGptDocumentToken,
    downloadHandle: row.downloadHandle, leaseId: '00000000-0000-4000-8000-000000000001' });
  const run = row => owner.start(JSON.stringify(descriptor(row)), (...args) => receipts.push(args));
  const f = { root, owner, browser, node, calls, events, packets, receipts, stored, list, descriptor, run,
    setIdentity: value => { identity = value; }, get saved() { return saved; } };
  return f;
}

for (const id of [DRIVE, 'external-gdrive:account:synthetic_account:file:synthetic_file',
  'external-box:file:12345', 'external-dropbox:file:id:synthetic', SHAREPOINT]) {
  test('native catalogue download reuses mounted materialization: ' + id.split(':')[0], async () => {
    const f = fixture({ id });
    await f.list();
    const row = f.events.at(-1).items[0];
    assert.match(row.downloadHandle, /^download_[a-f0-9]{32}$/);
    assert.equal(row.canRename, false);
    assert.equal(row.canTrash, false);
    assert.equal(f.calls.length, 1, 'listing never materializes a file');
    assert.doesNotMatch(JSON.stringify(f.events), /external-|synthetic_file|Bearer/);
    f.node.id = 'external-box:file:9999';
    await f.run(row);
    assert.equal(f.receipts.at(-1)[2], 'download_saved');
    assert.equal(Buffer.concat(f.stored).toString(), 'mounted bytes');
    assert.equal(f.calls.length, 4);
    assert.deepEqual(JSON.parse(f.calls[1].init.body), { file_id: id, name: 'fixture.txt',
      mime_type: 'text/plain', index_for_retrieval: false });
    assert.equal(f.calls[1].init.method, 'POST');
    assert.equal(f.calls[1].init.credentials, 'same-origin');
    assert.equal(f.calls[1].init.redirect, 'error');
    assert.deepEqual(Object.fromEntries(new URL(f.calls[2].url).searchParams), { download_intent: 'true' });
    assert.equal(f.calls[3].init.headers, undefined);
    assert.doesNotMatch(JSON.stringify(f.packets), /external-|Bearer|materialize/);
    await f.run(row);
    assert.equal(f.receipts.at(-1)[2], 'download_selection_expired');
    assert.equal(f.calls.length, 4, 'consumed selections never replay the materialization write');
  });
}

test('library download never borrows the displayed ordinary, temporary or project conversation scope', async () => {
  for (const href of ['https://chatgpt.com/c/unrelated', 'https://chatgpt.com/?temporary-chat=true',
    'https://chatgpt.com/g/g-p-' + 'a'.repeat(32) + '/c/unrelated']) {
    const f = fixture({ href, node: { cloud_doc_url: 'https://drive.google.com/private/attribution',
      file_id: 'file-copied-but-not-selected' } });
    await f.list();
    await f.run(f.events.at(-1).items[0]);
    assert.equal(f.receipts.at(-1)[2], 'download_saved');
    assert.deepEqual(Object.fromEntries(new URL(f.calls[2].url).searchParams), { download_intent: 'true' });
    assert.equal(f.root.location.href, href);
    assert.ok(f.calls.every(call => new URL(call.url).origin === 'https://chatgpt.com'));
    assert.equal(JSON.parse(f.calls[1].init.body).file_id, DRIVE);
  }
});

for (const [source, suffix, mime] of [
  ['document', 'docx', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'],
  ['spreadsheet', 'xlsx', 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'],
  ['presentation', 'pptx', 'application/vnd.openxmlformats-officedocument.presentationml.presentation'],
]) {
  test('standalone Drive export keeps source MIME and resolves the native file: ' + source, async () => {
    const f = fixture({ node: { mime_type: 'application/vnd.google-apps.' + source, name: 'fixture' }, mime,
      response: { file_id: 'file-materialized', file_name: 'fixture.' + suffix } });
    await f.list();
    await f.run(f.events.at(-1).items[0]);
    assert.equal(f.receipts.at(-1)[2], 'download_saved');
    assert.equal(JSON.parse(f.calls[1].init.body).mime_type, f.node.mime_type);
    assert.deepEqual(f.packets.find(p => p.byteOperation === 'begin').resolvedFile,
      { version: 1, name: 'fixture.' + suffix, mediaType: mime });
  });
}

test('unsupported or ambiguous standalone scopes do not acquire a download handle', async () => {
  for (const node of [
    { id: 'external-gdrive:folder:synthetic' }, { kind: 'directory' },
    { id: 'https://drive.google.com/file/synthetic' }, { id: 'libfile_cloud', cloud_doc_url: 'https://example.com' },
    { external_account: {} }, { saved_entity: {} }, { library_artifact_type: 'saved_entity' },
    { trashed_at: '2026-09-10' }, { is_project: true }, { gizmo_id: 'g-p-' + 'a'.repeat(32) },
    { preview_file: { file_id: 'file-alternate' } }, { context_scopes: ['project'] },
    { library_provider: 'box' }, { mounted_library_file_id: 'external-box:file:9999' },
    { mime_type: 'application/vnd.google-apps.form' }, { id: 'external-box:file:12345', name: 'fixture.boxnote' },
  ]) {
    const f = fixture({ node });
    await f.list();
    assert.equal(f.events.at(-1).items[0].downloadHandle, '', JSON.stringify(node));
    assert.equal(f.calls.length, 1);
  }
});

test('cached listings reuse a live handle but new MIME or consumed identity cannot reuse it', async () => {
  const f = fixture();
  await f.list();
  const first = f.events.at(-1).items[0];
  await f.list();
  assert.equal(f.events.at(-1).items[0].downloadHandle, first.downloadHandle);
  f.node.mime_type = 'application/pdf';
  await f.list('refresh');
  const changed = f.events.at(-1).items[0];
  assert.notEqual(changed.downloadHandle, first.downloadHandle);
  await f.run(changed);
  await f.list();
  assert.notEqual(f.events.at(-1).items[0].downloadHandle, changed.downloadHandle);
});

test('identity, document, route, cancellation and timeout stop the transaction without alternate fetches', async () => {
  for (const boundary of ['identity', 'document', 'route', 'cancel', 'timeout', 'denied']) {
    const f = fixture({ fastTimeout: boundary === 'timeout', hang: boundary === 'timeout',
      status: boundary === 'denied' ? 403 : undefined,
      onMaterialize(f) {
        if (boundary === 'identity') f.setIdentity('Bearer synthetic-another-account');
        if (boundary === 'document') f.root.__elonChatGptDocumentToken = 'doc_replaced';
        if (boundary === 'route') f.root.location.href = 'https://chatgpt.com/c/other';
        if (boundary === 'cancel') f.owner.cancel('00000000-0000-4000-8000-000000000001');
      } });
    await f.list();
    const row = f.events.at(-1).items[0];
    assert.match(row.downloadHandle, /^download_/);
    await f.run(row);
    assert.equal(f.receipts.at(-1)[1], false, boundary);
    assert.equal(f.saved, false);
    assert.equal(f.calls.length, 2, boundary);
    await f.run(row);
    assert.equal(f.calls.length, 2, 'no automatic or duplicate-selector write replay');
  }
});

test('a duplicate click while materializing cannot create a second writer', async () => {
  let release, started;
  const began = new Promise(resolve => { started = resolve; });
  const f = fixture({ onMaterialize: () => { started(); return new Promise(resolve => { release = resolve; }); } });
  await f.list();
  const row = f.events.at(-1).items[0];
  assert.match(row.downloadHandle, /^download_/);
  const pending = f.run(row);
  await began;
  await f.run(row);
  assert.equal(f.receipts.at(-1)[2], 'download_busy');
  assert.equal(f.calls.length, 2);
  release();
  await pending;
  assert.equal(f.receipts.at(-1)[2], 'download_saved');
  assert.equal(f.calls.filter(call => new URL(call.url).pathname === MATERIALIZE).length, 1);
});

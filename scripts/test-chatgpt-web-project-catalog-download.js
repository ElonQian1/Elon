'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const { webcrypto } = require('node:crypto');
const download = require('../android/app/src/main/assets/chatgpt_web_private_file_download.js');
const library = require('../android/app/src/main/assets/chatgpt_web_private_library_download.js');
const request = require('../android/app/src/main/assets/chatgpt_web_private_json_request.js');
const PROJECT = 'g-p-' + 'a'.repeat(32), OTHER = 'g-p-' + 'b'.repeat(32);
const CONTENT = 'https://chatgpt.com/backend-api/estuary/content?id=file-synthetic';

function fixture(options = {}) {
  const calls = [], receipts = [], stored = [], packets = [];
  let identity = 'Bearer synthetic-project-catalog', saved = false;
  const source = { kind: 'file', id: 'libfile_synthetic', file_id: 'file-synthetic',
    is_project: true, gizmo_id: PROJECT, name: 'fixture.png', mime_type: 'image/png',
    ...options.source };
  const bridge = { onmessage: null, postMessage(raw) {
    const p = JSON.parse(raw); packets.push(p);
    if (p.cancel) return;
    let state;
    if (p.byteOperation === 'begin') state = 'ready';
    if (p.byteOperation === 'chunk') { stored.push(Buffer.from(p.data, 'base64')); state = 'written'; }
    if (p.byteOperation === 'commit') { saved = true; state = 'saved'; }
    queueMicrotask(() => bridge.onmessage?.({ data: JSON.stringify({ leaseId: p.leaseId,
      byteOperation: p.byteOperation, sequence: p.sequence, state }) }));
  } };
  const root = { location: { origin: 'https://chatgpt.com', href: options.href || 'https://chatgpt.com/c/unrelated' },
    __elonChatGptDocumentToken: 'doc_project_catalog', crypto: webcrypto, AbortController, btoa,
    setTimeout: (fn, ms) => setTimeout(fn, options.fastTimeout ? Math.min(ms, 25) : ms), clearTimeout,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: identity }) },
    __elonChatGptPrivateJsonRequest: request, __elonChatGptPrivateLibraryDownload: library,
    elonChatGptFileDownload: bridge,
    get document() { throw Error('catalogue download must not read DOM'); },
    fetch: async (url, init) => {
      calls.push({ url, init });
      const path = new URL(url).pathname;
      if (path === '/backend-api/files/file-synthetic/simple') {
        await options.onMetadata?.(f);
        if (options.hang) return new Promise(() => {});
        if (options.status) return new Response('', { status: options.status });
        return Response.json(options.info || { file_id: source.file_id, is_library_file: true,
          library_file_id: source.id, is_project: true, gizmo_id: PROJECT });
      }
      if (path === '/backend-api/files/download/file-synthetic') {
        return Response.json({ status: 'success', file_id: source.file_id, download_url: CONTENT });
      }
      if (url === CONTENT) {
        const response = new Response('synthetic PNG bytes', { headers: { 'content-type': 'image/png' } });
        Object.defineProperty(response, 'url', { value: url });
        return response;
      }
      return new Response('', { status: 404 });
    } };
  const owner = download.create(root);
  const register = () => owner.registerLibraryFile(source);
  const run = handle => owner.start(JSON.stringify({ version: 1, byteTransferVersion: 1,
    path: '/library', name: source.name, href: root.location.href, documentToken: root.__elonChatGptDocumentToken,
    downloadHandle: handle, leaseId: '00000000-0000-4000-8000-000000000001' }),
  (...args) => receipts.push(args));
  const f = { source, root, owner, calls, receipts, packets, stored, register, run,
    identity: value => { identity = value; }, get saved() { return saved; } };
  return f;
}

for (const href of ['https://chatgpt.com/', 'https://chatgpt.com/c/unrelated',
  `https://chatgpt.com/g/${OTHER}/c/unrelated`, 'https://chatgpt.com/?temporary-chat=true']) {
  test('project catalogue uses source scope, not current route: ' + new URL(href).pathname, async () => {
    const f = fixture({ href }), handle = f.register();
    assert.match(handle, /^download_[a-f0-9]{32}$/);
    assert.equal(f.calls.length, 0);
    await f.run(handle);
    assert.equal(f.receipts.at(-1)[2], 'download_saved');
    assert.equal(f.saved, true);
    assert.equal(Buffer.concat(f.stored).toString(), 'synthetic PNG bytes');
    assert.equal(f.calls.length, 3);
    for (const c of f.calls.slice(0, 2)) {
      assert.equal(c.init.method, 'GET');
      assert.equal(c.init.credentials, 'same-origin');
      assert.equal(c.init.redirect, 'error');
      const query = Object.fromEntries(new URL(c.url).searchParams);
      assert.deepEqual(query, c === f.calls[0] ? { gizmo_id: PROJECT } : { gizmo_id: PROJECT, download_intent: 'true' });
    }
    assert.equal(f.calls.some(c => c.url.includes('/api/library/')), false);
  });
}

test('missing catalogue project ID is resolved from authoritative metadata', async () => {
  const f = fixture({ source: { gizmo_id: undefined } });
  await f.run(f.register());
  assert.equal(f.saved, true);
  assert.equal(new URL(f.calls[0].url).search, '');
  assert.equal(new URL(f.calls[1].url).searchParams.get('gizmo_id'), PROJECT);
});

for (const change of [
  { file_id: 'file-other' }, { library_file_id: 'libfile_other' }, { is_project: false },
  { gizmo_id: OTHER }, { is_library_file: false }, { gizmo_id: null },
]) test('metadata mismatch does not fall through: ' + JSON.stringify(change), async () => {
  const f = fixture({ info: { file_id: 'file-synthetic', library_file_id: 'libfile_synthetic',
    is_library_file: true, is_project: true, gizmo_id: PROJECT, ...change } });
  await f.run(f.register());
  assert.equal(f.saved, false);
  assert.equal(f.receipts.at(-1)[2], 'download_prepare_failed');
  assert.equal(f.calls.length, 1);
});

for (const source of [
  { file_id: undefined }, { file_id: 'libfile_other' }, { gizmo_id: 'invalid' }, { is_project: 'true' },
  { project_id: OTHER }, { context_scopes: [] }, { preview_file: {} }, { external_account: {} },
  { cloud_doc_url: 'https://example.org/private' }, { library_artifact_type: 'image' }, { saved_entity: {} },
]) test('ambiguous project source has no download handle: ' + Object.keys(source)[0], () => {
  const f = fixture({ source });
  assert.equal(f.register(), '');
  assert.equal(f.calls.length, 0);
});

for (const change of [f => f.identity('Bearer changed-synthetic-identity'),
  f => { f.root.location.href = 'https://chatgpt.com/c/changed'; },
  f => { f.root.__elonChatGptDocumentToken = 'doc_changed'; }]) {
  test('changed owner prevents late authorization', async () => {
    const f = fixture({ onMetadata: change });
    await f.run(f.register());
    assert.equal(f.saved, false);
    assert.equal(f.calls.length, 1);
  });
}
test('selection captures project backing identity and refresh does not reuse changed scope', async () => {
  const f = fixture(), original = f.register();
  f.source.file_id = 'file-other';
  assert.notEqual(f.register(), original);
  f.source.file_id = 'file-synthetic'; f.source.gizmo_id = OTHER;
  assert.notEqual(f.register(), original);
});
for (const status of [403, 404]) test('metadata denial is final without alternate request: ' + status, async () => {
  const f = fixture({ status });
  await f.run(f.register());
  assert.equal(f.saved, false);
  assert.equal(f.calls.length, 1);
});
test('ignored metadata abort remains bounded without retry', async () => {
  const f = fixture({ hang: true, fastTimeout: true });
  await f.run(f.register());
  assert.equal(f.saved, false);
  assert.equal(f.calls.length, 1);
});

'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const { webcrypto } = require('node:crypto');
const projection = require('../android/app/src/main/assets/chatgpt_web_private_history_projection.js');
const download = require('../android/app/src/main/assets/chatgpt_web_private_file_download.js');
const request = require('../android/app/src/main/assets/chatgpt_web_private_json_request.js');

function fixture() {
  let account = 'Bearer synthetic-file-download';
  const calls = [], queued = [], receipts = [], cancelled = [];
  const payload = { messages: [{ id: 'message-1', author: { role: 'user' },
    content: { parts: ['test'] }, metadata: { attachments: [{ id: 'file-synthetic', name: 'fixture.txt', mime_type: 'text/plain' }] } }] };
  const bridge = { onmessage: null, postMessage(raw) {
    const value = JSON.parse(raw);
    if (value.cancel === true) { cancelled.push(value); return; }
    queued.push(value);
    queueMicrotask(() => bridge.onmessage?.({ data: JSON.stringify({ leaseId: value.leaseId, state: 'queued' }) }));
  } };
  const root = {
    location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/current' },
    __elonChatGptDocumentToken: 'doc_synthetic_1',
    __elonChatGptPrivateHistoryProjection: projection,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: account }) },
    __elonChatGptPrivateJsonRequest: request, elonChatGptFileDownload: bridge,
    AbortController, crypto: webcrypto, setTimeout, clearTimeout,
    fetch: async (url, init) => {
      calls.push({ url, init });
      return new Response(JSON.stringify({ status: 'success', file_id: 'file-synthetic',
        download_url: 'https://files.oaiusercontent.com/fixture?sig=synthetic' }));
    },
  };
  const service = download.create(root);
  const register = (path = '/c/source') => service.register(path, payload, projection.create({}).files(payload));
  const descriptor = (rows = register(), path = '/c/source') => ({ version: 1,
    leaseId: '00000000-0000-4000-8000-000000000001', documentToken: root.__elonChatGptDocumentToken,
    href: root.location.href, path, name: 'fixture.txt', downloadHandle: rows[0]?.downloadHandle });
  const run = value => service.start(JSON.stringify(value || descriptor()), (...args) => receipts.push(args));
  return { root, calls, queued, receipts, cancelled, payload, bridge, service, register, descriptor, run,
    setAccount: value => { account = value; } };
}

test('official download authorization uses selected conversation without navigation or DOM', async () => {
  const f = fixture(), original = f.root.location.href;
  await f.run();
  assert.equal(f.calls.length, 1);
  const url = new URL(f.calls[0].url);
  assert.equal(url.pathname, '/backend-api/files/download/file-synthetic');
  assert.equal(url.searchParams.get('conversation_id'), 'source');
  assert.equal(url.searchParams.get('download_intent'), 'true');
  assert.equal(f.calls[0].init.method, 'GET');
  assert.equal(f.calls[0].init.redirect, 'error');
  assert.equal(f.queued.length, 1);
  assert.deepEqual(Object.keys(f.queued[0]).sort(), ['documentToken', 'leaseId', 'url']);
  assert.deepEqual(f.receipts, [['download_conversation_file', true, 'download_queued']]);
  assert.equal(f.root.location.href, original);
});

test('native descriptors contain opaque expiring handles, not file IDs or authorization URLs', () => {
  const f = fixture(), rows = f.register();
  assert.match(rows[0].downloadHandle, /^download_[a-f0-9]{32}$/);
  assert.ok(!JSON.stringify(rows).includes('file-synthetic'));
  assert.ok(!JSON.stringify(rows).includes('Bearer'));
  assert.equal(projection.create({}).files(f.payload).files[0].downloadHandle, undefined);
  f.payload.messages[0].metadata.attachments[0].id = 'file_000000synthetic';
  assert.match(f.register()[0].downloadHandle, /^download_[a-f0-9]{32}$/);
});

test('project, library, connector, image pointer and unknown file identifiers stay unclaimed', () => {
  for (const patch of [{ id: '../x' }, { id: 'file-x?scope=other' }, { library_file_id: 'library-1' },
    { gizmo_id: 'g-p-test' }, { context_connector: {} }, { source_url: 'https://external.test' }]) {
    const f = fixture();
    Object.assign(f.payload.messages[0].metadata.attachments[0], patch);
    assert.equal(f.register()[0].downloadHandle, undefined);
  }
  const f = fixture();
  assert.equal(f.register('/g/g-p-test/c/source')[0].downloadHandle, undefined);
  f.payload.gizmo_id = 'g-p-test';
  assert.equal(f.register()[0].downloadHandle, undefined);
  delete f.payload.gizmo_id;
  f.payload.messages[0].content.parts.unshift({ content_type: 'image_asset_pointer', asset_pointer: 'sediment://file-image' });
  const rows = f.register();
  assert.equal(rows[0].downloadHandle, undefined);
  assert.match(rows[1].downloadHandle, /^download_/);
});

test('selection handles cannot be retargeted, reused after refresh or crossed between accounts/documents', async () => {
  for (const mutate of [
    (f, d) => { d.path = '/c/other'; }, (f, d) => { d.name = 'other.txt'; },
    f => f.setAccount('Bearer another-account-token'),
    f => { f.root.__elonChatGptDocumentToken = 'doc_replacement_2'; },
    f => { f.register(); },
  ]) {
    const f = fixture(), d = f.descriptor();
    mutate(f, d);
    await f.run(d);
    assert.equal(f.queued.length, 0);
    assert.equal(f.receipts[0][1], false);
  }
});

test('a late authorization response cannot enqueue after account, document or route changes', async () => {
  for (const mutate of [f => f.setAccount('Bearer changed-account-token'),
    f => { f.root.__elonChatGptDocumentToken = 'doc_replacement_2'; },
    f => { f.root.location.href += '-other'; }]) {
    const f = fixture(), fetch = f.root.fetch;
    f.root.fetch = async (...args) => { const result = await fetch(...args); mutate(f); return result; };
    await f.run();
    assert.equal(f.queued.length, 0);
    assert.equal(f.receipts[0][1], false);
  }
});

test('retry, wrong file, expired file and invalid signed origins never report a successful download', async () => {
  for (const body of [{ status: 'retry' }, { status: 'success', file_id: 'file-other' },
    ...['http://files.oaiusercontent.com/a', 'https://oaiusercontent.com.evil.test/a',
      'https://user:pass@files.oaiusercontent.com/a', 'https://files.oaiusercontent.com:8443/a',
      'https://chatgpt.com/backend-api/secret'].map(download_url => ({ status: 'success', download_url }))]) {
    const f = fixture();
    f.root.fetch = async () => new Response(JSON.stringify(body));
    await f.run();
    assert.equal(f.queued.length, 0);
    assert.equal(f.receipts[0][1], false);
  }
  const f = fixture();
  f.root.fetch = async () => new Response('', { status: 404 });
  await f.run();
  assert.equal(f.receipts[0][2], 'download_file_unavailable');
  assert.equal(f.cancelled.length, 1);
  assert.equal(f.cancelled[0].cancel, true);
});

test('single-flight, cancellation and oversized authorization responses are bounded with no write replay', async () => {
  const f = fixture();
  let resolve;
  f.root.fetch = () => new Promise(done => { resolve = done; });
  const d = f.descriptor(), first = f.run(d);
  await f.run(d);
  assert.equal(f.receipts[0][2], 'download_busy');
  assert.equal(f.cancelled.length, 0);
  f.service.cancel();
  await first;
  resolve(new Response(JSON.stringify({ status: 'success' })));
  assert.equal(f.queued.length, 0);
  const large = fixture();
  large.root.fetch = async () => new Response('x'.repeat(65537));
  await large.run();
  assert.equal(large.queued.length, 0);
});

test('registry is bounded, expires old selections and dispose restores pending bridge listener', async () => {
  const expired = fixture(), selection = expired.descriptor(), now = Date.now;
  try {
    Date.now = () => now() + 120001;
    await expired.run(selection);
    assert.equal(expired.receipts[0][2], 'download_selection_expired');
  } finally { Date.now = now; }
  const f = fixture(), d = f.descriptor();
  for (let index = 0; index < 801; index++) f.register('/c/other-' + index);
  await f.run(d);
  assert.equal(f.receipts[0][2], 'download_selection_expired');
  const g = fixture(), original = () => {};
  g.bridge.onmessage = original;
  g.bridge.postMessage = () => { g.service.dispose(); };
  await g.run();
  assert.equal(g.bridge.onmessage, original);
  assert.equal(g.receipts[0][1], false);
});

test('a new context retires its old pending authorization instead of waiting for the old timeout', async () => {
  const f = fixture(), fetch = f.root.fetch;
  f.root.fetch = () => new Promise(() => {});
  const old = f.run();
  f.root.location.href = 'https://chatgpt.com/c/replacement';
  f.root.fetch = fetch;
  await f.run();
  await old;
  assert.equal(f.queued.length, 1);
  assert.equal(f.receipts.filter(item => item[1]).length, 1);
});

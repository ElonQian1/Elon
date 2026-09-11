'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const { webcrypto } = require('node:crypto');
const download = require('../android/app/src/main/assets/chatgpt_web_private_file_download.js');
const json = require('../android/app/src/main/assets/chatgpt_web_private_json_request.js');
const gallery = require('../android/app/src/main/assets/chatgpt_web_private_image_gallery.js');
const contentSource = require('../android/app/src/main/assets/chatgpt_web_private_content_source.js');

function fixture() {
  const calls = [], packets = [], receipts = [];
  let valid = true, account = 'Bearer synthetic-gallery-original';
  const bridge = { postMessage(raw) {
    const packet = JSON.parse(raw);
    if (packet.cancel) return;
    packets.push(packet);
    queueMicrotask(() => bridge.onmessage?.({ data: JSON.stringify({ leaseId: packet.leaseId, state: 'queued' }) }));
  } };
  const root = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/original' },
    __elonChatGptDocumentToken: 'doc_gallery_original',
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: account }) },
    __elonChatGptPrivateJsonRequest: json, elonChatGptFileDownload: bridge,
    __elonChatGptPrivateContentSource: contentSource,
    AbortController, crypto: webcrypto, setTimeout, clearTimeout,
    get document() { throw new Error('no DOM'); },
    fetch: async (url, init) => {
      calls.push({ url: new URL(url), init });
      return Response.json({ status: 'success', download_url: 'https://files.oaiusercontent.com/original?sig=synthetic' });
    } };
  const owner = download.create(root);
  const row = { asset_pointer: 'sediment://file-original?variant=final', conversation_id: 'source-conversation' };
  const select = (item = row) => owner.registerGalleryImage(item, () => valid);
  const run = handle => owner.start(JSON.stringify({ version: 1, path: '/images', name: 'image.png',
    downloadHandle: handle, documentToken: root.__elonChatGptDocumentToken, href: root.location.href,
    leaseId: '00000000-0000-4000-8000-000000000001' }), (...args) => receipts.push(args));
  return { root, owner, row, select, run, calls, packets, receipts,
    expire: () => { valid = false; }, switchAccount: () => { account = 'Bearer synthetic-other-account'; } };
}

test('gallery original download reuses scoped authorization and the native lease, never preview bytes', async () => {
  const f = fixture(), handle = f.select();
  assert.match(handle, /^download_[a-f0-9]{32}$/);
  assert.equal(f.calls.length, 0);
  await f.run(handle);
  assert.equal(f.calls.length, 1);
  const { url, init } = f.calls[0];
  assert.equal(url.pathname, '/backend-api/files/download/file-original');
  assert.deepEqual([...url.searchParams], [['variant', 'final'],
    ['check_context_scopes_for_conversation_id', 'source-conversation'], ['download_intent', 'true']]);
  assert.equal(init.method, 'GET');
  assert.equal(f.packets.length, 1);
  assert.deepEqual(f.receipts, [['download_conversation_file', true, 'download_queued']]);
  assert.equal(f.root.location.href, 'https://chatgpt.com/c/original');
});

test('gallery selection rejects unknown source scopes without guessing', () => {
  const f = fixture();
  for (const change of [{ conversation_id: null }, { conversation_id: 'bad/path' },
    { gizmo_id: 'g-p-0123456789abcdef0123456789abcdef' }, { library_file_id: 'libfile_other' },
    { post_id: 'post-one' }, { shared: true }, { is_project: true },
    { watermarked_url: 'https://files.oaiusercontent.com/marked' },
    { watermarkedUrl: 'https://files.oaiusercontent.com/marked' },
    { asset_pointer: 'https://example.test/image' }, { asset_pointer: 'sediment://file-one?gizmo_id=other' }]) {
    assert.equal(f.select({ ...f.row, ...change }), '', JSON.stringify(change));
  }
  assert.equal(f.calls.length, 0);
});

test('catalog original URL uses byte storage without stale pointer reauthorization or thumbnail export', async () => {
  const f = fixture(), urls = [];
  const url = '/backend-api/estuary/content?id=synthetic-original&sig=synthetic';
  f.root.__elonChatGptPrivateLibraryDownload = { contentUrl: contentSource.contentUrl,
    runContent: async (root, job, current, source) => {
      assert.equal(current(job), true); urls.push(source); return 'download_saved';
    } };
  const handle = f.select({ ...f.row, url, encodings: { thumbnail: { path: '/api/estuary/content?id=small' } } });
  assert.match(handle, /^download_/);
  await f.run(handle);
  assert.deepEqual(urls, ['https://chatgpt.com' + url]);
  assert.equal(f.calls.length, 0);
  assert.equal(f.packets.length, 0);
  assert.equal(f.receipts.at(-1)[2], 'download_saved');
});

test('expired original URL reports failure once without probing a different authorization path', async () => {
  const f = fixture(); let transfers = 0;
  f.root.__elonChatGptPrivateLibraryDownload = { contentUrl: contentSource.contentUrl,
    runContent: async () => { transfers++; throw new Error('download_file_unavailable'); } };
  await f.run(f.select({ ...f.row, url: '/api/estuary/content?id=expired' }));
  assert.equal(transfers, 1);
  assert.equal(f.calls.length, 0);
  assert.equal(f.receipts.at(-1)[2], 'download_file_unavailable');
  for (const url of ['https://evil.test/image', 'data:image/png;base64,AAA', '/api/arbitrary', '']) {
    assert.equal(f.select({ ...f.row, url }), '');
  }
});

test('closed gallery, changed account and switched document invalidate the original selection', async () => {
  for (const change of [f => f.expire(), f => f.switchAccount(), f => { f.root.__elonChatGptDocumentToken = 'doc_other'; }]) {
    const f = fixture(), handle = f.select();
    change(f);
    await f.run(handle);
    assert.equal(f.calls.length, 0);
    assert.equal(f.receipts.at(-1)[1], false);
  }
});

test('real gallery publishes aligned opaque selections, keeps ownership after previews, and revokes on close', async () => {
  const f = fixture(), events = [];
  f.root.__elonChatGptPrivateFileDownload = f.owner;
  const fetch = f.root.fetch;
  f.root.fetch = (url, init) => new URL(url).pathname.endsWith('/image_gen')
    ? Promise.resolve(Response.json({ items: [f.row], cursor: null })) : fetch(url, init);
  f.root.__elonChatGptImageAssets = { registerPrivate: () => 'image_0123456789abcdef',
    request: async () => ({ ok: true }), cancel() {} };
  const api = gallery.create(f.root);
  const result = await api.request({ requestId: 'mcp_gallery1',
    value: JSON.stringify({ operation: 'open', cachedHandles: [] }) }, event => events.push(event));
  assert.equal(result.ok, true);
  const page = events.at(-1);
  assert.equal(page.handles.length, page.downloadHandles.length);
  assert.match(page.downloadHandles[0], /^download_[a-f0-9]{32}$/);
  assert.doesNotMatch(JSON.stringify(events), /source-conversation|file-original|variant|Bearer|oaiusercontent/);
  assert.equal(f.calls.length, 0);
  await f.run(page.downloadHandles[0]);
  assert.equal(f.calls.length, 1);
  api.cancel();
  await f.run(page.downloadHandles[0]);
  assert.equal(f.calls.length, 1);
  assert.equal(f.receipts.at(-1)[2], 'download_selection_expired');
});

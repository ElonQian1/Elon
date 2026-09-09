'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { createHash } = require('node:crypto');
const { create } = require('../android/app/src/main/assets/chatgpt_web_private_image_gallery.js');
const row = n => ({ id: 'generation-' + n, asset_pointer: 'file-service://file-' + n,
  conversation_id: 'conversation-' + n, created_at: 1700000000 });
const deferred = () => { let resolve; const promise = new Promise(r => { resolve = r; }); return { promise, resolve }; };

function harness(pages = [{ items: [row(1)], cursor: null }]) {
  const events = [], calls = [], exports = [], registry = new Map(), cancelled = [], timers = new Map();
  let timerId = 0, id = 0;
  let headers = { authorization: 'Bearer synthetic-account-one', 'chatgpt-account-id': 'account-one' };
  const root = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/original' },
    __elonChatGptDocumentToken: 'doc_synthetic', AbortController,
    setTimeout: (fn, ms) => { timers.set(++timerId, { fn, ms }); return timerId; },
    clearTimeout: n => timers.delete(n),
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => headers },
    __elonChatGptPrivateJsonRequest: { request: async (_, url, init, limits) => {
      calls.push({ url: new URL(url), init, limits });
      const parsed = new URL(url);
      if (parsed.pathname.endsWith('/image_gen')) {
        const index = Number(parsed.searchParams.get('after') || 0);
        const response = pages[index];
        return { payload: typeof response === 'function' ? await response() : response };
      }
      return { payload: { status: 'success', download_url: 'https://files.oaiusercontent.com/synthetic?sig=private' } };
    } },
    __elonChatGptImageAssets: {
      registerPrivate: (identity, resolve, current) => {
        const handle = 'image_' + createHash('sha256').update(identity).digest('hex').slice(0, 16);
        registry.set(handle, { resolve, current }); return handle;
      },
      request: async (handle, emit) => {
        exports.push(handle);
        const entry = registry.get(handle);
        await entry.resolve(new AbortController().signal);
        if (!entry.current()) return { ok: false };
        emit({ type: 'image_asset', handle, state: 'ready', data: 'synthetic-preview' });
        return { ok: true };
      },
      cancel: handle => cancelled.push(handle),
    },
  };
  const api = create(root);
  return { api, root, events, calls, exports, registry, timers, cancelled,
    run: (operation = 'open', cachedHandles = []) => api.request({ requestId: 'mcp_gallery' + ++id,
      value: JSON.stringify({ operation, cachedHandles }) }, event => events.push(event)),
    snapshots: () => events.filter(e => e.type === 'image_gallery_snapshot'),
    catalogCalls: () => calls.filter(c => c.url.pathname.endsWith('/image_gen')),
    switchAccount: () => { headers = { authorization: 'Bearer synthetic-account-two', 'chatgpt-account-id': 'account-two' }; },
  };
}

test('reads the inspected generated catalog and file resolver without navigating or DOM', async () => {
  const h = harness();
  assert.equal((await h.run()).ok, true);
  const request = h.catalogCalls()[0];
  assert.equal(request.url.pathname, '/backend-api/my/recent/image_gen');
  assert.equal(request.url.search, '?limit=25');
  assert.equal(request.init.method, 'GET');
  assert.equal(request.init.credentials, 'include');
  assert.equal(request.init.redirect, 'error');
  const image = h.calls[1].url;
  assert.equal(image.pathname, '/backend-api/files/download/file-1');
  assert.equal(image.searchParams.get('conversation_id'), 'conversation-1');
  assert.equal(image.searchParams.get('inline'), 'true');
  assert.equal(image.searchParams.get('download_intent'), 'false');
  assert.equal(h.snapshots().at(-1).state, 'ready');
  assert.equal(h.root.location.href, 'https://chatgpt.com/c/original');
  assert.equal(h.timers.size, 0);
  assert.doesNotMatch(JSON.stringify(h.events), /synthetic-account|conversation-1|file-1|sig=private|oaiusercontent/);
});

test('warm reopen reuses catalog and native thumbnails without any request', async () => {
  const h = harness();
  await h.run();
  const handles = h.snapshots().at(-1).handles;
  h.calls.length = 0; h.exports.length = 0;
  assert.equal((await h.run('open', handles)).ok, true);
  assert.equal(h.calls.length, 0);
  assert.equal(h.exports.length, 0);
  await h.run('refresh', handles);
  assert.equal(h.catalogCalls().length, 1, 'explicit refresh bypasses catalog cache only');
  assert.equal(h.exports.length, 0);
});

test('cursor paging preserves order and previous page cache', async () => {
  const h = harness([{ items: [row(1), row(2)], cursor: '1' }, { items: [row(3)], cursor: null }]);
  await h.run();
  const first = h.snapshots().at(-1);
  assert.equal(first.hasNext, true);
  assert.equal(first.hasPrevious, false);
  await h.run('next');
  const second = h.snapshots().at(-1);
  assert.equal(second.pageIndex, 1);
  assert.equal(second.hasNext, false);
  assert.equal(second.hasPrevious, true);
  assert.equal(h.catalogCalls()[1].url.searchParams.get('after'), '1');
  await h.run('previous', first.handles);
  assert.deepEqual(h.snapshots().at(-1).handles, first.handles);
  assert.equal(h.catalogCalls().length, 2);
});

test('oversized official batches use native subpages without losing items or server cursors', async () => {
  const h = harness([{ items: Array.from({ length: 61 }, (_, i) => row(i)), cursor: '1' },
    { items: [row(61)], cursor: null }]);
  const all = [], snapshots = [];
  for (const operation of ['open', 'next', 'next', 'next']) {
    assert.equal((await h.run(operation)).ok, true);
    snapshots.push(h.snapshots().at(-1));
    all.push(...h.snapshots().at(-1).handles);
  }
  assert.deepEqual(snapshots.map(s => s.observedCount), [25, 25, 11, 1]);
  assert.deepEqual(snapshots.map(s => s.pageIndex), [0, 1, 2, 3]);
  assert.deepEqual(snapshots.map(s => s.hasNext), [true, true, true, false]);
  assert.equal(new Set(all).size, 62);
  assert.equal(h.catalogCalls().length, 2, 'subpages share the same bounded server batch');
  assert.equal(h.catalogCalls()[1].url.searchParams.get('after'), '1');
  assert.equal((await h.run('previous', all)).ok, true);
  assert.deepEqual(h.snapshots().at(-1).handles, snapshots[2].handles);
  assert.equal(h.catalogCalls().length, 2);
});

test('expired native subpage refetches its server batch and preserves its offset', async () => {
  const h = harness([{ items: Array.from({ length: 51 }, (_, i) => row(i)), cursor: null }]);
  await h.run();
  await h.run('next');
  const second = h.snapshots().at(-1).handles, originalNow = Date.now;
  const future = originalNow() + 120001;
  try {
    Date.now = () => future;
    assert.equal((await h.run('open', second)).ok, true);
    assert.deepEqual(h.snapshots().at(-1).handles, second);
    assert.equal(h.snapshots().at(-1).pageIndex, 1);
    assert.equal(h.catalogCalls().at(-1).url.searchParams.get('after'), null);
  } finally { Date.now = originalNow; }
});

test('a valid terminal empty catalog is ready, malformed or unknown payload is not empty success', async () => {
  const empty = harness([{ items: [], cursor: null }]);
  assert.equal((await empty.run()).ok, true);
  assert.equal(empty.snapshots().at(-1).observedCount, 0);
  assert.equal(empty.snapshots().at(-1).state, 'ready');
  for (const payload of [{}, { items: null }, { items: [null] }, { items: [], cursor: 'next' },
    { items: [row(1)], cursor: 42 }, { items: Array(257).fill(row(1)) }]) {
    const h = harness([payload]);
    assert.equal((await h.run()).ok, false);
    assert.equal(h.snapshots().at(-1).state, 'failed');
    assert.equal(h.exports.length, 0);
  }
});

test('gallery failures retain safe admission and catalog evidence, not response content', async () => {
  for (const [payload, reason] of [
    [{}, 'catalog_items_invalid'],
    [{ items: [null] }, 'catalog_item_invalid'],
    [{ items: Array(257).fill(row(1)) }, 'catalog_page_limit'],
    [{ items: [row(1)], cursor: '' }, 'catalog_cursor_invalid'],
    [{ items: [], cursor: 'private-cursor' }, 'catalog_empty_continuation'],
  ]) {
    const h = harness([payload]);
    assert.equal((await h.run()).code, 'private_image_gallery_unavailable:catalog:' + reason);
    assert.equal(h.exports.length, 0);
    assert.equal(h.timers.size, 0);
  }
  for (const [message, expected] of [['http_401', 'http_401'], ['timeout', 'timeout'],
    ['https://private.invalid?token=secret', 'unexpected']]) {
    const h = harness([() => { throw new Error(message); }]);
    const result = await h.run();
    assert.equal(result.code, 'private_image_gallery_unavailable:catalog:' + expected);
    assert.doesNotMatch(JSON.stringify([result, h.events]), /secret|private\.invalid/);
  }
  const noIdentity = harness();
  noIdentity.root.__elonChatGptPrivateTransport.copySameOriginRequestHeaders = () => null;
  assert.equal((await noIdentity.run()).code,
    'private_image_gallery_unavailable:identity:identity_unavailable');
});

test('unrecognized pointer scopes are partial rather than fabricated empty gallery', async () => {
  const h = harness([{ items: [row(1), { ...row(2), asset_pointer: 'file-service://file-2?shared=unknown' }], cursor: null }]);
  assert.equal((await h.run()).ok, false);
  const last = h.snapshots().at(-1);
  assert.equal(last.state, 'partial');
  assert.equal(last.observedCount, 2);
  assert.equal(last.unavailableCount, 1);
  assert.equal(last.handles.length, 1);
});

test('parameterized gallery previews retain the official query and warm cache', async () => {
  const item = { ...row(1), asset_pointer: 'sediment://file-1#preview?variant=first&label=A+B&variant=last' };
  const h = harness([{ items: [item], cursor: null }]);
  assert.equal((await h.run()).ok, true);
  const image = h.calls[1].url;
  assert.equal(image.pathname, '/backend-api/files/download/file-1*preview');
  assert.equal(image.searchParams.get('variant'), 'last');
  assert.equal(image.searchParams.get('label'), 'A B');
  assert.equal(image.searchParams.get('conversation_id'), 'conversation-1');
  assert.equal(image.searchParams.get('inline'), 'true');
  assert.equal(image.searchParams.get('download_intent'), 'false');
  const handles = h.snapshots().at(-1).handles;
  h.calls.length = 0;
  assert.equal((await h.run('open', handles)).ok, true);
  assert.equal(h.calls.length, 0);
  assert.doesNotMatch(JSON.stringify(h.events), /variant|label|file-1|conversation-1/);
});

test('different pointer variants never alias one cached thumbnail', async () => {
  const h = harness([{ items: [
    { ...row(1), asset_pointer: 'file-service://file-1?variant=one' },
    { ...row(1), asset_pointer: 'file-service://file-1?variant=two' },
  ], cursor: null }]);
  assert.equal((await h.run()).ok, true);
  assert.equal(new Set(h.snapshots().at(-1).handles).size, 2);
  assert.deepEqual(h.calls.slice(1).map(c => c.url.searchParams.get('variant')), ['one', 'two']);
});

test('segmented gallery pointers resolve as encoded IDs without conflating cached images', async () => {
  const ids = ['file-1/container/image.png#preview', 'file-1/other/image.png#preview'];
  const h = harness([{ items: ids.map(id => ({ ...row(1), asset_pointer: 'sediment://' + id })), cursor: null }]);
  assert.equal((await h.run()).ok, true);
  const handles = h.snapshots().at(-1).handles;
  assert.equal(new Set(handles).size, 2);
  assert.deepEqual(h.calls.slice(1).map(c => c.url.pathname),
    ids.map(id => '/backend-api/files/download/' + encodeURIComponent(id.replaceAll('#', '*'))));
  assert.ok(h.calls.slice(1).every(c => c.url.searchParams.get('conversation_id') === 'conversation-1'));
  h.calls.length = 0;
  assert.equal((await h.run('open', handles)).ok, true);
  assert.equal(h.calls.length, 0);
  assert.doesNotMatch(JSON.stringify(h.events), /container|other\/image|conversation-1|file-1/);
});

test('preview registration snapshots the selected pointer before asynchronous resolution', async () => {
  const item = { ...row(1), asset_pointer: 'file-service://file-1?variant=selected' };
  const h = harness([{ items: [item], cursor: null }]);
  const register = h.root.__elonChatGptImageAssets.registerPrivate;
  h.root.__elonChatGptImageAssets.registerPrivate = (...args) => {
    item.asset_pointer = 'sediment://file-other?variant=changed';
    item.conversation_id = 'conversation-other';
    return register(...args);
  };
  assert.equal((await h.run()).ok, true);
  assert.equal(h.calls[1].url.pathname, '/backend-api/files/download/file-1');
  assert.equal(h.calls[1].url.searchParams.get('variant'), 'selected');
  assert.equal(h.calls[1].url.searchParams.get('conversation_id'), 'conversation-1');
});

test('gallery rejects non-string conversation identities rather than coercing scope', async () => {
  for (const conversation_id of [123, true, {}, '']) {
    const h = harness([{ items: [{ ...row(1), conversation_id }], cursor: null }]);
    assert.equal((await h.run()).ok, false);
    assert.equal(h.exports.length, 0);
  }
});

test('gallery never drops explicit unresolved project, library or connector scope', async () => {
  for (const key of ['gizmo_id', 'project_id', 'post_id', 'library_file_id', 'shared_library_file_id',
    'library_download_id', 'context_scopes', 'source_url', 'context_connector', 'connector_id', 'context_connector_info']) {
    const h = harness([{ items: [{ ...row(1), [key]: 'unresolved' }], cursor: null }]);
    assert.equal((await h.run()).ok, false, key);
    assert.equal(h.snapshots().at(-1).state, 'partial');
    assert.equal(h.exports.length, 0);
    assert.equal(h.calls.length, 1);
  }
});

test('gallery query parameters cannot change scope, credentials or download intent', async () => {
  for (const query of ['conversation_id=other', 'inline=false', 'download_intent=true', 'authorization=other',
    'library_file_id=other', 'shared_library_file_id=other', 'shared=unknown', 'source_url=https%3A%2F%2Fexample.com',
    'label=%', 'label=%00', 'variant=' + 'a'.repeat(1025)]) {
    const h = harness([{ items: [{ ...row(1), asset_pointer: 'file-service://file-1?' + query }], cursor: null }]);
    assert.equal((await h.run()).ok, false, query.slice(0, 40));
    assert.equal(h.calls.length, 1);
    assert.equal(h.exports.length, 0);
  }
});

test('gallery upgrade retires one older instance without stacking requests', () => {
  const fs = require('node:fs'), vm = require('node:vm');
  const source = fs.readFileSync(require.resolve('../android/app/src/main/assets/chatgpt_web_private_image_gallery.js'), 'utf8');
  let disposed = 0;
  const root = { location: { origin: 'https://chatgpt.com' },
    __elonChatGptPrivateImageGallery: { version: 2, dispose: () => disposed++ },
    __elonChatGptPrivateImagePointer: require('../android/app/src/main/assets/chatgpt_web_private_image_pointer.js') };
  vm.runInNewContext(source, { window: root });
  const instance = root.__elonChatGptPrivateImageGallery;
  assert.equal(instance.version, 5);
  assert.equal(disposed, 1);
  vm.runInNewContext(source, { window: root });
  assert.equal(root.__elonChatGptPrivateImageGallery, instance);
  assert.equal(disposed, 1);
});

test('account switch invalidates catalog, opaque handles and native cache claims', async () => {
  const h = harness();
  await h.run();
  const first = h.snapshots().at(-1).handles;
  h.switchAccount();
  await h.run('open', first);
  assert.equal(h.catalogCalls().length, 2);
  assert.notDeepEqual(h.snapshots().at(-1).handles, first);
  assert.equal(h.exports.length, 2);
});

test('late catalog after navigation or account change cannot populate native UI', async () => {
  for (const change of ['document', 'href', 'account']) {
    const pending = deferred();
    const h = harness([() => pending.promise]);
    const result = h.run();
    await Promise.resolve(); await Promise.resolve();
    if (change === 'document') h.root.__elonChatGptDocumentToken = 'doc_other';
    if (change === 'href') h.root.location.href = 'https://chatgpt.com/c/other';
    if (change === 'account') h.switchAccount();
    pending.resolve({ items: [row(1)], cursor: null });
    assert.equal((await result).ok, false);
    assert.equal(h.exports.length, 0);
    assert.equal(h.snapshots().some(e => e.handles), false);
    assert.equal(h.timers.size, 0);
  }
});

test('close cancels only the matching gallery operation and never sends a write', async () => {
  const pending = deferred(), h = harness([() => pending.promise]);
  const result = h.run();
  await Promise.resolve(); await Promise.resolve();
  h.api.cancel('mcp_old');
  assert.equal(h.calls[0].init.signal.aborted, false);
  h.api.cancel('mcp_gallery1');
  assert.equal(h.calls[0].init.signal.aborted, true);
  pending.resolve({ items: [row(1)], cursor: null });
  assert.equal((await result).ok, false);
  assert.equal(h.exports.length, 0);
  assert.equal(h.calls.every(c => c.init.method === 'GET'), true);
});

test('individual preview failure does not discard the page or successful previews', async () => {
  const h = harness([{ items: [row(1), row(2)], cursor: null }]);
  const original = h.root.__elonChatGptImageAssets.request;
  let count = 0;
  h.root.__elonChatGptImageAssets.request = (...args) => ++count === 1 ? Promise.resolve({ ok: false }) : original(...args);
  await h.run();
  assert.equal(h.snapshots().at(-1).state, 'partial');
  assert.equal(h.snapshots().at(-1).handles.length, 2);
  assert.equal(h.events.filter(e => e.type === 'image_asset').length, 1);
});

test('repeated cursor is rejected and cannot loop on the same page', async () => {
  const h = harness([{ items: [row(1)], cursor: '1' }, { items: [row(2)], cursor: '1' }]);
  await h.run();
  assert.equal((await h.run('next')).ok, false);
  assert.equal(h.snapshots().at(-1).state, 'failed');
});

test('identity unavailability does not fall through to DOM or a guessed unauthenticated request', async () => {
  const h = harness();
  h.root.__elonChatGptPrivateTransport.copySameOriginRequestHeaders = () => null;
  assert.equal((await h.run()).ok, false);
  assert.equal(h.calls.length, 0);
  assert.equal(h.snapshots().at(-1).state, 'failed');
});

test('catalog expiry revalidates without losing the selected cursor', async () => {
  const h = harness([{ items: [row(1)], cursor: '1' }, { items: [row(2)], cursor: null }]);
  await h.run();
  const originalNow = Date.now, future = originalNow() + 120001;
  try {
    Date.now = () => future;
    assert.equal((await h.run('next')).ok, true);
    assert.equal(h.catalogCalls().at(-1).url.searchParams.get('after'), '1');
    assert.equal(h.snapshots().at(-1).pageIndex, 1);
  } finally { Date.now = originalNow; }
});

test('bounded deadline and disposal cancel active reads without late events', async () => {
  for (const mode of ['timeout', 'dispose']) {
    const pending = deferred(), h = harness([() => pending.promise]);
    const result = h.run();
    await Promise.resolve(); await Promise.resolve();
    if (mode === 'timeout') [...h.timers.values()].find(t => t.ms === 35000).fn();
    else h.api.dispose();
    const count = h.events.length;
    assert.equal(h.calls[0].init.signal.aborted, true);
    pending.resolve({ items: [row(1)], cursor: null });
    assert.equal((await result).ok, false);
    assert.equal(h.events.length, count);
    assert.equal(h.timers.size, 0);
  }
});

test('same-version reinjection replaces a disposed gallery after background resume', async () => {
  const fs = require('node:fs'), path = require('node:path'), vm = require('node:vm');
  const source = fs.readFileSync(path.join(__dirname,
    '../android/app/src/main/assets/chatgpt_web_private_image_gallery.js'), 'utf8');
  const pending = deferred(), h = harness([() => pending.promise]);
  h.root.__elonChatGptPrivateImagePointer = require(
    '../android/app/src/main/assets/chatgpt_web_private_image_pointer.js');
  const context = vm.createContext({ window: h.root, URL, URLSearchParams });
  vm.runInContext(source, context);
  const before = h.root.__elonChatGptPrivateImageGallery;
  const command = { requestId: 'mcp_resume', value: JSON.stringify({ operation: 'open', cachedHandles: [] }) };
  const result = before.request(command, event => h.events.push(event));
  await Promise.resolve(); await Promise.resolve();
  before.dispose();
  const eventCount = h.events.length;
  pending.resolve({ items: [row(1)], cursor: null });
  assert.equal((await result).ok, false);
  assert.equal(h.events.length, eventCount, 'suspended callback cannot paint into the new UI');
  vm.runInContext(source, context);
  const after = h.root.__elonChatGptPrivateImageGallery;
  assert.notEqual(after, before, 'same module version must not reuse a disposed owner');
  assert.equal((await after.request(command, event => h.events.push(event))).ok, true);
  assert.equal(h.snapshots().at(-1).state, 'ready');
  vm.runInContext(source, context);
  assert.equal(h.root.__elonChatGptPrivateImageGallery, after, 'healthy reinjection retains the cache');
  assert.equal(h.timers.size, 0);
});

test('production integration keeps gallery receipts separate and removes the extra WebView path', () => {
  const fs = require('node:fs'), path = require('node:path');
  const base = path.join(__dirname, '../android/app/src/main');
  const read = (file, kotlin = false) => fs.readFileSync(path.join(base,
    kotlin ? 'kotlin/com/elon/app/chatgptweb' : 'assets', file), 'utf8');
  const adapter = read('chatgpt_web_adapter.js');
  assert.match(adapter, /PrivateImageGallery\?\.handle\(action, command, respond, emitEvent\)/);
  assert.match(adapter, /PrivateImageGallery\?\.dispose\(\)/);
  assert.match(read('chatgpt_web_adapter_bootstrap.js'), /'__elonChatGptPrivateImageGallery'/);
  assert.match(require('./chatgpt-web-adapter-assembly').readAdapterSource(), /"chatgpt_web_private_image_gallery.js"/);
  const controller = read('ChatGptWebImageGalleryController.kt', true);
  assert.doesNotMatch(controller, /ChatGptWebImageGallerySync\(/);
  assert.match(controller, /state.requestId != activeRequestId/);
  assert.match(controller, /asset.galleryRequestId == activeRequestId/);
  assert.match(controller, /pageSnapshot\?\.handles.orEmpty\(\).forEachIndexed/);
  assert.match(read('ChatGptWebImageSession.kt', true), /if \(asset.galleryRequestId == null\) assets.accept\(asset\)/);
});

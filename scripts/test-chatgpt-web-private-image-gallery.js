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

test('a valid terminal empty catalog is ready, malformed or unknown payload is not empty success', async () => {
  const empty = harness([{ items: [], cursor: null }]);
  assert.equal((await empty.run()).ok, true);
  assert.equal(empty.snapshots().at(-1).observedCount, 0);
  assert.equal(empty.snapshots().at(-1).state, 'ready');
  for (const payload of [{}, { items: null }, { items: [null] }, { items: [], cursor: 'next' },
    { items: [row(1)], cursor: 42 }, { items: Array(26).fill(row(1)) }]) {
    const h = harness([payload]);
    assert.equal((await h.run()).ok, false);
    assert.equal(h.snapshots().at(-1).state, 'failed');
    assert.equal(h.exports.length, 0);
  }
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

test('production integration keeps gallery receipts separate and removes the extra WebView path', () => {
  const fs = require('node:fs'), path = require('node:path');
  const base = path.join(__dirname, '../android/app/src/main');
  const read = (file, kotlin = false) => fs.readFileSync(path.join(base,
    kotlin ? 'kotlin/com/elon/app/chatgptweb' : 'assets', file), 'utf8');
  const adapter = read('chatgpt_web_adapter.js');
  assert.match(adapter, /PrivateImageGallery\?\.handle\(action, command, respond, emitEvent\)/);
  assert.match(adapter, /PrivateImageGallery\?\.dispose\(\)/);
  assert.match(read('chatgpt_web_adapter_bootstrap.js'), /'__elonChatGptPrivateImageGallery'/);
  assert.match(read('ChatGptWebPageAdapter.kt', true), /"chatgpt_web_private_image_gallery.js"/);
  const controller = read('ChatGptWebImageGalleryController.kt', true);
  assert.doesNotMatch(controller, /ChatGptWebImageGallerySync\(/);
  assert.match(controller, /state.requestId != activeRequestId/);
  assert.match(controller, /asset.galleryRequestId == activeRequestId/);
  assert.match(controller, /pageSnapshot\?\.handles.orEmpty\(\).forEachIndexed/);
  assert.match(read('ChatGptWebImageSession.kt', true), /if \(asset.galleryRequestId == null\) assets.accept\(asset\)/);
});

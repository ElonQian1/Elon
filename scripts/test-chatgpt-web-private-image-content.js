'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { create } = require('../android/app/src/main/assets/chatgpt_web_private_image_gallery.js');
const assets = path.join(__dirname, '../android/app/src/main/assets');
const contentPath = '/backend-api/estuary/content?id=file-fixture&sig=synthetic';
const origin = 'https://chatgpt.com';
const deferred = () => { let resolve; const promise = new Promise(r => { resolve = r; }); return { promise, resolve }; };

function harness(t, options = {}) {
  const calls = [], events = [], canvases = [], timers = new Set();
  const emit = event => { events.push(event); options.onEvent?.(event); };
  let closed = 0, account = 'synthetic-account-one', current = true, id = 0;
  const root = { location: { origin, href: origin + '/c/synthetic' },
    __elonChatGptDocumentToken: 'doc_synthetic', AbortController,
    setTimeout: (fn, ms) => { const timer = setTimeout(fn, ms); timers.add(timer); return timer; },
    clearTimeout: timer => { clearTimeout(timer); timers.delete(timer); },
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ authorization: 'Bearer ' + account }) },
    __elonChatGptPrivateJsonRequest: { request: async (_, url, init) => {
      calls.push({ kind: 'json', url, init });
      if (new URL(url).pathname.endsWith('/image_gen')) {
        return { payload: { items: [{ id: 'generation-fixture', asset_pointer: 'file-service://file-fixture',
          conversation_id: 'conversation-fixture', ...options.itemFields }], cursor: null } };
      }
      return { payload: { status: 'success', download_url: options.source || contentPath } };
    } },
  };
  const fetch = async (url, init) => {
    calls.push({ kind: 'bytes', url, init });
    if (options.fetch) return options.fetch(url, init);
    return { ok: true, url: options.responseUrl ?? new URL(url, origin).href,
      headers: { get: name => name === 'content-length' ? String(options.size || 128) : options.mime || 'image/png' },
      blob: async () => ({ type: options.mime || 'image/png', size: options.size || 128 }) };
  };
  const context = vm.createContext({ window: root, location: root.location, fetch, URL, URLSearchParams,
    AbortController, Blob, HTMLImageElement: class {},
    document: { createElement: () => {
      const canvas = { width: 0, height: 0, getContext: () => ({ fillRect() {}, drawImage() {} }),
        toDataURL: () => 'data:image/jpeg;base64,cHJldmlldw==' };
      canvases.push(canvas); return canvas;
    } },
    createImageBitmap: async () => ({ width: 2048, height: 1024, close: () => closed++ }),
  });
  for (const file of ['chatgpt_web_private_content_source.js', 'chatgpt_web_image_assets.js']) {
    vm.runInContext(fs.readFileSync(path.join(assets, file), 'utf8'), context);
  }
  const api = root.__elonChatGptImageAssets, gallery = create(root);
  t.after(() => { gallery.dispose(); api.dispose(); for (const timer of timers) clearTimeout(timer); });
  return { root, api, gallery, calls, events, canvases, timers, closed: () => closed,
    bytes: () => calls.filter(call => call.kind === 'bytes'),
    invalidate: () => { current = false; account = 'synthetic-account-two'; },
    request: (source = contentPath) => {
      const handle = api.registerPrivate('fixture-' + ++id, async () => source, () => current);
      return api.request(handle, event => events.push(event));
    },
    run: (cachedHandles = []) => gallery.request({ requestId: 'mcp_gallery' + ++id,
      value: JSON.stringify({ operation: 'open', cachedHandles }) }, emit),
    preview: handle => new Promise(resolve => {
      const handled = gallery.handle('request_image_asset', { value: handle },
        (_, ok, code) => resolve({ ok, code }), event => events.push(event));
      if (!handled) resolve({ ok: false, code: 'not_gallery_handle' });
    }),
  };
}

test('real private preview exports both authorized same-origin content routes', async t => {
  const h = harness(t);
  for (const route of ['/api/estuary/content', '/backend-api/estuary/content']) {
    for (const prefix of ['', origin, origin + ':443']) {
      assert.equal((await h.request(prefix + route + '?id=file-fixture&sig=synthetic')).ok, true);
      const call = h.bytes().at(-1);
      assert.equal(call.url, origin + route + '?id=file-fixture&sig=synthetic');
      assert.equal(call.init.credentials, 'include');
      assert.equal(call.init.redirect, 'error');
      assert.equal(call.init.headers, undefined);
      assert.equal(h.events.at(-1).width, 1024);
      assert.equal(h.events.at(-1).height, 512);
    }
  }
  assert.equal(h.closed(), 6);
  assert.equal(h.timers.size, 0);
  assert.ok(h.canvases.every(canvas => canvas.width === 0 && canvas.height === 0));
  assert.doesNotMatch(JSON.stringify(h.events), /estuary|sig=|synthetic|https:/);
});

test('gallery loads a separate thumbnail and resolves the full image only when selected', async t => {
  const thumbnail = '/backend-api/estuary/content?id=thumbnail-fixture';
  const h = harness(t, { itemFields: { url: contentPath, encodings: { thumbnail: { path: thumbnail } } } });
  assert.equal((await h.run()).ok, true);
  const page = h.events.filter(e => e.type === 'image_gallery_snapshot').at(-1);
  assert.notEqual(page.handles[0], page.previewHandles[0]);
  assert.deepEqual(h.bytes().map(c => c.url), [origin + thumbnail]);
  assert.equal((await h.preview(page.previewHandles[0])).ok, true, 'page completion does not expire its preview lease');
  assert.deepEqual(h.bytes().map(c => c.url), [origin + thumbnail, origin + contentPath]);
  const full = h.events.filter(e => e.type === 'image_asset').at(-1);
  assert.equal(full.handle, page.previewHandles[0]);
  assert.equal(full.requestId, page.requestId);
  assert.equal(full.width, 1024);
  assert.doesNotMatch(JSON.stringify(h.events), /https:|sig=|synthetic-account|file-fixture/);
  h.calls.length = 0;
  await h.run([page.handles[0]]);
  assert.equal(h.bytes().length, 0, 'thumbnail cache reopen does not fetch the full image');
  await h.run([page.previewHandles[0]]);
  assert.equal(h.bytes().length, 0, 'existing full preview cache can also serve the grid');
  const warm = h.events.filter(e => e.type === 'image_gallery_snapshot').at(-1);
  assert.deepEqual(warm.handles, warm.previewHandles);
});

test('unknown or invalid thumbnail sources retain the original full-image path', async t => {
  for (const thumbnail of [null, {}, { path: '' }, { path: 'https://untrusted.test/image' },
    { path: '/api/other' }, { path: '//chatgpt.com/api/estuary/content' }, { path: contentPath }]) {
    const h = harness(t, { itemFields: { url: contentPath, encodings: { thumbnail } } });
    assert.equal((await h.run()).ok, true);
    const page = h.events.filter(e => e.type === 'image_gallery_snapshot').at(-1);
    assert.deepEqual(page.handles, page.previewHandles);
    assert.deepEqual(h.bytes().map(c => c.url), [origin + contentPath]);
  }
});

test('partial thumbnail failure still permits its full preview without loading other full images', async t => {
  const thumbnail = '/backend-api/estuary/content?id=thumbnail-fixture';
  const h = harness(t, { itemFields: { url: contentPath, encodings: { thumbnail: { path: thumbnail } } },
    fetch: async url => url === origin + thumbnail ? { ok: false } : {
      ok: true, url, headers: { get: () => 'image/png' }, blob: async () => ({ type: 'image/png', size: 128 }),
    } });
  assert.equal((await h.run()).ok, false);
  const page = h.events.filter(e => e.type === 'image_gallery_snapshot').at(-1);
  assert.equal(page.state, 'partial');
  assert.equal(h.bytes().length, 1);
  assert.equal((await h.preview(page.previewHandles[0])).ok, true);
  assert.equal(h.bytes().length, 2);
});

test('expired on-demand full URL retries once and only exposes the final result', async t => {
  const h = harness(t, { source: '/backend-api/estuary/content?id=fresh',
    itemFields: { url: contentPath, encodings: { thumbnail: { path: '/api/estuary/content?id=small' } } },
    fetch: async url => url === origin + contentPath ? { ok: false } : {
      ok: true, url, headers: { get: () => 'image/png' }, blob: async () => ({ type: 'image/png', size: 128 }),
    } });
  await h.run();
  const page = h.events.filter(e => e.type === 'image_gallery_snapshot').at(-1);
  h.events.length = 0;
  assert.equal((await h.preview(page.previewHandles[0])).ok, true);
  assert.equal(h.calls.filter(c => c.kind === 'json').length, 2);
  assert.equal(h.events.filter(e => e.type === 'image_asset').length, 1);
  assert.equal(h.events[0].state, 'ready');
  assert.equal(h.timers.size, 0);
});

test('grid and selected full preview share the complete expired-URL recovery', async t => {
  for (const previewFirst of [false, true]) {
    const started = deferred(), response = deferred();
    let selected;
    const h = harness(t, { source: '/backend-api/estuary/content?id=fresh',
      itemFields: { url: contentPath },
      onEvent: event => {
        if (previewFirst && event.type === 'image_gallery_snapshot' && event.handles?.length && !selected) {
          selected = h.preview(event.previewHandles[0]);
        }
      },
      fetch: async url => {
        if (url === origin + contentPath) { started.resolve(); return response.promise; }
        return { ok: true, url, headers: { get: () => 'image/png' },
          blob: async () => ({ type: 'image/png', size: 128 }) };
      },
    });
    const batch = h.run();
    await started.promise;
    const page = h.events.filter(e => e.type === 'image_gallery_snapshot').at(-1);
    selected ??= h.preview(page.previewHandles[0]);
    response.resolve({ ok: false, status: 403 });
    assert.equal((await selected).ok, true, 'preview must wait for the complete recovery');
    assert.equal((await batch).ok, true);
    assert.deepEqual(h.calls.map(c => c.kind), ['json', 'bytes', 'json', 'bytes']);
    const imageEvents = h.events.filter(e => e.type === 'image_asset');
    assert.equal(imageEvents.length, 1, 'no premature failure or duplicate native image');
    assert.equal(imageEvents[0].state, 'ready');
    assert.equal(imageEvents[0].requestId, page.requestId);
    assert.equal(h.timers.size, 0);
  }
});

test('duplicate full previews coalesce and closing the gallery suppresses late results', async t => {
  for (const change of ['close', 'account', 'document', 'reopen']) {
    const started = deferred(), pending = deferred();
    const h = harness(t, {
      itemFields: { url: contentPath, encodings: { thumbnail: { path: '/api/estuary/content?id=small' } } },
      fetch: async url => {
        if (url === origin + contentPath) { started.resolve(); await pending.promise; }
        return { ok: true, url, headers: { get: () => 'image/png' },
          blob: async () => ({ type: 'image/png', size: 128 }) };
      },
    });
    await h.run();
    const page = h.events.filter(e => e.type === 'image_gallery_snapshot').at(-1);
    const first = h.preview(page.previewHandles[0]), second = h.preview(page.previewHandles[0]);
    await started.promise;
    assert.equal(h.bytes().filter(c => c.url === origin + contentPath).length, 1);
    if (change === 'close') h.gallery.cancel(page.requestId);
    if (change === 'account') h.invalidate();
    if (change === 'document') h.root.__elonChatGptDocumentToken = 'doc_other';
    if (change === 'reopen') await h.run(page.handles);
    const count = h.events.length;
    pending.resolve();
    assert.equal((await first).ok, false);
    assert.equal((await second).ok, false);
    assert.equal(h.events.length, count);
    assert.equal(h.timers.size, 0);
  }
});

test('private previews still reject arbitrary origins, paths, credentials and fragments', async t => {
  const h = harness(t);
  for (const source of [origin + '/backend-api/conversation', '/api/other',
    '//chatgpt.com/api/estuary/content', 'https://other.test/api/estuary/content',
    'https://user:pass@chatgpt.com/api/estuary/content', '/api/estuary/content#fragment']) {
    assert.equal((await h.request(source)).ok, false);
  }
  assert.equal(h.bytes().length, 0);
});

test('same-origin preview rejects final response URL changes or missing URL', async t => {
  for (const responseUrl of ['', 'https://external.test/image', origin + '/api/other',
    origin + '/backend-api/estuary/content?id=another']) {
    const h = harness(t, { responseUrl });
    assert.equal((await h.request()).ok, false);
    assert.equal(h.bytes().length, 1);
    assert.equal(h.closed(), 0);
    assert.equal(h.events.some(event => event.state === 'ready'), false);
  }
});

test('signed external previews still omit credentials and reject redirects', async t => {
  const h = harness(t);
  assert.equal((await h.request('https://files.oaiusercontent.com/image?sig=synthetic')).ok, true);
  assert.equal(h.bytes()[0].init.credentials, 'omit');
  assert.equal(h.bytes()[0].init.redirect, 'error');
});

test('same-origin private previews keep MIME and byte limits', async t => {
  for (const [options, error] of [[{ mime: 'text/html' }, 'not_image'],
    [{ size: 13 * 1024 * 1024 }, 'source_too_large']]) {
    const h = harness(t, options);
    assert.equal((await h.request()).error, error);
    assert.equal(h.closed(), 0);
    assert.equal(h.timers.size, 0);
  }
});

test('account change during same-origin image fetch prevents native publication', async t => {
  const response = deferred(), started = deferred();
  const h = harness(t, { fetch: async () => { started.resolve(); return response.promise; } });
  const pending = h.request();
  await Promise.race([started.promise, pending.then(result => assert.equal(result.ok, true))]);
  h.invalidate();
  response.resolve({ ok: true, url: origin + contentPath });
  assert.equal((await pending).ok, false);
  assert.equal(h.events.some(event => event.state === 'ready'), false);
  assert.equal(h.closed(), 0);
  assert.equal(h.timers.size, 0);
});

test('real gallery, resolver and exporter compose without DOM, and warm reopen makes zero requests', async t => {
  const h = harness(t);
  assert.equal((await h.run()).ok, true);
  assert.deepEqual(h.calls.map(call => call.kind), ['json', 'json', 'bytes']);
  assert.equal(new URL(h.calls[1].url).pathname, '/backend-api/files/download/file-fixture');
  const snapshot = h.events.filter(event => event.type === 'image_gallery_snapshot').at(-1);
  assert.equal(snapshot.state, 'ready');
  assert.equal(snapshot.handles.length, 1);
  assert.equal(h.events.filter(event => event.type === 'image_asset' && event.state === 'ready').length, 1);
  h.calls.length = 0;
  assert.equal((await h.run(snapshot.handles)).ok, true);
  assert.equal(h.calls.length, 0);
  assert.equal(h.root.location.href, origin + '/c/synthetic');
  assert.equal(h.timers.size, 0);
  assert.doesNotMatch(JSON.stringify(h.events), /Bearer|synthetic-account|file-fixture|conversation-fixture|sig=synthetic/);
});

test('catalog image URLs skip resolver reads while preserving preview size and cache identity', async t => {
  for (const url of [contentPath, 'https://files.oaiusercontent.com/image?sig=synthetic']) {
    const h = harness(t, { itemFields: { url, encodings: { thumbnail: { path: '/unverified-thumbnail' } } } });
    assert.equal((await h.run()).ok, true);
    assert.deepEqual(h.calls.map(call => call.kind), ['json', 'bytes']);
    assert.equal(h.bytes()[0].url, new URL(url, origin).href);
    assert.equal(h.events.find(event => event.type === 'image_asset' && event.state === 'ready').width, 1024);
    const handles = h.events.filter(event => event.type === 'image_gallery_snapshot').at(-1).handles;
    h.calls.length = 0;
    assert.equal((await h.run(handles)).ok, true);
    assert.equal(h.calls.length, 0);
    assert.doesNotMatch(JSON.stringify(h.events), /sig=|https:|Bearer|thumbnail/);
  }
});

test('expired catalog image URL uses one existing private resolver without duplicating images', async t => {
  const direct = 'https://files.oaiusercontent.com/image?sig=expired';
  const h = harness(t, { itemFields: { url: direct }, fetch: async url => url === direct
    ? { ok: false, status: 403 } : { ok: true, url, headers: { get: () => '' },
      blob: async () => ({ type: 'image/png', size: 128 }) } });
  assert.equal((await h.run()).ok, true);
  assert.deepEqual(h.calls.map(call => call.kind), ['json', 'bytes', 'json', 'bytes']);
  assert.equal(h.events.filter(event => event.type === 'image_asset' && event.state === 'ready').length, 1);
  assert.equal(h.events.filter(event => event.type === 'image_gallery_snapshot').at(-1).handles.length, 1);
  assert.equal(h.timers.size, 0);
});

test('invalid catalog URL stays on the private resolver and never fetches arbitrary targets', async t => {
  for (const url of ['https://untrusted.test/image', 'https://user:secret@files.oaiusercontent.com/image',
    'https://files.oaiusercontent.com:444/image', origin + '/backend-api/conversation',
    '//files.oaiusercontent.com/image', 'https://files.oaiusercontent.com/image#fragment']) {
    const h = harness(t, { itemFields: { url } });
    assert.equal((await h.run()).ok, true);
    assert.deepEqual(h.calls.map(call => call.kind), ['json', 'json', 'bytes']);
    assert.ok(h.bytes().every(call => call.url === origin + contentPath));
  }
});

test('catalog URL recovery is bounded and cancelled owners cannot start a resolver read', async t => {
  const direct = 'https://files.oaiusercontent.com/image?sig=expired';
  const failed = harness(t, { itemFields: { url: direct }, fetch: async () => ({ ok: false, status: 403 }) });
  assert.equal((await failed.run()).ok, false);
  assert.deepEqual(failed.calls.map(call => call.kind), ['json', 'bytes', 'json', 'bytes']);
  const response = deferred(), started = deferred();
  const h = harness(t, { itemFields: { url: direct }, fetch: async () => {
    started.resolve(); return response.promise;
  } });
  const pending = h.run();
  await started.promise;
  h.invalidate();
  response.resolve({ ok: false, status: 403 });
  assert.equal((await pending).ok, false);
  assert.deepEqual(h.calls.map(call => call.kind), ['json', 'bytes']);
  assert.equal(h.events.some(event => event.state === 'ready'), false);
});

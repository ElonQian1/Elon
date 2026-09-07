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
          conversation_id: 'conversation-fixture' }], cursor: null } };
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
      value: JSON.stringify({ operation: 'open', cachedHandles }) }, event => events.push(event)),
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

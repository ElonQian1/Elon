'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const source = fs.readFileSync(path.join(__dirname,
  '../android/app/src/main/assets/chatgpt_web_image_assets.js'), 'utf8');
const deferred = () => { let resolve; const promise = new Promise((r) => { resolve = r; }); return { promise, resolve }; };
const flush = async () => { for (let i = 0; i < 20; i++) await Promise.resolve(); };
const response = () => ({ ok: true, headers: { get: () => '128' },
  blob: async () => ({ type: 'image/png', size: 128 }) });

function harness(fetchImpl = async () => response()) {
  const timers = new Map();
  const requests = [];
  const canvases = [];
  let closedBitmaps = 0;
  let nextTimer = 0;
  class FakeImage {
    constructor(src) { this.currentSrc = src; this.naturalWidth = 512; this.naturalHeight = 512; }
    getAttribute(name) { return name === 'src' ? this.currentSrc : ''; }
  }
  const fetch = (...args) => { requests.push(args); return fetchImpl(...args); };
  const window = { fetch,
    setTimeout: (fn) => { timers.set(++nextTimer, fn); return nextTimer; },
    clearTimeout: (id) => timers.delete(id) };
  const document = { querySelectorAll: () => [], createElement: () => {
    const canvas = { width: 0, height: 0,
      getContext: () => ({ fillRect() {}, drawImage() {} }),
      toDataURL: () => 'data:image/jpeg;base64,cHJldmlldw==' };
    canvases.push(canvas);
    return canvas;
  } };
  vm.runInNewContext(source, { window, document, fetch,
    location: { origin: 'https://chatgpt.com' }, HTMLImageElement: FakeImage,
    URL, URLSearchParams, AbortController, Blob,
    createImageBitmap: async () => ({ width: 512, height: 512, close: () => closedBitmaps++ }) });
  return { api: window.__elonChatGptImageAssets, requests, timers, canvases,
    bitmapCloses: () => closedBitmaps,
    handle: (url) => window.__elonChatGptImageAssets.describe(new FakeImage(url)).assetHandle };
}

(async () => {
  const h = harness();
  const first = h.handle('https://chatgpt.com/image?id=one&token=old');
  assert.equal(first, h.handle('https://chatgpt.com/image?token=new&id=one'));
  assert.notEqual(first, h.handle('https://chatgpt.com/image?id=two&token=new'));

  const pending = deferred();
  const shared = harness(() => pending.promise);
  const handle = shared.handle('https://chatgpt.com/image?id=shared');
  const events = [];
  const observer = (event) => events.push(event);
  const a = shared.api.request(handle, observer);
  const b = shared.api.request(handle, observer);
  const other = [];
  const c = shared.api.request(handle, (event) => other.push(event));
  assert.equal(a, b);
  assert.equal(b, c);
  assert.equal(shared.requests.length, 1);
  pending.resolve(response());
  assert.equal((await a).ok, true);
  assert.equal(events.length, 1);
  assert.equal(other.length, 1);
  assert.equal(shared.bitmapCloses(), 1);
  assert.ok(shared.canvases.every((canvas) => canvas.width === 0 && canvas.height === 0));
  assert.equal(shared.timers.size, 0);

  const stalled = [];
  const pool = harness(() => { const request = deferred(); stalled.push(request); return request.promise; });
  const results = [];
  const poolEvents = [];
  for (let i = 0; i < 17; i++) {
    results.push(pool.api.request(pool.handle('https://chatgpt.com/image?id=' + i), (event) => poolEvents.push(event)));
  }
  assert.equal(pool.requests.length, 2, 'only two downloads may run at once');
  assert.equal((await results[16]).error, 'busy', 'the queue must be bounded');
  const timeout = pool.timers.values().next().value;
  timeout();
  assert.equal((await results[0]).error, 'timeout');
  assert.equal(pool.requests[0][1].signal.aborted, true);
  assert.equal(pool.requests.length, 3, 'a stalled download cannot block later images');
  stalled[0].resolve(response());
  await flush();
  assert.equal(poolEvents.filter((event) => event.handle === poolEvents[0].handle).length, 1,
    'late completion cannot overwrite a timeout');
  pool.api.dispose();
  for (const result of results.slice(1, 16)) assert.equal((await result).error, 'cancelled');
  assert.equal(pool.timers.size, 0);
  assert.equal(poolEvents.length, 1, 'disposed pages must not publish cancelled image events');

  let readerCancelled = false;
  const oversize = harness(async () => ({ ok: true,
    headers: { get: (name) => name === 'content-type' ? 'image/png' : '' },
    body: { getReader: () => ({ read: async () => ({ done: false, value: new Uint8Array(13 * 1024 * 1024) }),
      cancel: () => { readerCancelled = true; }, releaseLock() {} }) }
  }));
  const big = await oversize.api.request(oversize.handle('https://chatgpt.com/image?id=big'), () => {});
  assert.equal(big.error, 'source_too_large');
  assert.equal(readerCancelled, true);
  assert.equal(oversize.bitmapCloses(), 0);

  const bodyPending = deferred();
  const slowBody = harness(async () => ({ ...response(), blob: () => bodyPending.promise }));
  const bodyEvents = [];
  const slow = slowBody.api.request(slowBody.handle('https://chatgpt.com/image?id=body'), (event) => bodyEvents.push(event));
  await flush();
  slowBody.timers.values().next().value();
  assert.equal((await slow).error, 'timeout');
  bodyPending.resolve({ type: 'image/png', size: 128 });
  await flush();
  assert.equal(bodyEvents.length, 1);
  assert.equal(slowBody.bitmapCloses(), 0);
  console.log('IMAGE_REQUEST_LIFECYCLE_TESTS=passed');
})().catch((error) => { console.error(error); process.exitCode = 1; });

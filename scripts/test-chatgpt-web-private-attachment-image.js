'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const imageModule = require('../android/app/src/main/assets/chatgpt_web_private_attachment_image.js');

function fixture(width = 2400, height = 1600, type = 'image/png') {
  const state = { decoded: 0, closed: 0, drawn: [], encoded: [], canvases: [] };
  const file = new File(['synthetic image data'], 'fixture.' + type.split('/')[1], { type });
  const descriptor = { width, height, size: file.size, type };
  const root = {
    File, setTimeout, clearTimeout,
    createImageBitmap: async () => { state.decoded++; return { width, height, close: () => { state.closed++; } }; },
    OffscreenCanvas: class {
      constructor(w, h) { this.width = w; this.height = h; state.canvases.push(this); }
      getContext() { return { drawImage: (...args) => state.drawn.push(args.slice(1)) }; }
      async convertToBlob(options) { state.encoded.push(options); return new Blob(['encoded'], { type: options.type }); }
    },
  };
  return { root, state, file, descriptor, image: imageModule.create(root) };
}

test('normal native photos are prepared with official maximum edge and matching dimensions', async () => {
  for (const type of ['image/png', 'image/jpeg', 'image/webp']) {
    const f = fixture(2400, 1600, type), result = await f.image.prepare(f.file, f.descriptor);
    assert.equal(result.file.type, type);
    assert.equal(result.file.name, f.file.name);
    assert.deepEqual(result.dimensions, { width: 2048, height: 1365 });
    assert.deepEqual(f.state.drawn, [[0, 0, 2048, 1365]]);
    assert.equal(f.state.closed, 1);
    assert.equal(f.state.canvases[0].width, 1);
  }
});

test('small prepared images reuse the same bytes instead of re-encoding every photo', async () => {
  const f = fixture(320, 240), result = await f.image.prepare(f.file, f.descriptor);
  assert.equal(result.file, f.file);
  assert.deepEqual(result.dimensions, { width: 320, height: 240 });
  assert.equal(f.state.encoded.length, 0);
  assert.equal(f.state.closed, 1);
});

test('unknown metadata, oversized dimensions and unsupported images fail before decoding', async () => {
  for (const patch of [{ width: 0 }, { width: 3000, height: 2000 }, { height: undefined },
    { width: 20000, height: 1 }, { type: 'image/svg+xml' }, { type: 'image/gif' }, { size: 9 * 1024 * 1024 }]) {
    const f = fixture();
    await assert.rejects(f.image.prepare(f.file, { ...f.descriptor, ...patch }));
    assert.equal(f.state.decoded, 0);
  }
});

test('decoded dimensions must agree with native-normalized bytes', async () => {
  const f = fixture();
  await assert.rejects(f.image.prepare(f.file, { ...f.descriptor, width: 2399 }), /dimensions_mismatch/);
  assert.equal(f.state.closed, 1);
  assert.equal(f.state.encoded.length, 0);
});

test('cancellation or timeout settles before a stuck decoder and closes its late result', async () => {
  for (const mode of ['cancel', 'timeout']) {
    const f = fixture(), controller = new AbortController();
    let release;
    f.root.createImageBitmap = () => new Promise(resolve => { release = resolve; });
    if (mode === 'timeout') f.root.setTimeout = fn => setTimeout(fn, 1);
    const pending = f.image.prepare(f.file, f.descriptor, controller.signal);
    if (mode === 'cancel') controller.abort();
    await assert.rejects(pending, /cancelled|timeout/);
    release({ width: 2400, height: 1600, close: () => { f.state.closed++; } });
    await new Promise(resolve => setImmediate(resolve));
    assert.equal(f.state.closed, 1);
    assert.equal(f.state.encoded.length, 0);
  }
});

test('cancellation during encoding releases graphics and cannot return a prepared file', async () => {
  const f = fixture(), controller = new AbortController();
  let release;
  f.root.OffscreenCanvas.prototype.convertToBlob = () => new Promise(resolve => { release = resolve; });
  const pending = f.image.prepare(f.file, f.descriptor, controller.signal);
  await new Promise(resolve => setImmediate(resolve));
  controller.abort();
  await assert.rejects(pending, /cancelled/);
  assert.equal(f.state.closed, 1);
  assert.equal(f.state.canvases[0].width, 1);
  release(new Blob(['late'], { type: f.file.type }));
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(f.state.closed, 1);
});

test('encoder MIME mismatch, empty output and oversized bytes are not marked ready', async () => {
  for (const blob of [null, new Blob([], { type: 'image/png' }), new Blob(['x'], { type: 'image/jpeg' }),
    new Blob([Buffer.alloc(8 * 1024 * 1024 + 1)], { type: 'image/png' })]) {
    const f = fixture();
    f.root.OffscreenCanvas.prototype.convertToBlob = async () => blob;
    await assert.rejects(f.image.prepare(f.file, f.descriptor), /encoding_invalid/);
    assert.equal(f.state.closed, 1);
    assert.equal(f.state.canvases[0].height, 1);
  }
});

test('canvas encoding compatibility needs no mounted DOM or user input', async () => {
  const f = fixture();
  delete f.root.OffscreenCanvas;
  const canvas = { getContext: () => ({ drawImage() {} }), toBlob: (reply, type) => reply(new Blob(['x'], { type })) };
  f.root.document = { createElement: tag => { assert.equal(tag, 'canvas'); return canvas; } };
  const result = await f.image.prepare(f.file, f.descriptor);
  assert.equal(result.file.type, 'image/png');
  assert.equal(canvas.width, 1);
});

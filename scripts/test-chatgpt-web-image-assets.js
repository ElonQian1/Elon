'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const root = path.join(__dirname, '..');
const assetSource = fs.readFileSync(path.join(
  root, 'android', 'app', 'src', 'main', 'assets', 'chatgpt_web_image_assets.js'
), 'utf8');
const messagesSource = fs.readFileSync(path.join(
  root, 'android', 'app', 'src', 'main', 'assets', 'chatgpt_web_adapter_messages.js'
), 'utf8');
const pageAdapterSource = fs.readFileSync(path.join(
  root, 'android', 'app', 'src', 'main', 'kotlin', 'com', 'elon', 'app',
  'chatgptweb', 'ChatGptWebPageAdapter.kt'
), 'utf8');

class FakeImage {
  constructor(source, width = 1600, height = 900) {
    this.currentSrc = source;
    this.naturalWidth = width;
    this.naturalHeight = height;
    this.width = width;
    this.height = height;
  }
  getAttribute(name) { return name === 'src' ? this.currentSrc : ''; }
}

const emitted = [];
const canvas = {
  width: 0,
  height: 0,
  getContext: () => ({ fillRect() {}, drawImage() {}, set fillStyle(_) {} }),
  toDataURL: () => 'data:image/jpeg;base64,' + Buffer.from('bounded-preview').toString('base64')
};
const document = {
  createElement: (name) => {
    assert.equal(name, 'canvas');
    return canvas;
  },
  querySelectorAll: () => []
};
const fakeFetch = async () => ({
  ok: true,
  headers: { get: () => '128' },
  blob: async () => ({ type: 'image/png', size: 128 })
});
const window = {
  document,
  fetch: fakeFetch,
  setTimeout
};
window.window = window;

vm.runInNewContext(assetSource, {
  window,
  document,
  location: { origin: 'https://chatgpt.com' },
  HTMLImageElement: FakeImage,
  Image: FakeImage,
  fetch: fakeFetch,
  URL,
  Promise,
  Math,
  Number,
  Object,
  String,
  Map,
  Array,
  createImageBitmap: async () => ({ width: 1600, height: 900, close() {} })
}, { filename: 'chatgpt_web_image_assets.js' });

(async () => {
  const module = window.__elonChatGptImageAssets;
  assert.ok(module);
  const privateSource = 'https://files.oaiusercontent.com/generated/private.png?token=must-not-leave';
  const description = module.describe(new FakeImage(privateSource));
  assert.match(description.assetHandle, /^image_[a-f0-9]{16}$/);
  assert.equal(description.imageWidth, 1600);
  assert.doesNotMatch(JSON.stringify(description), /oaiusercontent|token|private\.png/);

  const outcome = await module.request(description.assetHandle, (event) => emitted.push(event));
  assert.equal(outcome.ok, true);
  assert.equal(emitted.length, 1);
  assert.equal(emitted[0].type, 'image_asset');
  assert.equal(emitted[0].state, 'ready');
  assert.equal(emitted[0].mediaType, 'image/jpeg');
  assert.ok(emitted[0].data.length > 0);
  assert.doesNotMatch(JSON.stringify(emitted), /oaiusercontent|token|private\.png/);
  assert.equal(canvas.width, 1024);
  assert.equal(canvas.height, 576);

  const missing = await module.request('image_0000000000000000', () => {});
  assert.equal(missing.ok, false);
  assert.equal(missing.error, 'unknown_handle');

  assert.match(messagesSource, /imageAssets\.describe\(node\)/);
  assert.ok(
    pageAdapterSource.indexOf('chatgpt_web_image_assets.js') <
      pageAdapterSource.indexOf('chatgpt_web_adapter_messages.js'),
    'image assets must register before message extraction'
  );
  assert.match(pageAdapterSource, /ADAPTER_VERSION = 247/);
  console.log('chatgpt web image asset tests passed');
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const golden = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'android', 'app', 'src',
  'test', 'resources', 'webchat', 'private-library-bridge.json'), 'utf8'));

function fixture() {
  const frames = [], requests = [];
  let response = { items: [{ id: 'libfile_synthetic', kind: 'file', name: 'fixture.txt',
    mime_type: 'text/plain', file_size_bytes: 7 }], cursor: null };
  class NodeElement {}
  class InputElement extends NodeElement {
    constructor() { super(); this.value = ''; }
    getAttribute() { return null; }
    getBoundingClientRect() { return { width: 100, height: 40 }; }
    closest() { return null; }
  }
  const composer = new InputElement();
  const document = { title: 'ChatGPT', documentElement: new NodeElement(), querySelector: () => null,
    querySelectorAll: selector => selector.includes('prompt-textarea') ? [composer] : [] };
  const window = {
    document, location: { origin: 'https://chatgpt.com', pathname: '/c/synthetic', href: 'https://chatgpt.com/c/synthetic' },
    elonChatGptNative: { postMessage: value => frames.push(JSON.parse(value)) },
    __elonChatGptAdapterVersion: golden.adapterVersion,
    __elonChatGptDocumentToken: golden.documentToken,
    __elonChatGptSnapshotScheduler: { create: () => ({ schedule() {}, dispose() {} }) },
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: 'Bearer synthetic-bridge-fixture' }) },
    __elonChatGptPrivateJsonRequest: { async request(_, url, init) {
      requests.push({ url, method: init.method });
      if (response instanceof Error) throw response;
      return { payload: response };
    } },
    crypto: { getRandomValues: array => array.fill(170) }, AbortController, setTimeout, clearTimeout,
    getComputedStyle: () => ({ display: 'block', visibility: 'visible' }),
    addEventListener() {}, removeEventListener() {},
    HTMLInputElement: InputElement, HTMLTextAreaElement: class extends InputElement {},
    InputEvent: class {}, Event: class {}, Node: NodeElement,
    MutationObserver: class { observe() {} disconnect() {} },
  };
  window.window = window;
  const context = vm.createContext({ ...window, window, URL });
  // Run the real command router and native serialization, not a mock emitEvent callback.
  for (const file of ['chatgpt_web_private_library_catalog.js',
    'chatgpt_web_adapter_conversation_directory_requests.js', 'chatgpt_web_adapter.js']) {
    vm.runInContext(fs.readFileSync(path.join(__dirname, '..', 'android', 'app', 'src', 'main', 'assets', file), 'utf8'),
      context, { filename: file });
  }
  return { frames, requests, response: value => { response = value; },
    async list(value = {}) {
      frames.length = 0;
      window.__elonChatGptBridge.command(JSON.stringify({ action: 'list_library_files',
        documentToken: golden.documentToken, requestId: golden.event.requestId, value: JSON.stringify(value) }));
      await new Promise(resolve => setImmediate(resolve));
      return frames;
    },
    dispose() { window.__elonChatGptBridge.dispose(); window.__elonChatGptPrivateLibraryCatalog.dispose(); },
  };
}

function snapshot(frames) {
  const frame = frames.find(row => row.schema === golden.schema && row.event?.type === 'library_files_snapshot');
  assert.ok(frame, 'a successful catalog read must reach the real native bridge as an object event');
  assert.ok(Number.isSafeInteger(frame.sequence) && frame.sequence > 0);
  assert.ok(Number.isFinite(Date.parse(frame.emittedAt)));
  const { sequence, emittedAt, ...stable } = frame;
  return stable;
}

function receipt(frames, detail, ok = true) {
  assert.deepEqual(frames.at(-1), { type: 'command_result', adapterVersion: golden.adapterVersion,
    documentToken: golden.documentToken, action: 'list_library_files', ok, detail, requestId: golden.event.requestId });
}

test('real adapter emits the shared Android library wire fixture before success, including cache hits', async t => {
  const f = fixture(); t.after(() => f.dispose());
  assert.deepEqual(snapshot(await f.list()), golden);
  receipt(f.frames, 'library_ready');
  assert.equal(f.requests.length, 1);
  assert.deepEqual(snapshot(await f.list()), golden);
  receipt(f.frames, 'library_cached');
  assert.equal(f.requests.length, 1, 'cache hits must still publish a native snapshot without another GET');
});

test('refresh failure preserves a typed cached snapshot and never publishes an empty success', async t => {
  const f = fixture(); t.after(() => f.dispose());
  await f.list();
  f.response(new Error('http_503'));
  const expected = structuredClone(golden);
  expected.event.stale = true;
  assert.deepEqual(snapshot(await f.list({ operation: 'refresh' })), expected);
  receipt(f.frames, 'library_read_failed', false);
});

test('a genuinely empty library remains distinguishable from a missing native event', async t => {
  const f = fixture(); t.after(() => f.dispose());
  f.response({ items: [], cursor: null });
  const expected = structuredClone(golden);
  expected.event.items = [];
  assert.deepEqual(snapshot(await f.list()), expected);
  receipt(f.frames, 'library_ready');
});

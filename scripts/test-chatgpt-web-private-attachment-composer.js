'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const composerModule = require('../android/app/src/main/assets/chatgpt_web_private_attachment_composer.js');
const sendModule = require('../android/app/src/main/assets/chatgpt_web_private_attachment_send.js');
const transportModule = require('../android/app/src/main/assets/chatgpt_web_private_attachment_transport.js');
const protocol = require('../android/app/src/main/assets/chatgpt_web_private_attachment_protocol.js');

test('production asset loader includes the dependency chain and the complete bundle parses', () => {
  const fs = require('node:fs'), path = require('node:path'), vm = require('node:vm');
  const adapter = fs.readFileSync(path.join(__dirname,
    '../android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt'), 'utf8');
  const declaration = adapter.slice(adapter.indexOf('private val ADAPTER_ASSETS = listOf('));
  const assets = [...declaration.slice(0, declaration.indexOf('\n        )')).matchAll(/"([^"\n]+\.js)"/g)].map(match => match[1]);
  const required = ['chatgpt_web_private_transport.js', 'chatgpt_web_private_attachment_protocol.js',
    'chatgpt_web_private_attachment_transport.js', 'chatgpt_web_native_attachment_source.js',
    'chatgpt_web_private_attachment_composer.js', 'chatgpt_web_private_attachment_send.js', 'chatgpt_web_adapter.js'];
  for (let index = 0; index < required.length; index++) {
    assert.equal(assets.filter(item => item === required[index]).length, 1);
    if (index) assert.ok(assets.indexOf(required[index - 1]) < assets.indexOf(required[index]));
  }
  new vm.Script(assets.map(name => fs.readFileSync(path.join(__dirname, '../android/app/src/main/assets', name), 'utf8')).join('\n'));
});

function fixture() {
  let values = [], account = 'Bearer synthetic-page-token', model = 'synthetic-model';
  const files$ = () => values;
  files$.set = next => { values = next; };
  const store = { files$, readyFiles$: () => values.filter(item => item.status === 'ready'),
    hasUploadInProgress$: () => values.some(item => item.status === 'uploading') };
  const fiber = { memoizedProps: { value: store }, dependencies: { firstContext: { memoizedValue: store } } };
  const input = { isConnected: true, __reactFiber$synthetic: fiber };
  const headers = () => ({ Authorization: account, 'chatgpt-account-id': 'synthetic-workspace' });
  const root = {
    location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/' },
    document: { querySelector: name => name === '#upload-files' ? input : {} },
    __elonChatGptDocumentToken: 'doc_synthetic_1',
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: headers, acquireSameOriginRequestHeaders: async () => headers() },
    __elonChatGptPrivateAttachmentTransport: transportModule,
    __elonChatGptComposer: { currentModel: () => model },
    AbortController, setTimeout, clearTimeout, setInterval, clearInterval,
  };
  const composer = composerModule.create(root);
  const file = new File(['synthetic bytes'], 'fixture.txt', { type: 'text/plain' });
  const descriptor = { version: 1, leaseId: '00000000-0000-4000-8000-000000000000',
    documentToken: root.__elonChatGptDocumentToken, href: root.location.href,
    name: file.name, size: file.size, type: file.type };
  const result = binding => ({ ok: true, stage: 'processed', associated: false, binding,
    fileId: 'file-synthetic', fileName: file.name, fileSize: file.size, mimeType: file.type, metadata: { fileTokenSize: 5 } });
  return { root, composer, store, input, fiber, file, descriptor, result,
    setAccount: next => { account = next; }, setModel: next => { model = next; } };
}

test('ready file association uses the official callable store and deduplicates native display', () => {
  const f = fixture(), binding = f.composer.capture();
  assert.deepEqual(f.composer.associate(binding, f.file, f.result(binding), f.descriptor.leaseId), { associated: true });
  assert.equal(f.store.readyFiles$().length, 1);
  assert.equal(f.store.readyFiles$()[0].fileSpec.id, 'file-synthetic');
  assert.equal(f.store.readyFiles$()[0].fileSpec.fileTokenSize, 5);
  const items = f.composer.merge([{ id: 'dom-1', name: 'fixture.txt', state: 'ready' }]);
  assert.equal(items.length, 1);
  assert.equal(items[0].removable, true);
  assert.equal(f.composer.remove(items[0].id), true);
  assert.equal(f.store.files$().length, 0);
});

test('context selection never guesses project, temporary, existing-thread or occupied composer contracts', () => {
  for (const path of ['/c/existing', '/g/g-p-example/project', '/?temporary-chat=true', '/?model=unknown']) {
    const f = fixture();
    f.root.location.href = 'https://chatgpt.com' + path;
    assert.equal(f.composer.available(), false);
  }
  const f = fixture();
  f.store.files$.set([{ tempId: 'user-owned', status: 'ready' }]);
  assert.equal(f.composer.available(), false);
});

test('ambiguous or missing official store is unknown, not fake readiness', () => {
  const f = fixture();
  f.fiber.return = { memoizedProps: { value: { ...f.store } } };
  assert.equal(f.composer.available(), false);
  f.fiber.return = null;
  f.input.isConnected = false;
  assert.equal(f.composer.available(), false);
});

test('document, route, account, model and store replacement each invalidate pending association', () => {
  for (const mutate of [
    f => { f.root.__elonChatGptDocumentToken = 'doc_replacement_2'; },
    f => { f.root.location.href += 'c/replacement'; },
    f => f.setAccount('Bearer replacement-page-token'),
    f => f.setModel('replacement-model'),
    f => { f.input.__reactFiber$synthetic = { memoizedProps: { value: { ...f.store } } }; },
  ]) {
    const f = fixture(), binding = f.composer.capture();
    mutate(f);
    assert.equal(f.composer.current(binding), false);
    assert.throws(() => f.composer.associate(binding, f.file, f.result(binding), f.descriptor.leaseId));
    assert.equal(f.store.files$().length, 0);
  }
});

test('processing completion is required and mismatched files cannot be associated', () => {
  for (const patch of [{ ok: false }, { associated: true }, { stage: 'uploading' },
    { fileName: 'wrong.txt' }, { fileId: '../bad' }, { fileSize: 999 }, { binding: {} }]) {
    const f = fixture(), binding = f.composer.capture();
    assert.throws(() => f.composer.associate(binding, f.file, { ...f.result(binding), ...patch }, f.descriptor.leaseId));
    assert.equal(f.store.files$().length, 0);
  }
});

test('association readback failure rolls back only the owned file, preserving concurrent user additions', () => {
  const f = fixture(), binding = f.composer.capture();
  const other = { tempId: 'user-added', status: 'ready' };
  f.store.readyFiles$ = () => {
    f.store.files$.set([...f.store.files$(), other]);
    return [];
  };
  assert.throws(() => f.composer.associate(binding, f.file, f.result(binding), f.descriptor.leaseId));
  assert.deepEqual(f.store.files$(), [other]);
});

test('a removed or consumed official file is not recreated by cached native readiness', () => {
  const f = fixture(), binding = f.composer.capture();
  f.composer.associate(binding, f.file, f.result(binding), f.descriptor.leaseId);
  f.store.files$.set([]);
  assert.deepEqual(f.composer.merge([]), []);
});

test('changing model after confirmed association does not hide the same conversation attachment', () => {
  const f = fixture(), binding = f.composer.capture();
  f.composer.associate(binding, f.file, f.result(binding), f.descriptor.leaseId);
  f.setModel('another-model');
  assert.equal(f.composer.merge([]).length, 1);
  f.root.location.href += 'c/elsewhere';
  assert.deepEqual(f.composer.merge([]), []);
});

function pipeline(f, overrides = {}) {
  const requests = [], receipts = [], changes = [];
  let fallbacks = 0;
  const send = sendModule.create(f.root, {
    composer: f.composer, source: { read: async () => f.file },
    createTransport: config => transportModule.create(f.root, {
      ...config, protocol,
      acquireHeaders: f.root.__elonChatGptPrivateTransport.acquireSameOriginRequestHeaders,
      request: async (_, url, init) => {
        requests.push({ url, method: init.method });
        if (overrides.request) return overrides.request(url, init);
        return url.endsWith('/files') ? { payload: { status: 'success', file_id: 'file-synthetic',
          upload_url: 'https://uploads.oaiusercontent.com/fixture?sig=synthetic' } } :
          url.endsWith('/process_upload_stream') ? { text: JSON.stringify({ file_id: 'file-synthetic',
            event: 'file.processing.completed', progress: 100 }) } : {};
      },
    }), ...overrides.options,
  });
  const start = () => send.start(JSON.stringify(f.descriptor), (...args) => receipts.push(args),
    value => changes.push(value), () => { fallbacks++; });
  return { send, start, requests, receipts, changes, fallbacks: () => fallbacks };
}

test('native handoff -> private POST/PUT/process -> official store -> native ready is one pipeline', async () => {
  const f = fixture(), p = pipeline(f);
  await p.start();
  assert.deepEqual(p.requests.map(item => item.method), ['POST', 'PUT', 'POST']);
  assert.equal(p.fallbacks(), 0);
  assert.deepEqual(p.receipts, [['request_attachment_upload', true, 'private_attachment_associated']]);
  assert.equal(p.changes.length, 1);
  assert.equal(f.composer.merge([])[0].state, 'ready');
  assert.equal(p.requests.some(item => item.url.includes('/conversation')), false, 'only the existing native send owner dispatches text');
});

test('compatibility path is selected before private writes for an unverified context', async () => {
  const f = fixture();
  f.root.location.href = 'https://chatgpt.com/g/g-p-fixture/project';
  const p = pipeline(f);
  await p.start();
  assert.equal(p.fallbacks(), 1);
  assert.equal(p.requests.length, 0);
});

test('failure after a private write never falls back or dispatches the prompt', async () => {
  const f = fixture(), p = pipeline(f, { request: async () => { throw new Error('timeout'); } });
  await p.start();
  assert.equal(p.requests.length, 1);
  assert.equal(p.fallbacks(), 0);
  assert.equal(p.receipts[0][1], false);
  assert.equal(f.store.files$().length, 0);
});

test('native byte read failure cannot silently switch to a file chooser', async () => {
  const f = fixture(), p = pipeline(f, { options: { source: { read: async () => { throw new Error('native_source_expired'); } } } });
  await p.start();
  assert.equal(p.requests.length, 0);
  assert.equal(p.fallbacks(), 0);
  assert.equal(p.receipts[0][1], false);
});

test('route change while create is in flight prevents PUT, association and write replay', async () => {
  const f = fixture(), p = pipeline(f, { request: async () => {
    f.root.location.href = 'https://chatgpt.com/c/switched';
    return { payload: { status: 'success', file_id: 'file-synthetic', upload_url: 'https://uploads.oaiusercontent.com/f?sig=s' } };
  } });
  await p.start();
  assert.equal(p.requests.length, 1);
  assert.equal(p.receipts[0][1], false);
  assert.equal(p.fallbacks(), 0);
  assert.equal(f.store.files$().length, 0);
});

test('concurrent starts do not create a second upload and cancellation prevents association', async () => {
  const f = fixture();
  let release;
  const p = pipeline(f, { options: { source: { read: () => new Promise(resolve => { release = resolve; }) } } });
  const first = p.start();
  await new Promise(resolve => setImmediate(resolve));
  await p.start();
  p.send.cancel();
  release(f.file);
  await first;
  assert.equal(p.requests.length, 0);
  assert.equal(p.receipts.length, 2);
  assert.equal(p.receipts.every(item => item[1] === false), true);
});

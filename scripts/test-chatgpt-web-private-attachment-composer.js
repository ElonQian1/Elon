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
    'chatgpt_web_private_attachment_transport.js', 'chatgpt_web_native_attachment_source.js', 'chatgpt_web_private_attachment_project.js',
    'chatgpt_web_private_attachment_composer.js', 'chatgpt_web_private_attachment_image.js',
    'chatgpt_web_private_attachment_send.js', 'chatgpt_web_adapter.js'];
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
    __elonChatGptPrivateAttachmentProtocol: protocol,
    __elonChatGptComposer: { currentModel: () => model },
    AbortController, setTimeout, clearTimeout, setInterval, clearInterval,
  };
  const composer = composerModule.create(root);
  const file = new File(['synthetic bytes'], 'fixture.txt', { type: 'text/plain' });
  const descriptor = { version: 1, leaseId: '00000000-0000-4000-8000-000000000000',
    documentToken: root.__elonChatGptDocumentToken, href: root.location.href,
    name: file.name, size: file.size, type: file.type };
  const result = binding => ({ ok: true, stage: 'processed', associated: false, binding,
    fileId: 'file-synthetic', fileName: file.name, fileSize: file.size, mimeType: file.type,
    isTemporaryChat: binding.isTemporaryChat, metadata: { fileTokenSize: 5 } });
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

test('context selection rejects unconfirmed scopes, invalid ids and occupied composers', () => {
  for (const path of ['/c/existing', '/g/g-p-example/project', '/?temporary-chat=false', '/?model=unknown',
    '/?temporary-chat=true&model=unknown', '/?temporary-chat=true&temporary-chat=false', '/?temporary-chat=true#other']) {
    const f = fixture();
    f.root.location.href = 'https://chatgpt.com' + path;
    assert.equal(f.composer.available(), false);
  }
  const f = fixture();
  f.store.files$.set([{ tempId: 'user-owned', status: 'ready' }]);
  assert.equal(f.composer.available(), false);
});

test('new temporary chat keeps both private upload and ready attachment out of the personal library', async () => {
  const f = fixture();
  f.root.location.href += '?temporary-chat=true';
  f.descriptor.href = f.root.location.href;
  const p = pipeline(f);
  await p.start();
  assert.equal(p.fallbacks(), 0);
  assert.equal(p.receipts[0][1], true);
  const create = JSON.parse(p.requests[0].init.body), process = JSON.parse(p.requests[2].init.body);
  assert.equal(create.store_in_library, false);
  assert.equal(Object.hasOwn(create, 'library_persistence_mode'), false);
  assert.equal(Object.hasOwn(process, 'library_persistence_mode'), false);
  assert.equal(process.metadata.is_temporary_chat, true);
  assert.equal(process.metadata.is_project_thread, false);
  assert.equal(f.store.readyFiles$()[0].isTemporaryChat, true);
  assert.equal(f.store.readyFiles$()[0].storeInLibrary, false);
  assert.equal(p.requests.length, 3, 'no text send or extra copy to library');
  assert.equal(f.composer.remove(f.composer.merge([])[0].id), true);
});

test('existing temporary chat requires matching server scope, never just the URL', async () => {
  for (const scope of [{ ordinary: false, temporary: true }, { ordinary: true, temporary: false },
    { ordinary: false, temporary: false }, { ordinary: false }]) {
    const f = existingFixture();
    f.root.location.href += '?temporary-chat=true';
    f.descriptor.href = f.root.location.href;
    f.root.__elonChatGptPrivateTransport.readAttachmentContext = async () => ({ conversationId: f.id, ...scope });
    const p = pipeline(f);
    await p.start();
    const supported = scope.temporary === true;
    assert.equal(p.requests.length, supported ? 3 : 0);
    assert.equal(p.fallbacks(), supported ? 0 : 1);
    assert.equal(f.store.readyFiles$().length, supported ? 1 : 0);
  }
});

test('temporary intent changes while uploading cannot attach, retry or persist the result', async () => {
  const f = fixture();
  f.root.location.href += '?temporary-chat=true';
  f.descriptor.href = f.root.location.href;
  const p = pipeline(f, { request: async (url, init) => {
    if (init.method === 'PUT') f.root.location.href = 'https://chatgpt.com/';
    return url.endsWith('/files') ? { payload: { status: 'success', file_id: 'file-synthetic',
      upload_url: 'https://uploads.oaiusercontent.com/fixture?sig=synthetic' } } : {};
  } });
  await p.start();
  assert.equal(p.fallbacks(), 0);
  assert.equal(p.requests.length, 2);
  assert.equal(p.receipts[0][1], false);
  assert.equal(f.store.files$().length, 0);
});

test('association rejects a receipt from a different persistence scope', () => {
  const f = fixture();
  f.root.location.href += '?temporary-chat=true';
  const binding = f.composer.capture();
  assert.throws(() => f.composer.associate(binding, f.file,
    { ...f.result(binding), isTemporaryChat: false }, f.descriptor.leaseId), /association_invalid/);
  assert.equal(f.store.files$().length, 0);
});

test('versioned reinjection cancels only the older owner and retains the current one', () => {
  const fs = require('node:fs'), path = require('node:path'), vm = require('node:vm');
  const source = fs.readFileSync(path.join(__dirname,
    '../android/app/src/main/assets/chatgpt_web_private_attachment_send.js'), 'utf8');
  let cancelled = 0;
  const root = { location: { origin: 'https://chatgpt.com' },
    __elonChatGptPrivateAttachmentSend: { version: 1, cancel: () => { cancelled++; } } };
  vm.runInNewContext(source, { window: root });
  const current = root.__elonChatGptPrivateAttachmentSend;
  assert.equal(current.version, 7);
  assert.equal(cancelled, 1);
  vm.runInNewContext(source, { window: root });
  assert.equal(root.__elonChatGptPrivateAttachmentSend, current);
  assert.equal(cancelled, 1);
});

function existingFixture() {
  const f = fixture();
  const id = '00000000-0000-4000-8000-000000000001';
  f.root.location.href = 'https://chatgpt.com/c/' + id;
  f.descriptor.href = f.root.location.href;
  f.root.__elonChatGptPrivateTransport.readAttachmentContext = async path => {
    assert.equal(path, '/c/' + id);
    return { conversationId: id, ordinary: true };
  };
  return { ...f, id };
}

function pdfFixture(temporary = false) {
  const f = fixture();
  if (temporary) f.root.location.href += '?temporary-chat=true';
  f.file = new File(['%PDF-1.7\nsynthetic fixture'], 'fixture.pdf', { type: 'application/pdf' });
  Object.assign(f.descriptor, { href: f.root.location.href, name: f.file.name, size: f.file.size, type: f.file.type });
  const props = { conversation: {}, onCreateNewCompletion() {}, currentModelId: 'synthetic-selected-model',
    currentModelConfig: { id: 'synthetic-default-model' } };
  const top = { stateNode: {} };
  top.stateNode.current = top;
  f.fiber.return = { memoizedProps: props, return: top };
  return { ...f, props };
}

test('PDF attachment uses the official composer model id, not the localized button label', async () => {
  for (const temporary of [false, true]) {
    for (const explicit of [false, true]) {
      const f = pdfFixture(temporary);
      if (!explicit) f.props.currentModelId = null;
      f.setModel('\u81ea\u52a8');
      const p = pipeline(f);
      await p.start();
      assert.equal(p.fallbacks(), 0);
      assert.equal(p.receipts[0][1], true);
      assert.equal(p.requests[0].init.headers['x-oai-model-slug'], explicit
        ? 'synthetic-selected-model' : 'synthetic-default-model');
      assert.equal(f.store.readyFiles$()[0].fileSpec.mimeType, 'application/pdf');
      assert.equal(f.store.readyFiles$()[0].isTemporaryChat, temporary);
    }
  }
});

test('missing, invalid or ambiguous PDF model binding stays unknown before reading bytes', async () => {
  for (const mutate of [f => { f.fiber.return = null; }, f => { f.props.currentModelId = '\u6781\u9ad8'; },
    f => { f.props.onCreateNewCompletion = null; }, f => { f.props.conversation = null; },
    f => { f.fiber.return.return.stateNode.current = {}; },
    f => { f.fiber.return.return = { memoizedProps: { ...f.props, currentModelId: 'other-model' },
      return: f.fiber.return.return }; }]) {
    const f = pdfFixture();
    mutate(f);
    const p = pipeline(f, { options: { source: { read: () => assert.fail('no PDF bytes before model binding') } } });
    await p.start();
    assert.equal(p.fallbacks(), 1);
    assert.equal(p.requests.length, 0);
    assert.equal(p.receipts.length, 0, 'missing runtime state is not an unsupported-PDF error');
  }
});

test('PDF model binding ignores stale React props and follows only a confirmed current alternate', async () => {
  const f = pdfFixture();
  const previous = f.fiber.return.return;
  const current = { stateNode: previous.stateNode };
  previous.stateNode.current = current;
  f.fiber.alternate = { return: { memoizedProps: { ...f.props, currentModelId: 'committed-model' }, return: current } };
  const p = pipeline(f);
  await p.start();
  assert.equal(p.receipts[0][1], true);
  assert.equal(p.requests[0].init.headers['x-oai-model-slug'], 'committed-model');
});

test('PDF native byte bridge, private transport and official ready store form one production pipeline', async () => {
  const f = pdfFixture(), bytes = Buffer.from(await f.file.arrayBuffer());
  f.root.File = File;
  f.root.atob = atob;
  const bridge = { onmessage: null, postMessage: raw => {
    const request = JSON.parse(raw);
    assert.equal(request.leaseId, f.descriptor.leaseId);
    assert.equal(request.documentToken, f.descriptor.documentToken);
    queueMicrotask(() => bridge.onmessage({ data: JSON.stringify({ requestId: request.requestId,
      offset: request.offset, data: bytes.subarray(request.offset, request.offset + 65536).toString('base64') }) }));
  } };
  f.root.elonChatGptAttachmentSource = bridge;
  const source = require('../android/app/src/main/assets/chatgpt_web_native_attachment_source.js').create(f.root);
  const p = pipeline(f, { options: { source } });
  await p.start();
  assert.equal(p.receipts[0][1], true);
  assert.equal(p.fallbacks(), 0);
  assert.equal(p.requests.length, 3);
  assert.deepEqual(Buffer.from(await p.requests[1].init.body.arrayBuffer()), bytes);
  assert.equal(f.store.readyFiles$()[0].fileSpec.mimeType, 'application/pdf');
  assert.equal(f.composer.merge([]).length, 1);
  assert.equal(bridge.onmessage, null);
});

test('actual PDF model changes cancel even when the displayed effort label stays unchanged', async () => {
  for (const afterWrite of [false, true]) {
    const f = pdfFixture();
    const p = pipeline(f, afterWrite ? { request: async () => {
      f.props.currentModelId = 'synthetic-other-model';
      return { payload: { status: 'success', file_id: 'file-synthetic',
        upload_url: 'https://uploads.oaiusercontent.com/fixture?sig=synthetic' } };
    } } : { options: { source: { read: async () => {
      f.props.currentModelId = 'synthetic-other-model';
      return f.file;
    } } } });
    await p.start();
    assert.equal(p.fallbacks(), 0);
    assert.equal(p.requests.length, afterWrite ? 1 : 0);
    assert.equal(p.receipts[0][1], false);
    assert.equal(f.store.readyFiles$().length, 0);
  }
});

test('existing ordinary conversation requires positive scope confirmation before association', async () => {
  const f = existingFixture(), binding = f.composer.capture();
  assert.equal(f.composer.available(), true);
  assert.throws(() => f.composer.associate(binding, f.file, f.result(binding), f.descriptor.leaseId));
  assert.equal(await f.composer.prepare(binding), true);
  assert.equal(f.composer.associate(binding, f.file, f.result(binding), f.descriptor.leaseId).associated, true);
  assert.equal(f.composer.merge([]).length, 1);
});

test('existing conversation reuses the private upload and exact official store without sending text', async () => {
  const f = existingFixture(), p = pipeline(f);
  await p.start();
  assert.deepEqual(p.requests.map(item => item.method), ['POST', 'PUT', 'POST']);
  assert.equal(p.fallbacks(), 0);
  assert.deepEqual(p.receipts, [['request_attachment_upload', true, 'private_attachment_associated']]);
  assert.equal(f.store.readyFiles$()[0].isProjectThread, false);
  assert.equal(f.store.readyFiles$()[0].isTemporaryChat, false);
});

test('production image preparation, private upload and store association preserve image type and dimensions', async () => {
  for (const temporary of [false, true]) {
    const f = existingFixture();
    if (temporary) f.root.location.href += '?temporary-chat=true';
    f.descriptor.href = f.root.location.href;
    f.root.__elonChatGptPrivateTransport.readAttachmentContext = async () =>
      ({ conversationId: f.id, ordinary: !temporary, temporary });
    f.file = new File(['synthetic PNG'], 'fixture.png', { type: 'image/png' });
    Object.assign(f.descriptor, { name: f.file.name, type: f.file.type, size: f.file.size, width: 320, height: 240 });
    f.root.File = File;
    let closed = 0;
    f.root.createImageBitmap = async () => ({ width: 320, height: 240, close: () => { closed++; } });
    f.root.__elonChatGptPrivateAttachmentImage = require('../android/app/src/main/assets/chatgpt_web_private_attachment_image.js');
    const p = pipeline(f);
    await p.start();
    assert.equal(p.receipts[0][1], true);
    assert.equal(p.fallbacks(), 0);
    assert.equal(p.requests.length, 3);
    const attachment = f.store.readyFiles$()[0];
    assert.equal(attachment.fileSpec.mimeType, 'image/png');
    assert.equal(attachment.fileSpec.width, 320);
    assert.equal(attachment.fileSpec.height, 240);
    assert.equal(attachment.file, f.file);
    assert.equal(attachment.isTemporaryChat, temporary);
    const body = JSON.parse(p.requests[2].init.body);
    assert.equal(body.use_case, 'multimodal');
    assert.equal(body.metadata.is_temporary_chat, temporary);
    assert.equal(closed, 1);
    assert.equal(f.composer.merge([])[0].name, 'fixture.png');
  }
});

test('unavailable image preparation selects compatibility before native byte reads', async () => {
  const f = fixture();
  f.descriptor.type = 'image/png';
  const p = pipeline(f, { options: { source: { read: () => assert.fail('must not read') } } });
  await p.start();
  assert.equal(p.fallbacks(), 1);
  assert.equal(p.requests.length, 0);
});

test('cancellation during image preparation never creates a private upload or starts a chooser', async () => {
  const f = fixture();
  f.file = new File(['x'], 'fixture.png', { type: 'image/png' });
  f.descriptor.type = f.file.type;
  let release;
  const p = pipeline(f, { options: { image: { available: () => true,
    prepare: () => new Promise(resolve => { release = resolve; }) } } });
  const pending = p.start();
  await new Promise(resolve => setImmediate(resolve));
  p.send.cancel();
  release({ file: f.file, dimensions: { width: 1, height: 1 } });
  await pending;
  assert.equal(p.requests.length, 0);
  assert.equal(p.fallbacks(), 0);
  assert.equal(p.receipts[0][1], false);
});

test('production reader integrates ordinary and temporary attachments without a substitute scope resolver', async () => {
  for (const temporary of [false, true]) {
    const f = existingFixture(), requests = [];
    if (temporary) {
      f.root.location.href += '?temporary-chat=true';
      f.descriptor.href = f.root.location.href;
    }
    const headers = f.root.__elonChatGptPrivateTransport.copySameOriginRequestHeaders;
    delete f.root.__elonChatGptPrivateTransport;
    f.root.__elonChatGptPrivateConversationPrefetchEnabled = true;
    f.root.__elonChatGptPrivateTransportPolicy = require('../android/app/src/main/assets/chatgpt_web_private_transport_policy.js');
    f.root.__elonChatGptPrivateJsonRequest = require('../android/app/src/main/assets/chatgpt_web_private_json_request.js');
    f.root.__elonChatGptPrivateAuthContext = { canAcquire: () => true, copyRequestHeaders: headers };
    f.root.location.pathname = '/c/' + f.id;
    f.root.fetch = async (url, init) => {
      requests.push({ url, init });
      return { ok: true, status: 200, text: async () => JSON.stringify({ conversation_id: f.id,
        is_do_not_remember: temporary, gizmo_id: null, mapping: {} }) };
    };
    const fs = require('node:fs'), path = require('node:path');
    require('node:vm').runInNewContext(fs.readFileSync(path.join(__dirname,
      '../android/app/src/main/assets/chatgpt_web_private_transport.js'), 'utf8'),
    { window: f.root, location: f.root.location, URL });
    const p = pipeline(f);
    await p.start();
    assert.equal(requests.length, 1);
    assert.equal(requests[0].url, '/backend-api/conversations/' + f.id);
    assert.equal(requests[0].init.method, 'GET');
    assert.equal(p.requests.length, 3);
    assert.equal(p.receipts[0][1], true);
    assert.equal(f.composer.merge([])[0].state, 'ready');
    assert.equal(f.store.readyFiles$()[0].isTemporaryChat, temporary);
  }
});

test('known project or temporary metadata selects compatibility before reading bytes or writing', async () => {
  const f = existingFixture();
  f.root.__elonChatGptPrivateTransport.readAttachmentContext = async () => ({ conversationId: f.id, ordinary: false });
  const p = pipeline(f, { options: { source: { read: () => assert.fail('must not read bytes') } } });
  await p.start();
  assert.equal(p.fallbacks(), 1);
  assert.equal(p.requests.length, 0);
  assert.equal(p.receipts.length, 0);
});

test('unknown context retains the existing path before any private bytes or writes', async () => {
  for (const read of [undefined, async () => null, async () => ({}),
    async () => ({ conversationId: 'wrong', ordinary: true }),
    async () => { throw new Error('http_503'); }]) {
    const f = existingFixture();
    f.root.__elonChatGptPrivateTransport.readAttachmentContext = read;
    const p = pipeline(f);
    assert.equal(await f.composer.prepare(f.composer.capture()), null, 'unknown is distinct from confirmed unsupported');
    await p.start();
    assert.equal(p.fallbacks(), 1);
    assert.equal(p.requests.length, 0);
    assert.equal(p.receipts.length, 0);
  }
});

test('context changes while metadata is pending prevent upload and compatibility replay', async () => {
  for (const mutate of [f => { f.root.location.href += '?changed'; },
    f => { f.root.__elonChatGptDocumentToken = 'doc_replacement_2'; },
    f => f.setAccount('Bearer switched-account-token'), f => f.setModel('new-model'),
    f => { f.input.__reactFiber$synthetic = { memoizedProps: { value: { ...f.store } } }; },
    f => { f.store.files$.set([{ status: 'ready', tempId: 'user-added' }]); }]) {
    const f = existingFixture();
    let release;
    f.root.__elonChatGptPrivateTransport.readAttachmentContext = () => new Promise(resolve => { release = resolve; });
    const p = pipeline(f), pending = p.start();
    await new Promise(resolve => setImmediate(resolve));
    mutate(f);
    release({ conversationId: f.id, ordinary: true });
    await pending;
    assert.equal(p.requests.length, 0);
    assert.equal(p.fallbacks(), 0);
    assert.equal(p.receipts[0][1], false);
  }
});

test('cancel and timeout release pending metadata ownership without waiting for its late response', async () => {
  for (const mode of ['cancel', 'timeout']) {
    const f = existingFixture();
    let release;
    f.root.__elonChatGptPrivateTransport.readAttachmentContext = () => new Promise(resolve => { release = resolve; });
    if (mode === 'timeout') f.root.setTimeout = (fn, ms) => setTimeout(fn, ms === 10000 ? 1 : ms);
    const p = pipeline(f), pending = p.start();
    await new Promise(resolve => setImmediate(resolve));
    if (mode === 'cancel') p.send.cancel();
    await pending;
    release({ conversationId: f.id, ordinary: true });
    await new Promise(resolve => setImmediate(resolve));
    assert.equal(p.requests.length, 0);
    assert.equal(p.fallbacks(), mode === 'timeout' ? 1 : 0);
    assert.equal(p.receipts.length, mode === 'timeout' ? 0 : 1);
    if (mode === 'cancel') assert.equal(p.receipts[0][1], false);
    f.root.__elonChatGptPrivateTransport.readAttachmentContext = async () => ({ conversationId: f.id, ordinary: true });
    await p.start();
    assert.equal(p.receipts.at(-1)[1], true, 'next explicit upload is not stuck behind the old read');
  }
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
        requests.push({ url, method: init.method, init });
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

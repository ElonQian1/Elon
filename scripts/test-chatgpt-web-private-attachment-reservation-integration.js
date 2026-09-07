'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const fs = require('node:fs'), path = require('node:path'), vm = require('node:vm');
const base = '../android/app/src/main/assets/';
const reservation = require(base + 'chatgpt_web_private_attachment_reservation.js');
const protocol = require(base + 'chatgpt_web_private_attachment_protocol.js');
const bytes = require(base + 'chatgpt_web_private_attachment_bytes.js');
const transport = require(base + 'chatgpt_web_private_attachment_transport.js');
const composerModule = require(base + 'chatgpt_web_private_attachment_composer.js');
const send = require(base + 'chatgpt_web_private_attachment_send.js');
const selection = require(base + 'chatgpt_web_private_attachment_selection.js');
const request = require(base + 'chatgpt_web_private_json_request.js');
const tick = () => new Promise(resolve => setImmediate(resolve));
const file = () => new File(['synthetic integration fixture'], 'fixture.txt', { type: 'text/plain' });
const blobUrl = 'https://uploads.oaiusercontent.com/fixture?sig=synthetic';
const completed = (id, mime) => JSON.stringify({ file_id: id, event: 'file.processing.completed', progress: 100,
  extra: { total_tokens: 4, mime_type: mime } });

function fixture(options = {}) {
  let values = [], account = 'Bearer synthetic-account', model = 'synthetic-model';
  const requests = [], receipts = [], events = [];
  const files$ = () => values;
  files$.set = next => { values = next; events.push('associated'); };
  const store = { files$, readyFiles$: () => values.filter(value => value.status === 'ready'),
    hasUploadInProgress$: () => false };
  const top = { stateNode: {}, memoizedProps: { conversation: {}, onCreateNewCompletion() {},
    currentModelId: 'synthetic-model' } };
  top.stateNode.current = top;
  const input = { isConnected: true, __reactFiber$test: { memoizedProps: { value: store }, return: top } };
  const selectedFile = options.file || file();
  const root = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/' + (options.temporary ? '?temporary-chat=true' : '') },
    document: { querySelector: selector => selector === '#upload-files' ? input : null },
    performance: { getEntriesByName: () => [{}], now: () => performance.now() },
    AbortController, FormData, setTimeout, clearTimeout, setInterval, clearInterval,
    __elonChatGptDocumentToken: 'doc_fixture_1',
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ authorization: account }),
      acquireSameOriginRequestHeaders: async () => ({ authorization: account, 'chatgpt-account-id': 'workspace-synthetic',
        Cookie: 'never-forward', 'x-oai-model-slug': 'stale-model', 'openai-sentinel-proof-token': 'never-forward' }) },
    __elonChatGptComposer: { currentModel: () => model },
    __elonChatGptPrivateAttachmentProtocol: protocol, __elonChatGptPrivateAttachmentTransport: transport,
    __elonChatGptPrivateAttachmentSelection: selection,
    __elonChatGptPrivateAttachmentBytes: bytes, __elonChatGptPrivateJsonRequest: request,
    __elonChatGptPrivateAttachmentReservation: { version: reservation.version, create: (host, config) => reservation.create(host, {
      ...config, loadRuntime: async () => ({ t6: () => ({ loadingStatus: 'Ready',
        getExperiment: () => ({ name: '3119290944', groupName: 'treatment', details: { reason: 'Network:Recognized' },
          get: () => options.enabled !== false }) }) }),
    }) },
  };
  root.fetch = async (url, init) => {
    requests.push({ url, init });
    events.push(url.endsWith('/upload_reservations') ? 'reserve' : url.endsWith('/claim_and_finish') ? 'claim' :
      url.endsWith('/files') ? 'create' : url.includes('oaiusercontent.com') ? 'bytes' : 'process');
    const overridden = await options.respond?.(url, init, root);
    if (overridden) return overridden;
    if (url.endsWith('/upload_reservations')) return Response.json({ eligible: true,
      reservation_id: 'reservation-synthetic', upload_url: options.estuary ? '/backend-api/estuary/upload_content_bytes' : blobUrl,
      upload_url_expires_at: new Date(Date.now() + 120000).toISOString(),
      reservation_expires_at: new Date(Date.now() + 180000).toISOString() });
    if (url.endsWith('/files')) return Response.json({ status: 'success', file_id: 'file-legacy', upload_url: blobUrl });
    if (url.endsWith('/claim_and_finish')) return new Response(completed('reservation-synthetic', selectedFile.type));
    if (url.endsWith('/process_upload_stream')) return new Response(completed('file-legacy', selectedFile.type));
    return new Response(null, { status: 201 });
  };
  const composer = composerModule.create(root);
  let fallbacks = 0;
  const instance = send.create(root, { composer,
    source: { read: async (_, signal) => {
      events.push('read');
      if (options.read) await options.read(root, signal);
      else await tick();
      events.push('read_done');
      return selectedFile;
    } },
    image: { available: () => true, prepare: async value => {
      events.push('image_prepare'); await tick(); return { file: value, dimensions: { width: 30, height: 20 } };
    } } });
  const descriptor = { name: selectedFile.name, size: selectedFile.size, type: selectedFile.type,
    documentToken: root.__elonChatGptDocumentToken, href: root.location.href,
    leaseId: '00000000-0000-4000-8000-000000000000' };
  return { root, instance, store, requests, receipts, events, composer, descriptor,
    begin: (kind = 'document', id = 'selection_' + 'a'.repeat(32)) => {
      descriptor.selectionId = id;
      return instance.beginSelection(JSON.stringify({ id, kind, documentToken: descriptor.documentToken, href: descriptor.href }));
    },
    setAccount: value => { account = value; }, setModel: value => { model = value; },
    start: () => instance.start(JSON.stringify(descriptor), (...value) => receipts.push(value),
      () => events.push('changed'), () => { fallbacks++; }), fallbacks: () => fallbacks };
}

test('production asset bundle loads reservation before transport and retains one sender', () => {
  const adapter = fs.readFileSync(path.join(__dirname,
    '../android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt'), 'utf8');
  const declaration = adapter.slice(adapter.indexOf('private val ADAPTER_ASSETS = listOf('));
  const assets = [...declaration.slice(0, declaration.indexOf('\n        )')).matchAll(/"([^"\n]+\.js)"/g)].map(match => match[1]);
  const name = 'chatgpt_web_private_attachment_reservation.js';
  assert.equal(assets.filter(value => value === name).length, 1);
  assert.ok(assets.indexOf('chatgpt_web_private_attachment_bytes.js') < assets.indexOf(name));
  assert.ok(assets.indexOf(name) < assets.indexOf('chatgpt_web_private_attachment_transport.js'));
  assert.ok(assets.indexOf('chatgpt_web_private_attachment_selection.js') < assets.indexOf('chatgpt_web_private_attachment_send.js'));
  new vm.Script(assets.map(value => fs.readFileSync(path.join(__dirname, base, value), 'utf8')).join('\n'));
});

test('native attachment preparation overlaps reservation and final claim associates exactly once', async () => {
  const f = fixture(); await f.start();
  assert.deepEqual(f.receipts, [['request_attachment_upload', true, 'private_attachment_associated']]);
  assert.ok(f.events.indexOf('reserve') < f.events.indexOf('read_done'));
  assert.deepEqual(f.events.filter(value => ['reserve', 'bytes', 'claim', 'associated'].includes(value)),
    ['reserve', 'bytes', 'claim', 'associated']);
  assert.equal(f.requests.length, 3);
  assert.equal(f.requests.some(value => value.url.endsWith('/files') || value.url.endsWith('/process_upload_stream')), false);
  assert.equal(f.store.files$().length, 1);
  assert.equal(f.store.files$()[0].fileId, 'reservation-synthetic');
  assert.equal(f.fallbacks(), 0);
  const blob = f.requests[1];
  assert.equal(blob.init.credentials, 'omit');
  assert.equal(blob.init.headers.authorization, undefined);
  assert.equal(await blob.init.body.text(), await file().text());
  for (const call of [f.requests[0], f.requests[2]]) {
    assert.equal(call.init.headers.authorization, 'Bearer synthetic-account');
    assert.equal(call.init.headers.Cookie, undefined);
    assert.equal(call.init.headers['openai-sentinel-proof-token'], undefined);
    assert.equal(call.init.redirect, 'error');
  }
});

test('pending or failed reservations cannot hold up the existing private create route', async () => {
  for (const mode of ['pending', 'failed', 'ineligible', 'disabled']) {
    let late;
    const f = fixture({ enabled: mode !== 'disabled', respond: async url => {
      if (!url.endsWith('/upload_reservations')) return;
      if (mode === 'pending') return new Promise(resolve => { late = resolve; });
      return mode === 'failed' ? new Response('synthetic failure', { status: 503 }) : Response.json({ eligible: false });
    } });
    await f.start();
    assert.equal(f.receipts[0][1], true);
    assert.equal(f.store.files$()[0].fileId, 'file-legacy');
    assert.equal(f.requests.filter(value => value.url.endsWith('/files')).length, 1);
    assert.equal(f.requests.some(value => value.url.endsWith('/claim_and_finish')), false);
    late?.(Response.json({ eligible: true })); await tick();
    assert.equal(f.store.files$().length, 1);
    assert.equal(f.fallbacks(), 0);
  }
});

test('PDF model and normalized image geometry accompany final claim, never blob credentials', async () => {
  for (const type of ['application/pdf', 'image/png']) {
    const f = fixture({ file: new File(['synthetic bytes'], type === 'application/pdf' ? 'fixture.pdf' : 'image.png', { type }) });
    await f.start();
    assert.equal(f.receipts[0][1], true);
    const claim = f.requests.find(value => value.url.endsWith('/claim_and_finish'));
    assert.ok(claim);
    const body = JSON.parse(claim.init.body);
    assert.equal(body.mime_type, type);
    assert.equal(f.requests[1].init.headers['x-oai-model-slug'], undefined);
    if (type === 'application/pdf') assert.equal(claim.init.headers['x-oai-model-slug'], 'synthetic-model');
    else {
      assert.equal(body.width, 30); assert.equal(body.height, 20);
      assert.equal(body.use_case, 'multimodal');
      assert.equal(f.store.files$()[0].fileSpec.width, 30);
    }
  }
});

test('same-origin reserved byte upload uses FormData and still requires final processing', async () => {
  const f = fixture({ estuary: true }); await f.start();
  assert.equal(f.receipts[0][1], true);
  const uploaded = f.requests[1];
  assert.equal(uploaded.url, 'https://chatgpt.com/backend-api/estuary/upload_content_bytes');
  assert.equal(uploaded.init.method, 'POST');
  assert.equal(uploaded.init.body.get('upload_url'), '/backend-api/estuary/upload_content_bytes');
  assert.equal(await uploaded.init.body.get('file').text(), await file().text());
  assert.equal(uploaded.init.headers['Content-Type'], undefined);
  assert.equal(uploaded.init.credentials, 'include');
  assert.equal(f.requests[2].url.endsWith('/claim_and_finish'), true);
});

test('temporary upload keeps its legacy privacy contract when the official experiment is disabled', async () => {
  const f = fixture({ temporary: true, enabled: false }); await f.start();
  assert.equal(f.receipts[0][1], true);
  assert.equal(f.events.includes('reserve'), false);
  const body = JSON.parse(f.requests[2].init.body);
  assert.equal(body.metadata.is_temporary_chat, true);
  assert.equal(body.metadata.store_in_library, false);
  assert.equal(Object.hasOwn(body, 'library_persistence_mode'), false);
});

test('byte failure, incomplete claim or a mismatched file never attaches or replays an upload', async () => {
  for (const mode of ['bytes', 'claim_error', 'not_terminal', 'wrong_id']) {
    const f = fixture({ respond: async (url, init) => {
      if (mode === 'bytes' && init.method === 'PUT') return new Response('', { status: 500 });
      if (!url.endsWith('/claim_and_finish')) return;
      if (mode === 'claim_error') return new Response('', { status: 500 });
      if (mode === 'not_terminal') return new Response(JSON.stringify({ file_id: 'reservation-synthetic',
        event: 'file.processing.file_ready', progress: 100 }));
      if (mode === 'wrong_id') return new Response(completed('file-other', 'text/plain'));
    } });
    await f.start();
    assert.equal(f.receipts[0][1], false);
    assert.equal(f.store.files$().length, 0);
    assert.equal(f.requests.filter(value => value.url.endsWith('/files')).length, 0);
    assert.equal(f.requests.filter(value => value.init.method === 'PUT').length, 1);
    assert.equal(f.requests.filter(value => value.url.endsWith('/claim_and_finish')).length, mode === 'bytes' ? 0 : 1);
    assert.equal(f.fallbacks(), 0, 'failure after bytes is not permission to replay via another route');
  }
});

test('cancellation and account/document/model changes before byte read completes cannot dispatch user data', async () => {
  for (const mode of ['cancel', 'account', 'document', 'model']) {
    let release;
    const f = fixture({ read: () => new Promise(resolve => { release = resolve; }) });
    const pending = f.start(); await tick();
    assert.equal(f.events.includes('reserve'), true);
    if (mode === 'cancel') f.instance.cancel();
    if (mode === 'account') f.setAccount('Bearer different-account');
    if (mode === 'document') f.root.__elonChatGptDocumentToken = 'doc_other_2';
    if (mode === 'model') f.setModel('different-model');
    release(); await pending;
    assert.equal(f.receipts[0][1], false);
    assert.equal(f.requests.length, 1);
    assert.equal(f.store.files$().length, 0);
    assert.equal(f.fallbacks(), 0);
  }
});

test('reservation intent excludes unconfirmed and hidden project membership, not just project URLs', async () => {
  const f = fixture();
  const id = '00000000-0000-4000-8000-000000000001';
  f.root.location.href = 'https://chatgpt.com/c/' + id;
  const binding = f.composer.capture();
  assert.equal(f.composer.reservationContext(binding, f.descriptor), null);
  f.root.__elonChatGptPrivateTransport.readAttachmentContext = async () => ({ conversationId: id,
    projectId: 'g-p-' + 'a'.repeat(32) });
  assert.equal(await f.composer.prepare(binding, new AbortController().signal, f.descriptor), null);
  assert.equal(f.composer.reservationContext(binding, f.descriptor), null);
  f.root.__elonChatGptPrivateTransport.readAttachmentContext = async () => ({ conversationId: id, ordinary: true });
  assert.equal(await f.composer.prepare(binding, new AbortController().signal, f.descriptor), true);
  assert.equal(f.composer.reservationContext(binding, f.descriptor).useCase, 'ace_upload');
});

test('picker-open prewarm precedes all bytes, survives host suspension and is consumed once', async t => {
  for (const type of ['text/plain', 'image/png']) {
    const f = fixture({ file: new File(['synthetic bytes'], type === 'text/plain' ? 'fixture.txt' : 'image.png', { type }) });
    t.after(() => f.instance.cancel());
    assert.equal(f.begin(type === 'text/plain' ? 'document' : 'image'), true);
    await tick();
    assert.deepEqual(f.events, ['reserve']);
    f.instance.suspend();
    f.root.__elonChatGptPrivateAttachmentSend = f.instance;
    vm.runInNewContext(fs.readFileSync(path.join(__dirname, base, 'chatgpt_web_private_attachment_send.js'), 'utf8'), { window: f.root });
    assert.equal(f.root.__elonChatGptPrivateAttachmentSend, f.instance);
    await f.start();
    assert.equal(f.receipts[0][1], true);
    assert.equal(f.requests.filter(value => value.url.endsWith('/upload_reservations')).length, 1);
    assert.equal(f.requests.filter(value => value.url.endsWith('/claim_and_finish')).length, 1);
    assert.equal(f.requests.some(value => value.url.endsWith('/files')), false);
    assert.equal(f.store.files$().length, 1);
  }
});

test('picker cancellation cannot upload bytes, and stale cancellation cannot cancel a replacement', async t => {
  const f = fixture();
  t.after(() => f.instance.cancel());
  f.begin(); await tick();
  const oldId = f.descriptor.selectionId;
  f.instance.cancelSelection(oldId);
  assert.equal(f.requests[0].init.signal.aborted, true);
  assert.equal(f.store.files$().length, 0);
  assert.deepEqual(f.events, ['reserve']);
  f.begin('document', 'selection_' + 'b'.repeat(32)); await tick();
  f.instance.cancelSelection(oldId);
  await f.start();
  assert.equal(f.receipts[0][1], true);
  assert.equal(f.requests.filter(value => value.url.endsWith('/claim_and_finish')).length, 1);
});

test('selection mismatch and changed account/model discard prewarm before a regular scoped upload', async () => {
  for (const mode of ['selection', 'account', 'model']) {
    const f = fixture(); f.begin(); await tick();
    if (mode === 'selection') f.descriptor.selectionId = 'selection_' + 'c'.repeat(32);
    if (mode === 'account') f.setAccount('Bearer account-replacement');
    if (mode === 'model') f.setModel('model-replacement');
    await f.start();
    assert.equal(f.requests[0].init.signal.aborted, true);
    assert.equal(f.receipts[0][1], true);
    assert.equal(f.requests.filter(value => value.url.endsWith('/upload_reservations')).length, 2);
    assert.equal(f.store.files$().length, 1);
  }
});

test('document picker selecting an image does not reuse the document reservation', async () => {
  const f = fixture({ file: new File(['synthetic image'], 'image.png', { type: 'image/png' }) });
  f.begin('document'); await tick(); await f.start();
  assert.equal(f.receipts[0][1], true);
  assert.equal(f.requests.filter(value => value.url.endsWith('/files')).length, 1);
  assert.equal(f.requests.some(value => value.url.endsWith('/claim_and_finish')), false);
});

test('unconfirmed or hidden project picker context never allocates a reservation', async () => {
  const f = fixture();
  const id = '00000000-0000-4000-8000-000000000001';
  f.root.location.href = 'https://chatgpt.com/c/' + id;
  f.descriptor.href = f.root.location.href;
  f.root.__elonChatGptPrivateTransport.readAttachmentContext = async () => ({ conversationId: id,
    projectId: 'g-p-' + 'a'.repeat(32) });
  assert.equal(f.begin(), true); await tick();
  assert.equal(f.requests.length, 0);
  f.instance.cancel();
});

test('new and existing temporary pickers reserve text, PDF and images without changing persistence', async t => {
  for (const existing of [false, true]) for (const type of ['text/plain', 'application/pdf', 'image/png']) {
    const name = type === 'text/plain' ? 'fixture.txt' : type === 'application/pdf' ? 'fixture.pdf' : 'image.png';
    const f = fixture({ temporary: true, file: new File(['synthetic bytes'], name, { type }) });
    t.after(() => f.instance.cancel());
    let reads = 0;
    if (existing) {
      const id = '00000000-0000-4000-8000-000000000001';
      f.root.location.href = 'https://chatgpt.com/c/' + id + '?temporary-chat=true';
      f.descriptor.href = f.root.location.href;
      f.root.__elonChatGptPrivateTransport.readAttachmentContext = async () => {
        reads++; return { conversationId: id, temporary: true, ordinary: false };
      };
    }
    assert.equal(f.begin(type === 'image/png' ? 'image' : 'document'), true); await tick();
    assert.deepEqual(f.events, ['reserve'], 'no file bytes are read while the picker is open');
    f.instance.suspend();
    await f.start();
    assert.equal(reads, existing ? 2 : 0);
    assert.deepEqual(f.events.filter(value => ['reserve', 'create', 'bytes', 'claim', 'associated'].includes(value)),
      ['reserve', 'bytes', 'claim', 'associated']);
    const allocation = JSON.parse(f.requests[0].init.body);
    const claim = JSON.parse(f.requests[2].init.body);
    assert.equal(allocation.store_in_library, false);
    assert.equal(allocation.library_persistence_mode, 'required');
    assert.equal(claim.library_persistence_mode, 'required');
    assert.deepEqual(claim.metadata, { store_in_library: false, is_temporary_chat: true, is_project_thread: false });
    assert.equal(claim.index_for_retrieval, false);
    if (type === 'application/pdf') assert.equal(f.requests[2].init.headers['x-oai-model-slug'], 'synthetic-model');
    if (type === 'image/png') { assert.equal(claim.width, 30); assert.equal(claim.height, 20); }
    assert.equal(f.receipts[0][1], true);
    assert.equal(f.store.files$().length, 1);
    assert.equal(f.store.files$()[0].storeInLibrary, false);
    assert.equal(f.store.files$()[0].isTemporaryChat, true);
    assert.equal(f.fallbacks(), 0);
  }
});

test('temporary reservations also overlap byte preparation without a picker lease', async () => {
  const f = fixture({ temporary: true }); await f.start();
  assert.equal(f.receipts[0][1], true);
  assert.ok(f.events.indexOf('reserve') < f.events.indexOf('read_done'));
  assert.equal(f.requests.filter(value => value.url.endsWith('/claim_and_finish')).length, 1);
  assert.equal(f.store.files$()[0].isTemporaryChat, true);
});

test('temporary pending allocation does not add waiting or change the established create/process requests', async t => {
  let release;
  const f = fixture({ temporary: true, respond: url => url.endsWith('/upload_reservations')
    ? new Promise(resolve => { release = resolve; }) : null });
  t.after(() => f.instance.cancel());
  assert.equal(f.begin(), true); await tick(); await f.start();
  assert.equal(f.requests.filter(value => value.url.endsWith('/upload_reservations')).length, 1);
  assert.equal(f.requests.some(value => value.url.endsWith('/claim_and_finish')), false);
  const create = JSON.parse(f.requests.find(value => value.url.endsWith('/files')).init.body);
  const processing = JSON.parse(f.requests.find(value => value.url.endsWith('/process_upload_stream')).init.body);
  assert.equal(create.store_in_library, false);
  assert.equal(Object.hasOwn(create, 'library_persistence_mode'), false);
  assert.equal(Object.hasOwn(processing, 'library_persistence_mode'), false);
  assert.equal(processing.metadata.is_temporary_chat, true);
  assert.equal(f.receipts[0][1], true);
  release(Response.json({ eligible: true })); await tick();
  assert.equal(f.store.files$().length, 1);
});

test('temporary selection rejects changed server scope and privacy-mismatched processing without replay', async t => {
  for (const mode of ['scope', 'library']) {
    const f = fixture({ temporary: true, respond: url => mode === 'library' && url.endsWith('/claim_and_finish')
      ? new Response(JSON.stringify({ file_id: 'reservation-synthetic', event: 'file.processing.completed', progress: 100,
        extra: { library_persistence_result: 'library', metadata_object_id: 'library-synthetic' } })) : null });
    t.after(() => f.instance.cancel());
    if (mode === 'scope') {
      const id = '00000000-0000-4000-8000-000000000001';
      f.root.location.href = 'https://chatgpt.com/c/' + id + '?temporary-chat=true'; f.descriptor.href = f.root.location.href;
      let reads = 0;
      f.root.__elonChatGptPrivateTransport.readAttachmentContext = async () => ({ conversationId: id, temporary: ++reads === 1 });
    }
    assert.equal(f.begin(), true); await tick(); await f.start();
    assert.equal(f.store.files$().length, 0);
    assert.equal(f.events.includes('bytes'), mode === 'library');
    assert.equal(f.requests.some(value => value.url.endsWith('/files')), false);
    assert.equal(f.fallbacks(), mode === 'scope' ? 1 : 0);
    if (mode === 'library') assert.equal(f.receipts[0][1], false);
  }
});

test('a pending picker reservation is abandoned without adding another wait or allocation', async () => {
  let release;
  const f = fixture({ respond: url => url.endsWith('/upload_reservations')
    ? new Promise(resolve => { release = resolve; }) : null });
  f.begin(); await tick(); await f.start();
  assert.equal(f.requests[0].init.signal.aborted, true);
  assert.equal(f.requests.filter(value => value.url.endsWith('/upload_reservations')).length, 1);
  assert.equal(f.requests.filter(value => value.url.endsWith('/files')).length, 1);
  assert.equal(f.receipts[0][1], true);
  release(Response.json({ eligible: true })); await tick();
  assert.equal(f.store.files$().length, 1);
});

test('existing chat scope is revalidated after picking, even when its URL did not change', async t => {
  const f = fixture(); t.after(() => f.instance.cancel());
  const id = '00000000-0000-4000-8000-000000000001';
  f.root.location.href = 'https://chatgpt.com/c/' + id;
  f.descriptor.href = f.root.location.href;
  let reads = 0;
  f.root.__elonChatGptPrivateTransport.readAttachmentContext = async () => {
    reads++;
    return reads === 1 ? { conversationId: id, ordinary: true }
      : { conversationId: id, projectId: 'g-p-' + 'a'.repeat(32) };
  };
  f.begin(); await tick();
  assert.deepEqual(f.events, ['reserve']);
  await f.start();
  assert.equal(reads, 2);
  assert.equal(f.fallbacks(), 1);
  assert.deepEqual(f.events, ['reserve']);
  assert.equal(f.store.files$().length, 0);
});

test('production picker and background lifecycle are connected to the scoped reservation owner', () => {
  const readNative = name => fs.readFileSync(path.join(__dirname, '../android/app/src/main/kotlin/com/elon/app/', name), 'utf8');
  const picker = readNative('MainAttachmentPickerActions.kt');
  for (const launcher of ['cameraAttachmentLauncher', 'photoAttachmentLauncher', 'documentAttachmentLauncher']) {
    assert.match(picker, new RegExp('beginSelection\\(WebChatAttachmentSelectionKind\\.(IMAGE|DOCUMENT)\\)[\\s\\S]*?' + launcher + '\\.launch'));
  }
  assert.match(picker, /finishSelection\(if \(uris.size == 1\) files else emptyList\(\)\)/);
  assert.match(readNative('MainInputActions.kt'), /preparationPort = \{ inputComposerViews\?\.attachmentPreparation \}/);
  assert.match(readNative('MainSocialAiChatFeature.kt'), /WebChatAttachmentPreparationPort\(controller::beginAttachmentSelection\)/);
  const gateway = readNative('chatgptweb/ChatGptWebNativeAttachmentGateway.kt');
  assert.match(gateway, /val selectionId = pickerPreparation.take\(file\)\s+revokeLease\(preserveSelection = true\)/);
  assert.match(gateway, /\.put\("selectionId", selectionId\)/);
  const adapter = fs.readFileSync(path.join(__dirname, base, 'chatgpt_web_adapter.js'), 'utf8');
  const dispose = adapter.slice(adapter.indexOf('  function dispose()'), adapter.indexOf('  window.__elonChatGptBridge ='));
  assert.match(dispose, /privateAttachments.suspend\(\)/);
  assert.doesNotMatch(dispose, /privateAttachments.cancel\(\)/);
  assert.match(readNative('chatgptweb/ChatGptBackgroundSession.kt'), /fun deactivate\(\) \{\s+pageAdapter\?\.cancelAttachmentSelection\(\)/);
});

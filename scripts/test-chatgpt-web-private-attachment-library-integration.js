'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const { webcrypto, createHash } = require('node:crypto');
const fs = require('node:fs'), path = require('node:path'), vm = require('node:vm');
const base = '../android/app/src/main/assets/';
const load = name => require(base + 'chatgpt_web_private_attachment_' + name + '.js');
const protocol = load('protocol'), library = load('library'), reservation = load('reservation');
const composerModule = load('composer'), send = load('send');
const tick = () => new Promise(resolve => setImmediate(resolve));
const gates = () => ({ t6: () => ({ loadingStatus: 'Ready',
  getFeatureGate: name => ({ name, value: true, details: { reason: 'Network:Recognized' } }) }) });
const completed = id => JSON.stringify({ file_id: id, event: 'file.processing.completed', progress: 100 });

function fixture(options = {}) {
  let values = [], account = 'Bearer synthetic-account', writes = 0, fallbacks = 0, releaseBytes;
  const calls = [], receipts = [];
  const file = options.file || new File(['synthetic integration'], 'picked.txt', { type: 'text/plain' });
  const files$ = () => values;
  files$.set = next => { values = next; writes++; };
  const store = { files$, readyFiles$: () => values, hasUploadInProgress$: () => false };
  const props = { conversation: {}, onCreateNewCompletion() {}, currentModelId: 'synthetic-model',
    entrySurface: 'chat_composer', isLibraryEnabled: options.libraryEnabled !== false };
  const top = { stateNode: {}, memoizedProps: props }; top.stateNode.current = top;
  const input = { isConnected: true, __reactFiber$test: { memoizedProps: { value: store }, return: top } };
  const root = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/' + (options.temporary ? '?temporary-chat=true' : '') },
    document: { querySelector: selector => selector === '#upload-files' ? input : null },
    performance: { getEntriesByName: () => [{}], now: () => performance.now() },
    crypto: webcrypto, AbortController, FormData, setTimeout, clearTimeout, setInterval, clearInterval,
    btoa: value => Buffer.from(value, 'binary').toString('base64'),
    __elonChatGptDocumentToken: 'doc_fixture_library',
    __elonChatGptComposer: { currentModel: () => 'synthetic-model' },
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ authorization: account }),
      acquireSameOriginRequestHeaders: async () => ({ authorization: account, Cookie: 'do-not-forward',
        'openai-sentinel-proof-token': 'do-not-forward' }) },
    __elonChatGptPrivateAttachmentProtocol: protocol,
    __elonChatGptPrivateAttachmentTransport: load('transport'),
    __elonChatGptPrivateAttachmentBytes: load('bytes'),
    __elonChatGptPrivateJsonRequest: require(base + 'chatgpt_web_private_json_request.js'),
    __elonChatGptPrivateAttachmentLibrary: { version: library.version, create: (host, config) => library.create(host, {
      ...config, loadRuntime: options.loadRuntime || (async () => gates()),
    }) },
  };
  if (options.selection) root.__elonChatGptPrivateAttachmentSelection = load('selection');
  if (options.reserved) root.__elonChatGptPrivateAttachmentReservation = {
    version: reservation.version, create: (host, config) => reservation.create(host, { ...config,
      loadRuntime: async () => ({ t6: () => ({ loadingStatus: 'Ready', getExperiment: name => ({ name,
        groupName: 'treatment', details: { reason: 'Network:Recognized' }, get: () => true }) }) }),
    }),
  };
  const destination = options.estuary ? '/backend-api/estuary/upload_content_bytes'
    : 'https://uploads.oaiusercontent.com/fixture?sig=synthetic';
  root.fetch = async (url, init) => {
    calls.push({ url, init });
    if (url.endsWith('/upload_reservations')) return Response.json({ eligible: true,
      reservation_id: 'reservation-synthetic', upload_url: destination,
      upload_url_expires_at: new Date(Date.now() + 120000).toISOString(),
      reservation_expires_at: new Date(Date.now() + 180000).toISOString() });
    if (url.endsWith('/files')) return Response.json({ status: 'success', file_id: 'file-uploaded', upload_url: destination,
      ...(options.multipart ? { direct_library_upload_strategy: { kind: 'direct_azure_multipart',
        part_size_bytes: Math.ceil(file.size / 2), part_count: 2, max_part_concurrency: 2 } } : {}) });
    if (url.endsWith('/library/reuse')) {
      if (options.onLookup) return options.onLookup(root, init, () => releaseBytes?.());
      return Response.json({ reusable_library_file: { file_id: 'file-reused', library_file_id: 'library-reused',
        file_name: 'stored-name.txt', mime_type: file.type } });
    }
    if (url.endsWith('/claim_and_finish')) return new Response(completed('reservation-synthetic'));
    if (url.endsWith('/process_upload_stream')) return new Response(completed('file-uploaded'));
    if (options.fastBytes) return new Response(null, { status: 201 });
    return new Promise((resolve, reject) => {
      releaseBytes = () => resolve(new Response(null, { status: 201 }));
      const abort = () => reject(new Error('cancelled'));
      if (init.signal.aborted) abort(); else init.signal.addEventListener('abort', abort, { once: true });
    });
  };
  const composer = composerModule.create(root);
  const instance = send.create(root, { composer: options.composer || composer,
    source: { read: async () => { await tick(); return file; } },
    image: { available: () => options.imageAvailable !== false,
      prepare: async () => ({ file, dimensions: { width: 32, height: 24 } }) } });
  const descriptor = { name: file.name, size: file.size, type: file.type, documentToken: root.__elonChatGptDocumentToken,
    href: root.location.href, leaseId: '00000000-0000-4000-8000-000000000000',
    ...('uploadCopy' in options ? { uploadCopy: options.uploadCopy } : {}) };
  return { root, props, instance, composer, descriptor, store, file, calls, receipts, writes: () => writes,
    setAccount: value => { account = value; }, fallbacks: () => fallbacks,
    releaseBytes: () => releaseBytes?.(), start: () => instance.start(JSON.stringify(descriptor),
      (...value) => receipts.push(value), () => {}, () => { fallbacks++; }) };
}

test('native sender associates the reused library entry once and skips processing the cancelled upload', async () => {
  const f = fixture(); await f.start();
  assert.deepEqual(f.receipts, [['request_attachment_upload', true, 'private_attachment_associated']]);
  const creation = f.calls.find(call => call.url.endsWith('/files'));
  assert.equal(JSON.parse(creation.init.body).store_in_library, true);
  assert.equal(JSON.parse(creation.init.body).library_persistence_mode, 'opportunistic');
  const lookup = f.calls.find(call => call.url.endsWith('/library/reuse'));
  assert.deepEqual(JSON.parse(lookup.init.body), { file_size_bytes: f.file.size,
    sha256_digest: createHash('sha256').update(await f.file.text()).digest('hex') });
  assert.equal(lookup.init.credentials, 'include'); assert.equal(lookup.init.redirect, 'error');
  assert.equal(lookup.init.headers.Cookie, undefined);
  assert.equal(lookup.init.headers['openai-sentinel-proof-token'], undefined);
  const uploaded = f.calls.find(call => call.init.method === 'PUT');
  assert.equal(uploaded.init.signal.aborted, true); assert.equal(uploaded.init.credentials, 'omit');
  assert.equal(uploaded.init.headers.authorization, undefined);
  assert.equal(f.calls.some(call => /process_upload_stream|claim_and_finish/.test(call.url)), false);
  assert.equal(f.writes(), 1); assert.equal(f.fallbacks(), 0);
  const attached = f.store.files$()[0];
  assert.equal(attached.source, 'library'); assert.equal(attached.autoReused, true);
  assert.equal(attached.fileId, 'file-reused'); assert.equal(attached.fileSpec.id, 'file-reused');
  assert.equal(attached.libraryFileId, 'library-reused'); assert.equal(attached.fileSpec.name, 'stored-name.txt');
  assert.equal(attached.file, f.file); assert.equal(attached.storeInLibrary, true);
  const rows = f.instance.merge([]); assert.equal(rows[0].name, 'picked.txt');
  assert.equal(f.composer.remove(rows[0].id), true); assert.equal(f.store.files$().length, 0);
});

test('a ready library reservation is abandoned on reuse without claiming or allocating again', async () => {
  const f = fixture({ reserved: true }); await f.start();
  assert.equal(f.receipts[0][1], true);
  assert.equal(f.calls.filter(call => call.url.endsWith('/upload_reservations')).length, 1);
  const body = JSON.parse(f.calls[0].init.body);
  assert.equal(body.store_in_library, true); assert.equal(body.library_persistence_mode, 'opportunistic');
  assert.equal(f.calls.some(call => /claim_and_finish$|\/files$/.test(call.url)), false);
  assert.equal(f.store.files$()[0].fileId, 'file-reused'); assert.equal(f.writes(), 1);
});

test('PDF/image descriptors and all evidenced byte routes retain their original scope during reuse', async () => {
  for (const options of [{ file: new File(['pdf'], 'fixture.pdf', { type: 'application/pdf' }) },
    { file: new File(['png'], 'fixture.png', { type: 'image/png' }), estuary: true }, { multipart: true }]) {
    const f = fixture(options); await f.start();
    assert.equal(f.receipts[0][1], true); assert.equal(f.writes(), 1);
    assert.equal(f.store.files$()[0].fileSpec.mimeType, f.file.type);
    const create = f.calls.find(call => call.url.endsWith('/files'));
    const lookup = f.calls.find(call => call.url.endsWith('/library/reuse'));
    assert.equal(lookup.init.headers['x-oai-model-slug'], undefined);
    if (f.file.type === 'application/pdf') assert.equal(create.init.headers['x-oai-model-slug'], 'synthetic-model');
    if (options.estuary) {
      assert.equal(f.store.files$()[0].fileSpec.width, 32);
      assert.equal(f.store.files$()[0].fileSpec.height, 24);
      assert.ok(f.calls.some(call => call.init.body instanceof FormData));
    }
    if (options.multipart) {
      assert.equal(f.calls.filter(call => /comp=block&/.test(call.url)).length, 2);
      assert.equal(f.calls.some(call => call.url.includes('comp=blocklist')), false);
    }
    assert.equal(f.calls.some(call => call.url.endsWith('/process_upload_stream')), false);
  }
});

test('a lookup miss or failure leaves the existing byte upload and processing route intact', async () => {
  for (const status of [200, 503]) {
    const f = fixture({ onLookup: async (_, __, release) => {
      setTimeout(release, 10); return status === 200 ? Response.json({ reusable_library_file: null }) : new Response('', { status });
    } });
    await f.start(); assert.equal(f.receipts[0][1], true); assert.equal(f.writes(), 1);
    assert.equal(f.calls.filter(call => call.init.method === 'PUT').length, 1);
    assert.equal(f.calls.filter(call => call.url.endsWith('/process_upload_stream')).length, 1);
    assert.equal(f.store.files$()[0].fileId, 'file-uploaded');
    assert.equal(f.store.files$()[0].source, 'local'); assert.equal(f.fallbacks(), 0);
  }
});

test('completed bytes never wait for a slow runtime and a late reuse result cannot add another attachment', async () => {
  let release;
  const f = fixture({ fastBytes: true, loadRuntime: () => new Promise(resolve => { release = resolve; }) });
  await f.start(); assert.equal(f.receipts[0][1], true); assert.equal(f.writes(), 1);
  assert.equal(f.store.files$()[0].fileId, 'file-uploaded');
  release(gates()); await tick();
  assert.equal(f.calls.some(call => call.url.endsWith('/library/reuse')), false);
  assert.equal(f.writes(), 1);
});

test('temporary chats and disabled library settings never enter reuse', async () => {
  for (const options of [{ temporary: true }, { libraryEnabled: false }]) {
    const f = fixture({ ...options, fastBytes: true }); await f.start();
    assert.equal(f.receipts[0][1], true); assert.equal(f.writes(), 1);
    assert.equal(f.calls.some(call => call.url.endsWith('/library/reuse')), false);
    assert.equal(f.store.files$()[0].storeInLibrary, false);
    assert.equal(f.store.files$()[0].source, 'local');
  }
});

test('upload-copy reads local bytes once without reuse, reservations or changing storage intent', async () => {
  for (const options of [{}, { reserved: true }, { temporary: true }, { libraryEnabled: false },
    { estuary: true }, { multipart: true }]) {
    const f = fixture({ ...options, uploadCopy: true, fastBytes: true }); await f.start();
    assert.deepEqual(f.receipts, [['request_attachment_upload', true, 'private_attachment_associated']]);
    assert.equal(f.fallbacks(), 0); assert.equal(f.writes(), 1);
    assert.equal(f.calls.some(call => /library\/reuse|upload_reservations|claim_and_finish/.test(call.url)), false);
    const creations = f.calls.filter(call => call.url.endsWith('/files'));
    assert.equal(creations.length, 1);
    const body = JSON.parse(creations[0].init.body);
    assert.equal(body.store_in_library, !options.temporary && options.libraryEnabled !== false);
    assert.equal(body.uploadCopy, undefined); assert.equal(body.checkForReusableLibraryFile, undefined);
    assert.equal(f.calls.filter(call => call.url.endsWith('/process_upload_stream')).length, 1);
    const attached = f.store.files$()[0];
    assert.equal(attached.fileId, 'file-uploaded'); assert.equal(attached.source, 'local');
    assert.notEqual(attached.autoReused, true); assert.equal(attached.file, f.file);
  }
});

test('explicit copy never silently falls back when its composer, image or scope is unavailable', async () => {
  for (const mode of ['composer', 'image', 'scope', 'transport']) {
    const fake = { available: () => mode !== 'composer',
      capture: () => ({ token: 'doc_fixture_library', href: 'https://chatgpt.com/' }),
      current: () => true, prepare: async () => false };
    const f = fixture({ uploadCopy: true, fastBytes: true,
      ...(mode === 'composer' || mode === 'scope' ? { composer: fake } : {}),
      ...(mode === 'image' ? { file: new File(['png'], 'fixture.png', { type: 'image/png' }), imageAvailable: false } : {}) });
    if (mode === 'transport') delete f.root.__elonChatGptPrivateTransport;
    await f.start(); assert.equal(f.fallbacks(), 0);
    assert.equal(f.receipts.length, 1); assert.equal(f.receipts[0][1], false);
    assert.equal(f.writes(), 0); assert.equal(f.calls.length, 0);
  }
});

test('malformed upload choice fails before reads or writes, while explicit false preserves reuse', async () => {
  for (const uploadCopy of ['true', 'false', null, 0, {}, []]) {
    const f = fixture({ uploadCopy, fastBytes: true }); await f.start();
    assert.equal(f.receipts[0][1], false); assert.equal(f.fallbacks(), 0);
    assert.equal(f.calls.length, 0); assert.equal(f.writes(), 0);
  }
  const f = fixture({ uploadCopy: false }); await f.start();
  assert.equal(f.receipts[0][1], true); assert.equal(f.store.files$()[0].fileId, 'file-reused');
});

test('choosing copy after opening the picker abandons its old slot without claiming it', async () => {
  const f = fixture({ selection: true, reserved: true, uploadCopy: true, fastBytes: true });
  f.descriptor.selectionId = 'selection_' + 'a'.repeat(32);
  assert.equal(f.instance.beginSelection(JSON.stringify({ id: f.descriptor.selectionId,
    kind: 'document', documentToken: f.descriptor.documentToken, href: f.descriptor.href })), true);
  for (let i = 0; i < 100 && !f.calls.some(call => call.url.endsWith('/upload_reservations')); i++) await tick();
  assert.equal(f.calls.filter(call => call.url.endsWith('/upload_reservations')).length, 1);
  await tick(); await f.start();
  assert.equal(f.receipts[0][1], true); assert.equal(f.writes(), 1); assert.equal(f.fallbacks(), 0);
  assert.equal(f.calls.filter(call => call.url.endsWith('/upload_reservations')).length, 1);
  assert.equal(f.calls.filter(call => call.url.endsWith('/files')).length, 1);
  assert.equal(f.calls.some(call => /claim_and_finish|library\/reuse/.test(call.url)), false);
  assert.equal(f.store.files$()[0].fileId, 'file-uploaded');
});

test('copy cancellation or context change after allocation cannot associate or replay', async () => {
  for (const mode of ['cancel', 'document', 'account']) {
    const f = fixture({ uploadCopy: true, fastBytes: true });
    const fetch = f.root.fetch;
    f.root.fetch = async (url, init) => {
      const result = await fetch(url, init);
      if (url.endsWith('/files')) {
        if (mode === 'cancel') f.instance.cancel();
        if (mode === 'document') f.root.__elonChatGptDocumentToken = 'doc_changed';
        if (mode === 'account') f.setAccount('Bearer another-account');
      }
      return result;
    };
    await f.start();
    assert.equal(f.receipts[0][1], false); assert.equal(f.writes(), 0); assert.equal(f.fallbacks(), 0);
    assert.equal(f.calls.length, 1); assert.equal(f.calls[0].url, '/backend-api/files');
  }
});

test('context changes and cancellation during reuse cannot attach, replay or switch to compatibility', async () => {
  for (const mode of ['account', 'document', 'library', 'cancel']) {
    const f = fixture({ onLookup: async () => {
      if (mode === 'account') f.setAccount('Bearer another-account');
      if (mode === 'document') f.root.__elonChatGptDocumentToken = 'doc_replaced';
      if (mode === 'library') f.props.isLibraryEnabled = false;
      if (mode === 'cancel') f.instance.cancel();
      return Response.json({ reusable_library_file: { file_id: 'file-reused', library_file_id: 'library-reused', file_name: 'stored.txt' } });
    } });
    await f.start(); assert.equal(f.receipts[0][1], false);
    assert.equal(f.writes(), 0); assert.equal(f.fallbacks(), 0);
    assert.equal(f.calls.filter(call => call.url.endsWith('/files')).length, 1);
    assert.equal(f.calls.filter(call => call.init.method === 'PUT').length, 1);
    assert.equal(f.calls.some(call => call.url.endsWith('/process_upload_stream')), false);
  }
});

test('production bundle registers one library module before its transport and remains executable', () => {
  const adapter = require('./chatgpt-web-adapter-assembly').readAdapterSource();
  const declaration = adapter.slice(adapter.indexOf('private val ADAPTER_ASSETS = listOf('));
  const assets = [...declaration.slice(0, declaration.indexOf('\n        )')).matchAll(/"([^"\n]+\.js)"/g)].map(match => match[1]);
  const name = 'chatgpt_web_private_attachment_library.js';
  assert.equal(assets.filter(value => value === name).length, 1);
  assert.ok(assets.indexOf(name) < assets.indexOf('chatgpt_web_private_attachment_transport.js'));
  new vm.Script(assets.map(value => fs.readFileSync(path.join(__dirname, base, value), 'utf8')).join('\n'));
});

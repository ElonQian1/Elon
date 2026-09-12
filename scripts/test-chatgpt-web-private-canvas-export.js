'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
const { fixture, PATH, ID } = require('./fixtures/chatgpt-web-private-canvas-documents.cjs');
const exporter = require('../android/app/src/main/assets/chatgpt_web_private_canvas_export.js');
const downloads = require('../android/app/src/main/assets/chatgpt_web_private_file_download.js');
const binary = require('../android/app/src/main/assets/chatgpt_web_private_library_download.js');
const actions = require('../android/app/src/main/assets/chatgpt_web_canvas_document_actions.js');
const MIME = { pdf: 'application/pdf', docx: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document' };
const URL = 'https://chatgpt.com/backend-api/export_doc/canvas';
function setup(options = {}) {
  const f = fixture(), packets = [], requests = [], stored = [], receipts = [];
  const format = options.format || 'pdf';
  const data = options.bytes || Buffer.from(format === 'pdf' ? '%PDF-1.7\nSynthetic\n%%EOF' : 'PK\x03\x04SyntheticDocx');
  let saved = false, cancelled = false;
  const bridge = { onmessage: null, postMessage(raw) {
    const p = JSON.parse(raw); packets.push(p);
    if (p.cancel) { cancelled = true; if (!saved) stored.length = 0; return; }
    if (p.byteOperation === 'chunk') stored.push(Buffer.from(p.data, 'base64'));
    if (p.byteOperation === 'commit') saved = true;
    options.packet?.(p, f);
    if (options.dropCommit && p.byteOperation === 'commit') return;
    queueMicrotask(() => bridge.onmessage?.({ data: JSON.stringify({ leaseId: p.leaseId, sequence: p.sequence,
      byteOperation: p.byteOperation, state: { begin: 'ready', chunk: 'written', commit: 'saved' }[p.byteOperation] }) }));
  } };
  Object.assign(f.page, { __elonChatGptPrivateCanvasExport: exporter, __elonChatGptPrivateLibraryDownload: binary,
    __elonChatGptPrivateCanvasDocuments: { create: () => f.api }, elonChatGptFileDownload: bridge, AbortController, btoa,
    setTimeout: (fn, ms) => setTimeout(fn, options.dropCommit && ms === 8000 || options.fastPrepare && ms === 20000 ? 15 : ms),
    fetch: async (url, init) => {
      requests.push({ url, init });
      await options.fetch?.(f);
      if (options.lostResponse) throw Error('network_failure');
      const response = new Response(new ReadableStream({ start(controller) {
        if (options.fragmented) for (const b of data) controller.enqueue(Uint8Array.of(b));
        else controller.enqueue(data);
        controller.close();
      }, cancel() { f.bodyCancelled = true; } }), { status: options.status || 200,
        headers: { 'content-type': options.mime || MIME[format], 'content-length': String(options.length ?? data.length) } });
      Object.defineProperty(response, 'url', { value: options.url || url });
      return response;
    } });
  const download = downloads.create(f.page);
  f.page.__elonChatGptPrivateFileDownload = download;
  async function prepare() {
    const list = await f.list();
    return f.run({ operation: 'prepare_export', id: ID, ticket: list.ticket, scope: list.scope, format });
  }
  const run = value => download.start(JSON.stringify({ version: 1, byteTransferVersion: 1,
    leaseId: '00000000-0000-4000-8000-000000000001', documentToken: f.page.__elonChatGptDocumentToken,
    href: f.page.location.href, path: PATH, name: value.file.name, downloadHandle: value.file.downloadHandle }),
    (...args) => receipts.push(args));
  return Object.assign(f, { prepare, runDownload: run, download, packets, exports: requests, stored, receipts, data,
    saved: () => saved, cancelled: () => cancelled });
}

for (const format of ['pdf', 'docx']) test('original ' + format + ' export uses one private POST and the existing acknowledged byte owner', async () => {
  const f = setup({ format, fragmented: true }), result = await f.prepare();
  assert.equal(result.ok, true); assert.equal(f.exports.length, 0);
  assert.equal(result.exportFile.documentVersion, 4);
  await f.runDownload(result.exportFile);
  assert.equal(f.saved(), true); assert.deepEqual(Buffer.concat(f.stored), f.data);
  assert.equal(f.exports.length, 1);
  const request = f.exports[0];
  assert.equal(request.url, URL); assert.equal(request.init.method, 'POST');
  assert.equal(request.init.redirect, 'error'); assert.equal(request.init.credentials, 'include');
  assert.deepEqual(JSON.parse(request.init.body), { textdoc_id: ID, export_type: format });
  assert.ok(f.requests.every(value => value.init.method === 'GET'));
  assert.equal(f.requests.length, 3); assert.deepEqual(f.receipts.at(-1), ['download_conversation_file', true, 'download_saved']);
  assert.equal(f.packets.filter(p => p.byteOperation === 'commit').length, 1);
  assert.ok(!JSON.stringify(f.packets).includes('Bearer'));
  await f.runDownload(result.exportFile);
  assert.equal(f.exports.length, 1); assert.equal(f.receipts.at(-1)[2], 'download_selection_expired');
});

test('export preparation and display are typed and do not send or modify the document', async () => {
  const f = setup(), events = [], list = await f.list(), api = actions.create(f.page);
  const command = { operation: 'prepare_export', path: PATH, id: ID, ticket: list.ticket, scope: list.scope, format: 'pdf' };
  const call = input => new Promise(resolve => api.handle('canvas_document',
    { value: JSON.stringify(input), requestId: 'mcp_export', selected: false }, (_, ok, detail) => resolve({ ok, detail }),
    () => f.snapshot, e => events.push(e)));
  assert.equal((await call(command)).ok, true);
  assert.equal(events[0].exportFile.format, 'pdf'); assert.equal(f.exports.length, 0);
  for (const change of [{ format: 'html' }, { url: URL }, { headers: {} }, { content: 'not a body save' }]) {
    assert.equal((await call({ ...command, ...change })).ok, false);
  }
  f.rows[0].textdoc_type = 'code/javascript';
  await f.list(true);
  assert.equal((await f.prepare()).code, 'canvas_export_unsupported');
});

for (const mode of ['version', 'title', 'body', 'comment', 'pending', 'streaming', 'account', 'page', 'token']) {
  test('stale ' + mode + ' is rejected before exporting', async () => {
    const f = setup(), result = await f.prepare();
    if (mode === 'version') f.rows[0].version++;
    if (mode === 'title') f.rows[0].title += 'changed';
    if (mode === 'body') f.rows[0].content += 'changed';
    if (mode === 'comment') f.rows[0].comments = [{ id: 'new', start: 0, end: 1, content: 'new' }];
    if (mode === 'pending') f.edits.userEdits[ID] = [{ isPending: true }];
    if (mode === 'streaming') f.snapshot.streaming = true;
    if (mode === 'account') f.headers.Authorization = 'Bearer another_identity';
    if (mode === 'page') f.page.location = new global.URL('https://chatgpt.com/');
    if (mode === 'token') f.page.__elonChatGptDocumentToken = 'doc_changed';
    await f.runDownload(result.exportFile);
    assert.equal(f.exports.length, 0); assert.equal(f.saved(), false); assert.equal(f.receipts.at(-1)[1], false);
  });
}

for (const options of [{ status: 401 }, { status: 403 }, { status: 404 }, { status: 500 }, { mime: 'text/html' },
  { mime: 'application/json' }, { url: 'https://example.com/export' }, { bytes: Buffer.from('<html>') },
  { format: 'docx', bytes: Buffer.from('%PDF-1.7') }, { bytes: Buffer.from('%P') }, { bytes: Buffer.alloc(0) },
  { length: 12345 }, { lostResponse: true }, { fetch: f => { f.rows[0].version++; } }]) {
  test('export rejects incomplete or changed output ' + JSON.stringify(options), async () => {
    const f = setup(options), result = await f.prepare();
    await f.runDownload(result.exportFile);
    assert.equal(f.saved(), false); assert.equal(f.receipts.at(-1)[1], false);
    assert.equal(f.exports.length, 1); assert.ok(!f.packets.some(p => p.byteOperation === 'commit'));
    await f.runDownload(result.exportFile); assert.equal(f.exports.length, 1);
  });
}

test('document writes share ownership while the server renders the export', async () => {
  let entered, release;
  const f = setup({ fetch: () => new Promise(resolve => { entered = true; release = resolve; }) });
  const result = await f.prepare(), pending = f.runDownload(result.exportFile);
  for (let i = 0; !entered && i < 100; i++) await new Promise(resolve => setTimeout(resolve, 1));
  assert.ok(entered); assert.equal((await f.save(result.ticket)).code, 'canvas_busy');
  release(); await pending; assert.equal(f.saved(), true);
});

test('cancellation and source changes during transfer prevent the final save', async () => {
  for (const cancel of [true, false]) {
    const f = setup({ fragmented: true, packet: (p, current) => {
      if (p.byteOperation !== 'chunk') return;
      if (cancel) current.download.cancel(); else current.snapshot.streaming = true;
    } });
    await f.runDownload((await f.prepare()).exportFile);
    assert.equal(f.saved(), false); assert.ok(!f.packets.some(p => p.byteOperation === 'commit'));
  }
});

test('lost save acknowledgement is not retried and long filenames keep the selected extension', async () => {
  const f = setup({ dropCommit: true }); f.rows[0].title = 'x'.repeat(512);
  const result = await f.prepare(); assert.ok(result.exportFile.file.name.length < 150);
  assert.ok(result.exportFile.file.name.endsWith('.pdf'));
  await f.runDownload(result.exportFile);
  assert.equal(f.receipts.at(-1)[2], 'download_confirmation_unknown');
  assert.equal(f.packets.filter(p => p.byteOperation === 'commit').length, 1);
  await f.runDownload(result.exportFile); assert.equal(f.exports.length, 1);
});

test('a response delayed by post-export verification is closed after the download timed out', async () => {
  const f = setup({ fastPrepare: true }); let release;
  f.setHook(({ init }) => init.method === 'GET' && f.exports.length ? new Promise(resolve => { release = resolve; }) : undefined);
  await f.runDownload((await f.prepare()).exportFile);
  assert.equal(f.saved(), false); assert.equal(f.receipts.at(-1)[2], 'download_transfer_timeout');
  assert.equal(typeof release, 'function'); release();
  for (let i = 0; !f.bodyCancelled && i < 25; i++) await new Promise(resolve => setTimeout(resolve, 1));
  assert.equal(f.bodyCancelled, true);
  assert.ok(!f.packets.some(p => p.byteOperation === 'begin'));
});

test('production native editor and the official binary export contract are connected', {
  skip: !process.env.CHATGPT_PUBLIC_RUNTIME_DIR && 'Reviewed public asset is an optional offline input.'
}, () => {
  const source = fs.readFileSync(path.join(process.env.CHATGPT_PUBLIC_RUNTIME_DIR, 'bc86e6a9-lyyfjzfq9uy5wnj2.js'), 'utf8');
  assert.equal(crypto.createHash('sha256').update(source).digest('hex'), '9417a623b117d5c76343b91f9a2c6c3a0402c24d50cec8fd8a671e2fc1493eeb');
  assert.ok(source.includes('safePost(`/export_doc/canvas`,{requestBody:{textdoc_id:e,export_type:t},skipJsonTransform:!0})'));
  assert.ok(source.includes('N(await t.blob()'));
  const read = name => fs.readFileSync(path.join(__dirname, '../android/app/src/main/kotlin/com/elon/app/', name), 'utf8');
  assert.ok(read('WebChatCanvasEditorView.kt').includes('web-chat-canvas-editor-export'));
  assert.ok(read('WebChatCanvasManagementCoordinator.kt').includes('web-chat-canvas-export-formats'));
  assert.ok(read('WebChatCanvasDocumentsCoordinator.kt').includes('downloads.show(owner, requestId)'));
  assert.ok(read('chatgptweb/ChatGptWebMcpActions.kt').includes('ChatGptWebCanvasExportProtocol.selected'));
  assert.ok(read('chatgptweb/ChatGptWebAdapterAssets.kt').includes('chatgpt_web_private_canvas_export.js'));
});

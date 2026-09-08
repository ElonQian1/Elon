'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const { webcrypto } = require('node:crypto');
const projection = require('../android/app/src/main/assets/chatgpt_web_private_history_projection.js');
const download = require('../android/app/src/main/assets/chatgpt_web_private_file_download.js');
const library = require('../android/app/src/main/assets/chatgpt_web_private_library_download.js');
const request = require('../android/app/src/main/assets/chatgpt_web_private_json_request.js');
const LIBRARY = 'libfile_synthetic';
const PROJECT = 'g-p-' + 'a'.repeat(32);
const DOWNLOAD = 'https://chatgpt.com/api/library/files/' + LIBRARY + '/download';

function fixture(options = {}) {
  const data = options.data || Uint8Array.from({ length: 100_009 }, (_, i) => i % 256);
  const calls = [], packets = [], receipts = [], stored = [];
  let identity = 'Bearer synthetic-library-identity', sequence = 0, reads = 0, saved = false, cancelled = false;
  const attachment = { name: 'fixture.bin', source: 'library', library_file_id: LIBRARY, mime_type: 'application/octet-stream' };
  if (options.linked) attachment.id = 'file-synthetic';
  const sharedReference = { library_file_id: LIBRARY, name: 'fixture.bin', mime_type: 'application/octet-stream',
    size_bytes: data.byteLength, display_path: '/Shared/fixture.bin', entrypoint: 'library' };
  const payload = { ...(options.project ? { gizmo_id: PROJECT } : {}), messages: [{ id: 'message-synthetic',
    author: { role: 'user' }, content: { parts: ['fixture'] }, metadata: options.shared
      ? { shared_library_file_references: [sharedReference] } : { attachments: [attachment] } }] };
  const bridge = { onmessage: null, postMessage(raw) {
    const packet = JSON.parse(raw); packets.push(packet);
    if (packet.cancel) { cancelled = true; if (!saved) stored.length = 0; return; }
    let state = 'failed';
    if (packet.byteOperation === 'begin') {
      assert.equal(packet.sequence, 0); state = 'ready';
    } else if (packet.byteOperation === 'chunk') {
      assert.equal(packet.sequence, sequence++);
      const bytes = Buffer.from(packet.data, 'base64');
      assert.ok(bytes.length <= 49152); stored.push(bytes); state = 'written';
    } else if (packet.byteOperation === 'commit') {
      assert.equal(packet.sequence, sequence);
      assert.equal(packet.totalBytes, Buffer.concat(stored).length);
      saved = !options.failCommit; state = saved ? 'saved' : 'failed';
    }
    if (options.dropAck === packet.byteOperation) return;
    if (options.onPacket) options.onPacket(packet, f);
    const respond = () => bridge.onmessage?.({ data: JSON.stringify({ leaseId: packet.leaseId,
      sequence: packet.sequence, byteOperation: packet.byteOperation, state }) });
    if (options.manualAck) f.ack = respond; else queueMicrotask(respond);
  } };
  const root = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/current' },
    __elonChatGptDocumentToken: 'doc_library_download',
    __elonChatGptPrivateLibraryDownload: library, __elonChatGptPrivateHistoryProjection: projection,
    __elonChatGptPrivateJsonRequest: request,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: identity }) },
    elonChatGptFileDownload: bridge, crypto: webcrypto, AbortController, btoa,
    setTimeout: (fn, ms) => setTimeout(fn, options.fastTimeout && ms < 120000 ? Math.min(ms, 25) : ms), clearTimeout,
    fetch: async (url, init) => {
      calls.push({ url, init });
      if (options.linked && new URL(url).pathname === '/backend-api/files/file-synthetic/simple') {
        const response = Response.json(options.info || { file_id: attachment.id,
          is_library_file: true, library_file_id: LIBRARY, is_project: false });
        options.onMetadata?.(f);
        return response;
      }
      if (options.hangFetch) return new Promise(() => {});
      const headers = { 'content-type': options.mime || 'application/octet-stream',
        ...(options.noLength ? {} : { 'content-length': String(options.length ?? data.byteLength) }),
        ...(options.disposition ? { 'content-disposition': options.disposition } : {}),
        ...(options.encoding ? { 'content-encoding': options.encoding } : {}) };
      const response = new Response(new ReadableStream({
        pull(controller) {
          reads++;
          if (options.emptyLoop) { controller.enqueue(new Uint8Array()); return; }
          if (reads === 1) controller.enqueue(data); else controller.close();
        },
        cancel() { f.bodyCancelled = true; },
      }, { highWaterMark: 0 }), { status: options.status || 200, headers });
      Object.defineProperty(response, 'url', { value: options.finalUrl || url });
      return response;
    } };
  const api = download.create(root), privateProjection = projection.create({});
  const register = () => api.register('/c/selected', payload, privateProjection.files(payload));
  const run = row => api.start(JSON.stringify({ version: 1, byteTransferVersion: options.noNativeBytes ? undefined : 1,
    leaseId: '00000000-0000-4000-8000-000000000001', documentToken: root.__elonChatGptDocumentToken,
    href: root.location.href, path: '/c/selected', name: row.name, downloadHandle: row.downloadHandle }),
    (...args) => receipts.push(args));
  const f = { api, root, payload, attachment, sharedReference, calls, packets, receipts, data, stored, register, run,
    setIdentity: value => { identity = value; }, get reads() { return reads; }, get saved() { return saved; },
    get cancelled() { return cancelled; } };
  return f;
}

function contentFixture(options = {}) {
  const f = fixture(options), binaryFetch = f.root.fetch;
  delete f.attachment.source;
  delete f.attachment.library_file_id;
  f.attachment.id = 'file-synthetic';
  f.attachment.mime_type = options.mime || 'application/octet-stream';
  f.root.fetch = async (url, init) => {
    if (new URL(url).pathname === '/backend-api/files/download/file-synthetic') {
      f.calls.push({ url, init });
      return Response.json({ status: 'success', file_id: 'file-synthetic',
        download_url: options.source || '/backend-api/estuary/content?id=file-synthetic&sig=synthetic' });
    }
    return binaryFetch(url, init);
  };
  return f;
}

test('personal library files resolve ownership then stream the library route without ordinary authorization', async () => {
  for (const project of [false, true]) {
    const f = fixture({ linked: true, project });
    await f.run(f.register()[0]);
    assert.equal(f.calls.length, 2);
    const metadata = new URL(f.calls[0].url);
    assert.equal(metadata.pathname, '/backend-api/files/file-synthetic/simple');
    assert.equal(metadata.searchParams.get('conversation_id'), 'selected');
    assert.equal(metadata.searchParams.get('gizmo_id'), project ? PROJECT : null);
    assert.equal(f.calls[1].url, DOWNLOAD);
    assert.equal(f.calls[1].init.headers, undefined);
    assert.equal(f.calls[1].init.credentials, 'same-origin');
    assert.deepEqual(Buffer.concat(f.stored), Buffer.from(f.data));
    assert.deepEqual(f.receipts, [['download_conversation_file', true, 'download_saved']]);
    assert.doesNotMatch(JSON.stringify({ packets: f.packets, receipts: f.receipts }), /libfile|file-synthetic|Bearer|https:/);
  }
});

test('library ownership resolution uses the immutable selection and accepts an omitted project flag', async () => {
  const f = fixture({ linked: true, info: { file_id: 'file-synthetic', is_library_file: true,
    library_file_id: LIBRARY } });
  const row = f.register()[0];
  f.attachment.id = 'file-changed';
  f.attachment.library_file_id = 'libfile_changed';
  await f.run(row);
  assert.equal(f.calls[1]?.url, DOWNLOAD);
  assert.equal(f.saved, true);
});

test('personal library resolution fails closed before binary transfer when ownership changes or is unconfirmed', async () => {
  for (const patch of [{ library_file_id: 'libfile_other' }, { file_id: 'file-other' },
    { is_library_file: false }, { is_project: 'false' }, { gizmo_id: 'invalid-project' }]) {
    const f = fixture({ linked: true, info: { file_id: 'file-synthetic', is_library_file: true,
      library_file_id: LIBRARY, is_project: false, ...patch } });
    await f.run(f.register()[0]);
    assert.equal(f.calls.length, 1);
    assert.equal(f.saved, false);
    assert.equal(f.packets.some(packet => packet.byteOperation), false);
    assert.equal(f.receipts.at(-1)[1], false);
  }
  for (const onMetadata of [f => f.api.cancel(), f => f.setIdentity('Bearer changed-identity'),
    f => { f.root.location.href += '-changed'; }, f => { f.root.__elonChatGptDocumentToken = 'doc_changed'; }]) {
    const f = fixture({ linked: true, onMetadata });
    await f.run(f.register()[0]);
    assert.equal(f.calls.length, 1);
    assert.equal(f.saved, false);
    assert.equal(f.packets.some(packet => packet.byteOperation), false);
  }
});

test('resolved library transfers keep byte cancellation, storage confirmation and no replay semantics', async () => {
  for (const options of [{ status: 403 }, { failCommit: true }, { noNativeBytes: true },
    { dropAck: 'commit', fastTimeout: true },
    { onPacket: (packet, f) => { if (packet.byteOperation === 'chunk') f.api.cancel(packet.leaseId); } }]) {
    const f = fixture({ linked: true, ...options });
    await f.run(f.register()[0]);
    assert.ok(f.calls.length <= 2);
    assert.equal(f.calls.filter(call => call.url.includes('/backend-api/files/download/')).length, 0);
    assert.equal(f.receipts.length, 1);
    assert.equal(f.receipts[0][1], false);
    assert.equal(f.saved, options.dropAck === 'commit');
  }
});

test('a resolved library download gets a byte-transfer deadline without extending selection reuse', async () => {
  const now = Date.now;
  try {
    const startedAt = now(), timers = [];
    const f = fixture({ linked: true, onPacket: packet => {
      if (packet.byteOperation === 'chunk') Date.now = () => startedAt + 180000;
    } });
    const schedule = f.root.setTimeout;
    f.root.setTimeout = (fn, delay) => { timers.push(delay); return schedule(fn, delay); };
    const row = f.register()[0];
    await f.run(row);
    assert.equal(f.receipts.at(-1)[2], 'download_saved');
    assert.ok(timers.includes(120000));
    await f.run(row);
    assert.equal(f.receipts.at(-1)[2], 'download_selection_expired');
    assert.equal(f.calls.length, 2);
  } finally { Date.now = now; }
});

test('authorized same-origin content reuses native byte publication, never Android identity transfer', async () => {
  for (const source of ['/backend-api/estuary/content?id=file-synthetic&sig=synthetic',
    'https://chatgpt.com/api/estuary/content?id=file-synthetic&sig=synthetic']) {
    const f = contentFixture({ source });
    await f.run(f.register()[0]);
    assert.equal(f.calls.length, 2);
    assert.equal(f.calls[1].url, new URL(source, f.root.location.origin).href);
    assert.equal(f.calls[1].init.credentials, 'same-origin');
    assert.equal(f.calls[1].init.redirect, 'error');
    assert.equal(f.calls[1].init.headers, undefined);
    assert.deepEqual(Buffer.concat(f.stored), Buffer.from(f.data));
    assert.deepEqual(f.receipts, [['download_conversation_file', true, 'download_saved']]);
    assert.doesNotMatch(JSON.stringify({ packets: f.packets, receipts: f.receipts }), /sig=|file-synthetic|Bearer|https:/);
  }
});

test('authorized content is bound to the selected file, document, cancellation and storage acknowledgement', async () => {
  for (const onPacket of [(p, f) => { if (p.byteOperation === 'chunk') f.setIdentity('Bearer changed-account'); },
    (p, f) => { if (p.byteOperation === 'chunk') f.root.__elonChatGptDocumentToken = 'doc_changed'; },
    (p, f) => { if (p.byteOperation === 'chunk') f.api.cancel(p.leaseId); }]) {
    const f = contentFixture({ onPacket });
    await f.run(f.register()[0]);
    assert.equal(f.saved, false);
    assert.equal(f.stored.length, 0);
    assert.equal(f.receipts.at(-1)[2], 'download_cancelled');
  }
  const f = contentFixture({ failCommit: true });
  await f.run(f.register()[0]);
  assert.equal(f.saved, false);
  assert.equal(f.receipts.at(-1)[2], 'download_storage_failed');
});

test('same-origin content permits declared JSON files but never authentication HTML or changed response origins', async () => {
  const json = contentFixture({ mime: 'application/json' });
  await json.run(json.register()[0]);
  assert.equal(json.receipts.at(-1)[2], 'download_saved');
  for (const options of [{ mime: 'text/html' }, { finalUrl: 'https://chatgpt.com/auth/login' },
    { finalUrl: 'https://files.oaiusercontent.com/redirected' }, { status: 403 }, { noNativeBytes: true }]) {
    const f = contentFixture(options);
    await f.run(f.register()[0]);
    assert.equal(f.saved, false);
    assert.equal(f.receipts.at(-1)[1], false);
  }
});

test('same-origin downloads do not admit arbitrary routes, origins, userinfo or normalized hostile URLs', async () => {
  for (const source of ['https://external.test/backend-api/estuary/content', '/auth/login',
    '//chatgpt.com/backend-api/estuary/content', '/backend-api/estuary/content/../conversation',
    'https://user:pass@chatgpt.com/backend-api/estuary/content', '/backend-api/estuary/content#fragment',
    '/backend-api/estuary/content\n?id=file-synthetic', '/backend-api/estuary/content2',
    'https://chatgpt.com:8443/api/estuary/content', 'data:text/plain,fixture']) {
    const f = contentFixture({ source });
    await f.run(f.register()[0]);
    assert.equal(f.calls.length, 1);
    assert.equal(f.packets.some(p => p.byteOperation), false);
    assert.equal(f.receipts.at(-1)[2], 'download_source_unsupported');
  }
});

test('an authorized byte transfer can finish after selection TTL without extending new selections', async () => {
  const now = Date.now;
  try {
    const startedAt = now();
    const f = contentFixture({ onPacket: packet => {
      if (packet.byteOperation === 'chunk') Date.now = () => startedAt + 180000;
    } });
    const row = f.register()[0];
    await f.run(row);
    assert.equal(f.receipts.at(-1)[2], 'download_saved');
    await f.run(row);
    assert.equal(f.receipts.at(-1)[2], 'download_selection_expired');
    assert.equal(f.calls.length, 2);
  } finally { Date.now = now; }
});

test('an unconfirmed authorization never starts cookie content transfer or replays a request', async () => {
  for (const response of [{ status: 'retry' }, { status: 'success', file_id: 'file-another' }]) {
    const f = contentFixture();
    f.root.fetch = async (url, init) => {
      f.calls.push({ url, init });
      return Response.json({ ...response, download_url: '/api/estuary/content?id=file-synthetic' });
    };
    await f.run(f.register()[0]);
    assert.equal(f.calls.length, 1);
    assert.equal(f.packets.some(p => p.byteOperation), false);
    assert.equal(f.receipts.at(-1)[1], false);
  }
});

test('metadata-only shared files reuse the production download byte lease without a DOM or metadata lookup', async () => {
  for (const project of [false, true]) {
    const f = fixture({ shared: true, project });
    const rows = f.register();
    assert.match(rows[0]?.downloadHandle || '', /^download_[a-f0-9]{32}$/);
    f.sharedReference.library_file_id = 'libfile_changed';
    await f.run(rows[0]);
    assert.deepEqual(f.calls.map(call => call.url), [DOWNLOAD]);
    assert.equal(f.calls[0].init.headers, undefined);
    assert.deepEqual(Buffer.concat(f.stored), Buffer.from(f.data));
    assert.deepEqual(f.receipts, [['download_conversation_file', true, 'download_saved']]);
    assert.equal(f.root.location.href, 'https://chatgpt.com/c/current');
    assert.doesNotMatch(JSON.stringify({ rows, packets: f.packets, receipts: f.receipts }), /libfile|display_path|entrypoint/);
  }
});

test('unrecognized shared metadata cannot be silently dispatched to the ordinary download resolver', () => {
  for (const fields of [{ id: 'file-other' }, { source: 'library' }, { context_scopes: [] },
    { library_file_id: '../other' }, { mounted_library_file_id: LIBRARY }]) {
    const f = fixture({ shared: true }); Object.assign(f.sharedReference, fields);
    assert.equal(f.register()[0]?.downloadHandle, undefined);
    assert.equal(f.calls.length, 0);
  }
});

test('shared metadata retains cancellation and identity guards across native byte publication', async () => {
  const f = fixture({ shared: true, onPacket: (packet, owner) => {
    if (packet.byteOperation === 'chunk') owner.setIdentity('Bearer another-library-account');
  } });
  const row = f.register()[0]; assert.ok(row?.downloadHandle);
  await f.run(row);
  assert.equal(f.saved, false); assert.equal(f.stored.length, 0);
  assert.deepEqual(f.receipts, [['download_conversation_file', false, 'download_cancelled']]);
});

for (const sameId of [false, true]) {
  test('production file selection downloads a library reference with bounded native packets: ' + sameId, async () => {
    const f = fixture(); if (sameId) f.attachment.id = LIBRARY;
    const rows = f.register(); assert.match(rows[0].downloadHandle || '', /^download_[a-f0-9]{32}$/);
    await f.run(rows[0]);
    assert.equal(f.calls.length, 1); assert.equal(f.calls[0].url, DOWNLOAD);
    assert.equal(f.calls[0].init.method, 'GET'); assert.equal(f.calls[0].init.credentials, 'same-origin');
    assert.equal(f.calls[0].init.headers, undefined, 'library binary route must not copy bearer headers');
    assert.equal(f.packets.filter(p => p.byteOperation === 'chunk').length, 3);
    assert.deepEqual(Buffer.concat(f.stored), Buffer.from(f.data));
    assert.deepEqual(f.receipts, [['download_conversation_file', true, 'download_saved']]);
    assert.equal(f.saved, true); assert.equal(f.cancelled, false);
    assert.equal(f.root.location.href, 'https://chatgpt.com/c/current');
    const exported = JSON.stringify({ rows, receipts: f.receipts, packets: f.packets });
    assert.ok(!exported.includes(LIBRARY)); assert.ok(!exported.includes('Bearer'));
  });
}

test('acknowledgement backpressure stops the page from queuing the entire file', async () => {
  const f = fixture({ manualAck: true });
  const running = f.run(f.register()[0]);
  async function next() { await new Promise(resolve => setImmediate(resolve)); }
  await next(); assert.equal(f.reads, 0); assert.equal(f.packets.length, 1);
  f.ack(); await next(); assert.equal(f.packets.length, 2);
  await next(); assert.equal(f.packets.length, 2); assert.equal(f.receipts.length, 0);
  f.ack(); await next(); f.ack(); await next(); f.ack(); await next();
  assert.equal(f.packets.at(-1).byteOperation, 'commit');
  assert.equal(f.receipts.length, 0); f.ack(); await running;
  assert.equal(f.receipts[0][2], 'download_saved');
});

for (const options of [{ noLength: true }, { data: new Uint8Array() },
  { encoding: 'gzip', length: 2 }, { finalUrl: 'https://files.oaiusercontent.com/fixture?sig=synthetic' }, { project: true }]) {
  test('evidenced binary route supports streaming metadata without inventing another scope: ' + JSON.stringify(options), async () => {
    const f = fixture(options); await f.run(f.register()[0]);
    assert.equal(f.receipts[0][1], true); assert.equal(f.calls[0].url, DOWNLOAD);
  });
}

test('official library redirects reuse the strict same-origin content validator and existing byte owner', async () => {
  for (const path of ['/backend-api/estuary/content', '/api/estuary/content']) {
    const f = fixture({ finalUrl: 'https://chatgpt.com' + path + '?id=file-synthetic&sig=synthetic' });
    await f.run(f.register()[0]);
    assert.equal(f.calls.length, 1, 'consume the returned body without a second GET');
    assert.equal(f.calls[0].init.headers, undefined);
    assert.equal(f.calls[0].init.credentials, 'same-origin');
    assert.deepEqual(Buffer.concat(f.stored), Buffer.from(f.data));
    assert.deepEqual(f.receipts, [['download_conversation_file', true, 'download_saved']]);
    assert.doesNotMatch(JSON.stringify({ packets: f.packets, receipts: f.receipts }), /https:|sig=|file-synthetic/);
  }
});

test('library redirects cannot broaden the content origin, route or login/body policy', async () => {
  for (const finalUrl of ['https://chatgpt.com.evil.test/backend-api/estuary/content',
    'https://user:pass@chatgpt.com/backend-api/estuary/content',
    'https://chatgpt.com:8443/backend-api/estuary/content',
    'https://chatgpt.com/backend-api/estuary/content#fragment',
    'https://chatgpt.com/backend-api/estuary/content2', 'https://chatgpt.com/auth/login']) {
    const f = fixture({ finalUrl }); await f.run(f.register()[0]);
    assert.equal(f.receipts.at(-1)[2], 'download_source_unsupported');
    assert.equal(f.packets.some(p => p.byteOperation), false);
  }
  for (const mime of ['text/html', 'application/json']) {
    const f = fixture({ finalUrl: 'https://chatgpt.com/backend-api/estuary/content?id=file-synthetic', mime });
    await f.run(f.register()[0]);
    assert.equal(f.receipts.at(-1)[2], 'download_content_invalid');
    assert.equal(f.packets.some(p => p.byteOperation), false);
  }
});

for (const fields of [{ source: 'connector' }, { library_file_id: 'not-a-library-id' },
  { id: 'file-ordinary' }, { id: '' }, { mounted_library_file_id: 'external' },
  { shared_library_file_id: LIBRARY }, { preview_file: { id: 'file-other' } },
  { context_connector_info: {} }, { source_url: 'https://external.example.test/file' }, { context_scopes: ['unknown'] }]) {
  test('unknown and different references are not retargeted to library bytes: ' + JSON.stringify(fields), () => {
    const f = fixture(); Object.assign(f.attachment, fields);
    if (fields.id === 'file-ordinary') {
      assert.equal(library.target(f.attachment), null, 'ordinary library-linked files retain their own resolver');
    } else assert.equal(f.register()[0].downloadHandle, undefined);
    assert.equal(f.calls.length, 0);
  });
}

for (const options of [{ status: 404 }, { status: 403 }, { length: 512 * 1024 * 1024 + 1 },
  { mime: 'text/html' }, { mime: 'application/json' }, { length: 100_010 },
  { length: 100_000 }, { finalUrl: 'https://chatgpt.com/auth/login' },
  { finalUrl: 'https://external.example.test/download' }, { failCommit: true }, { noNativeBytes: true },
  { hangFetch: true, fastTimeout: true }, { dropAck: 'chunk', fastTimeout: true }, { emptyLoop: true }]) {
  test('failed or incomplete downloads never report saved: ' + JSON.stringify(options), async () => {
    const f = fixture(options); await f.run(f.register()[0]);
    assert.equal(f.receipts[0][1], false); assert.equal(f.saved, false);
    assert.equal(f.cancelled, true); assert.equal(f.stored.length, 0);
    assert.ok(f.calls.length <= 1, 'never retry a broader route');
  });
}

test('account, document, route and user cancellation abort before another chunk or publication', async () => {
  for (const mutate of [f => f.setIdentity('Bearer another-account'),
    f => { f.root.__elonChatGptDocumentToken = 'doc_another'; },
    f => { f.root.location.href += '-another'; }, f => f.api.cancel()]) {
    const f = fixture({ onPacket: (packet, owner) => { if (packet.byteOperation === 'chunk') mutate(owner); } });
    await f.run(f.register()[0]);
    assert.equal(f.receipts[0][1], false); assert.equal(f.saved, false); assert.equal(f.stored.length, 0);
    assert.equal(f.packets.filter(p => p.byteOperation === 'chunk').length, 1);
  }
});

test('a selected library ID cannot be replaced by a later mutable history response', async () => {
  const f = fixture(), rows = f.register(); f.attachment.library_file_id = 'libfile_another';
  await f.run(rows[0]); assert.equal(f.calls[0].url, DOWNLOAD); assert.equal(f.saved, true);
});

test('lost final acknowledgement is indeterminate and never repeats the file transfer', async () => {
  const f = fixture({ dropAck: 'commit', fastTimeout: true });
  await f.run(f.register()[0]);
  assert.equal(f.saved, true);
  assert.equal(f.calls.length, 1);
  assert.deepEqual(f.receipts, [['download_conversation_file', false, 'download_confirmation_unknown']]);
  assert.deepEqual(Buffer.concat(f.stored), Buffer.from(f.data));
});

test('native notification cancellation aborts the stream without waiting for the next acknowledgement', async () => {
  const f = fixture({ onPacket: (packet, owner) => {
    if (packet.byteOperation === 'chunk') owner.root.elonChatGptFileDownload.onmessage({ data: JSON.stringify({
      leaseId: packet.leaseId, state: 'cancelled',
    }) });
  } });
  const prior = () => {};
  f.root.elonChatGptFileDownload.onmessage = prior;
  await f.run(f.register()[0]);
  assert.equal(f.receipts[0][2], 'download_cancelled');
  assert.equal(f.saved, false); assert.equal(f.stored.length, 0);
  assert.equal(f.root.elonChatGptFileDownload.onmessage, prior);
});

test('a stale native progress panel cannot cancel another lease', async () => {
  let ignored = 0;
  const f = fixture({ onPacket: (packet, owner) => {
    if (packet.byteOperation === 'chunk') {
      assert.equal(owner.api.cancel('00000000-0000-4000-8000-000000000099'), false);
      ignored++;
    }
  } });
  await f.run(f.register()[0]);
  assert.equal(ignored, 3);
  assert.equal(f.saved, true);
  assert.equal(f.receipts[0][2], 'download_saved');
});

test('request-bound native cancellation aborts preparation before the byte bridge begins', async () => {
  const f = fixture({ hangFetch: true });
  const running = f.run(f.register()[0]);
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(f.calls.length, 1);
  assert.equal(f.api.cancel('00000000-0000-4000-8000-000000000001'), true);
  await running;
  assert.equal(f.calls[0].init.signal.aborted, true);
  assert.equal(f.packets.some(p => p.byteOperation === 'begin'), false);
  assert.deepEqual(f.receipts, [['download_conversation_file', false, 'download_cancelled']]);
  assert.equal(f.api.cancel('00000000-0000-4000-8000-000000000001'), false);
});

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

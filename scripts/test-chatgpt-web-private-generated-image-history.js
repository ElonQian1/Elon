'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const { webcrypto } = require('node:crypto');
const history = require('../android/app/src/main/assets/chatgpt_web_private_history_projection.js');
const download = require('../android/app/src/main/assets/chatgpt_web_private_file_download.js');
const projection = history.create({});
const image = (watermark = true) => ({ content_type: 'image_asset_pointer',
  asset_pointer: 'sediment://file-generated/original#v1?scale=full', width: 1024, height: 1024,
  metadata: watermark ? { watermarked_asset_pointer: 'sediment://file-watermarked/original#v2?scale=full' } : {} });
const message = (id = 'generated-1', asset = image()) => ({ id,
  author: { role: 'tool', name: 't2uay3k.sj1i4kz' },
  content: { content_type: 'multimodal_text', parts: ['internal tool text', asset] },
  metadata: { attachments: [{ id: 'internal-file', name: 'internal.txt' }] } });
const clone = value => JSON.parse(JSON.stringify(value));

function fixture() {
  let paid = false, accountId = 'synthetic-account', authorization = 'Bearer synthetic-image-download';
  let profile = 'web_20260912', accountMissing = false;
  const calls = [], queued = [], cancelled = [], receipts = [];
  const payload = { messages: [message()] };
  const shared = { mq: () => accountMissing ? null : { id: accountId, hasPaidSubscription: () => paid } };
  const bridge = { postMessage(raw) {
    const value = JSON.parse(raw);
    if (value.cancel) { cancelled.push(value); return; }
    queued.push(value);
    queueMicrotask(() => bridge.onmessage?.({ data: JSON.stringify({ leaseId: value.leaseId, state: 'queued' }) }));
  } };
  const root = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/current' },
    __elonChatGptDocumentToken: 'doc_synthetic_generated',
    __elonChatGptPrivateHistoryProjection: history,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: authorization }) },
    __elonChatGptPrivateRuntimeBindings: { state: () => ({ profile_id: profile }), peek: () => shared },
    __elonChatGptPrivateJsonRequest: { request: async (_, url) => {
      calls.push(url);
      return { payload: { status: 'success', download_url: 'https://files.oaiusercontent.com/original?sig=synthetic' } };
    } },
    elonChatGptFileDownload: bridge, AbortController, crypto: webcrypto, setTimeout, clearTimeout };
  const service = download.create(root);
  const rows = (path = '/c/source') => service.register(path, payload, projection.files(payload));
  const descriptor = (path = '/c/source') => {
    const row = rows(path)[0];
    return { version: 1, path, href: root.location.href, documentToken: root.__elonChatGptDocumentToken,
      name: row?.name, downloadHandle: row?.downloadHandle, leaseId: '00000000-0000-4000-8000-000000000001' };
  };
  const run = value => service.start(JSON.stringify(value || descriptor()), (...args) => receipts.push(args));
  return { root, payload, shared, calls, queued, cancelled, receipts, rows, descriptor, run, service,
    paid: value => { paid = value; }, account: value => { accountId = value; },
    profile: value => { profile = value; }, missing: value => { accountMissing = value; },
    authorization: value => { authorization = value; } };
}

test('official generated tool images enter files without exposing tool text or private descriptors', () => {
  const payload = { messages: [message()] };
  assert.deepEqual(projection.files(payload), { files: [{ id: 'generated-1:0', messageId: 'generated-1',
    role: 'assistant', name: '\u56fe\u7247', kind: 'image', mediaType: '' }], truncated: false });
  assert.doesNotMatch(JSON.stringify(projection.files(payload)), /sediment|watermark|internal/);
  assert.deepEqual(projection.project(payload), [], 'ordinary transcript projection is unchanged');
  assert.deepEqual(projection.sourceMessages(payload), [], 'tool internals are not context-source candidates');
  assert.deepEqual(projection.fileSource(payload, 'generated-1:0'), { image: payload.messages[0].content.parts[1],
    name: '\u56fe\u7247', attachments: [], generatedImageHistory: true });
  assert.equal(projection.fileSource(payload, 'generated-1:1'), null);
  payload.messages[0].metadata.attachments = Array.from({ length: 40 }, () => ({ name: 'internal.txt' }));
  assert.equal(projection.files(payload).truncated, false, 'ignored tool metadata is not a missing file page');
});

test('unknown tool names, hidden messages and non-multimodal tools stay out of the index', () => {
  const variants = [
    m => { m.author.name = 'image_gen'; }, m => { m.author.name = 'dalle'; },
    m => { m.author.name = 't2uay3k.other'; }, m => { m.author.role = 'system'; },
    m => { m.content.content_type = 'text'; },
    m => { m.metadata.is_visually_hidden_from_conversation = true; },
    m => { m.metadata.is_visually_hidden_from_conversation = 'hidden'; },
  ];
  for (const change of variants) {
    const m = message(); change(m);
    assert.deepEqual(projection.files({ messages: [m] }).files, []);
    assert.equal(projection.fileSource({ messages: [m] }, 'generated-1:0'), null);
  }
});

test('generated files follow the selected branch and reject ambiguous sources', () => {
  const payload = { current_node: 'chosen', mapping: {
    root: { parent: null }, chosen: { parent: 'root', message: message('chosen') },
    other: { parent: 'root', message: message('other') } } };
  assert.deepEqual(projection.files(payload).files.map(f => f.messageId), ['chosen']);
  assert.equal(projection.fileSource(payload, 'other:0'), null);
  delete payload.current_node;
  assert.equal(projection.files(payload), null);
  assert.equal(projection.fileSource({ messages: [message(), message()] }, 'generated-1:0'), null);
});

test('generated indexing remains bounded and scans beyond the transcript display window', () => {
  const m = message(); m.content.parts = Array.from({ length: 21 }, () => image());
  assert.equal(projection.files({ messages: [m] }).files.length, 20);
  assert.equal(projection.files({ messages: [m] }).truncated, true);
  const messages = [message(), ...Array.from({ length: 90 }, (_, i) => ({ id: 'text-' + i,
    author: { role: 'user' }, content: { parts: ['visible'] } }))];
  assert.equal(projection.files({ messages }).files[0].messageId, 'generated-1');
  const many = Array.from({ length: 6 }, (_, i) => ({ ...clone(m), id: 'many-' + i }));
  assert.equal(projection.files({ messages: many }).files.length, 100);
  assert.equal(projection.files({ messages: many }).truncated, true);
});

test('private generated downloads match paid/free policy without loading image DOM', async () => {
  for (const paid of [false, true]) {
    const f = fixture(); f.paid(paid);
    const path = '/g/g-p-0123456789abcdef0123456789abcdef-synthetic/c/source';
    f.payload.gizmo_id = 'g-p-0123456789abcdef0123456789abcdef';
    const rows = f.rows(path);
    assert.match(rows[0].downloadHandle, /^download_[a-f0-9]{32}$/);
    assert.doesNotMatch(JSON.stringify(rows), /sediment|file-generated|file-watermarked|Bearer/);
    await f.run(f.descriptor(path));
    assert.equal(f.calls.length, 1);
    const url = new URL(f.calls[0]);
    assert.equal(decodeURIComponent(url.pathname), '/backend-api/files/download/' +
      (paid ? 'file-generated/original*v1' : 'file-watermarked/original*v2'));
    assert.deepEqual(Object.fromEntries(url.searchParams), {
      scale: 'full', conversation_id: 'source', inline: 'false', download_intent: 'true' });
    assert.equal(f.root.location.href, 'https://chatgpt.com/c/current');
    assert.equal(f.queued.length, 1);
    assert.deepEqual(f.receipts, [['download_conversation_file', true, 'download_queued']]);
  }
});

test('images without a watermark do not load account policy or require a composer', async () => {
  const f = fixture(); f.payload.messages = [message('plain', image(false))];
  delete f.root.__elonChatGptPrivateRuntimeBindings;
  await f.run();
  assert.equal(f.calls.length, 1);
  assert.match(f.calls[0], /file-generated/);
  assert.equal(f.queued.length, 1);
});

test('unresolved policy is a retryable preparation failure, not website capability absence', async () => {
  for (const alter of [f => f.missing(true), f => f.paid('false'), f => f.account(''),
    f => f.profile('unobserved'), f => { f.shared.mq = () => { throw new Error('unready'); }; }]) {
    const f = fixture(); alter(f);
    assert.match(f.rows()[0].downloadHandle, /^download_/);
    await f.run();
    assert.equal(f.calls.length, 0); assert.equal(f.queued.length, 0);
    assert.deepEqual(f.receipts, [['download_conversation_file', false, 'download_prepare_failed']]);
  }
});

test('placeholder, invalid watermark and conflicting project scopes never receive handles', () => {
  for (const alter of [m => { m.content.parts[1].metadata.is_no_auth_placeholder = true; },
    m => { m.content.parts[1].metadata.watermarked_asset_pointer = 'https://external.test/original'; },
    m => { m.content.parts[1].metadata.watermarked_asset_pointer = 'sediment://file-x?conversation_id=other'; },
    m => { m.content.parts[1].metadata = []; }, m => { m.content.parts[1].library_file_id = 'libfile_other'; }]) {
    const f = fixture(); alter(f.payload.messages[0]);
    assert.equal(f.rows()[0].downloadHandle, undefined);
  }
  const f = fixture(); f.payload.gizmo_id = 'g-p-fedcba9876543210fedcba9876543210';
  assert.equal(f.rows('/g/g-p-0123456789abcdef0123456789abcdef/c/source')[0].downloadHandle, undefined);
});

test('one lazy runtime load resolves policy on download, cancellation prevents a late GET', async () => {
  for (const cancel of [false, true]) {
    const f = fixture(); let resolve, cached = null, loads = 0;
    const runtime = f.root.__elonChatGptPrivateRuntimeBindings;
    runtime.peek = () => cached;
    runtime.load = () => { loads++; return new Promise(r => { resolve = () => { cached = f.shared; r(cached); }; }); };
    const descriptor = f.descriptor();
    assert.equal(loads, 0, 'opening the list does not wait on account modules');
    const pending = f.run(descriptor);
    assert.equal(loads, 1); assert.equal(f.calls.length, 0);
    if (cancel) f.service.cancel(descriptor.leaseId);
    resolve(); await pending;
    assert.equal(f.calls.length, cancel ? 0 : 1);
    assert.equal(f.queued.length, cancel ? 0 : 1);
  }
});

test('account, plan, runtime, route and identity changes during authorization cancel handoff', async () => {
  for (const alter of [f => f.paid(true), f => f.account('different'), f => f.missing(true),
    f => f.profile('different'), f => { f.root.location.href = 'https://chatgpt.com/c/other'; },
    f => f.authorization('Bearer different-synthetic-token')]) {
    const f = fixture(), request = f.root.__elonChatGptPrivateJsonRequest.request;
    f.root.__elonChatGptPrivateJsonRequest.request = async (...args) => {
      const result = await request(...args); alter(f); return result;
    };
    await f.run();
    assert.equal(f.calls.length, 1); assert.equal(f.queued.length, 0);
    assert.deepEqual(f.receipts, [['download_conversation_file', false, 'download_cancelled']]);
  }
});

test('each download rechecks plan and authorization instead of caching a signed URL', async () => {
  const f = fixture(), descriptor = f.descriptor();
  await f.run(descriptor); f.paid(true); await f.run(descriptor);
  assert.equal(f.calls.length, 2); assert.match(f.calls[0], /file-watermarked/); assert.match(f.calls[1], /file-generated/);
  assert.equal(f.queued.length, 2);
});

test('response file identity must match the effective variant, never the discarded base image', async () => {
  for (const fileId of ['file-watermarked/original*v2', 'file-watermarked/original#v2?scale=full',
    'file-generated/original#v1?scale=full', 'unrelated']) {
    const f = fixture(), request = f.root.__elonChatGptPrivateJsonRequest.request;
    f.root.__elonChatGptPrivateJsonRequest.request = async (...args) => {
      const result = await request(...args); result.payload.file_id = fileId; return result;
    };
    await f.run();
    assert.equal(f.queued.length, fileId.startsWith('file-watermarked') ? 1 : 0);
  }
});

test('same-origin originals reuse the native byte owner with the selected image and live guard', async () => {
  const f = fixture(); let transferred = false;
  f.root.__elonChatGptPrivateLibraryDownload = {
    contentUrl: () => true,
    runContent: async (_, job, current) => {
      transferred = true;
      assert.match(job.entry.fileId, /^file-watermarked/); assert.equal(current(job), true);
      f.paid(true); assert.equal(current(job), false);
      throw new Error('download_cancelled');
    },
  };
  await f.run();
  assert.equal(transferred, true); assert.equal(f.queued.length, 0);
  assert.equal(f.receipts[0][2], 'download_cancelled');
});

test('generated policy is assembled after pointer parser and before the download owner', () => {
  const source = require('./chatgpt-web-adapter-assembly').readAdapterSource();
  const policy = source.indexOf('chatgpt_web_private_generated_image_download.js');
  assert.ok(policy > source.indexOf('chatgpt_web_private_image_pointer.js'));
  assert.ok(policy < source.indexOf('chatgpt_web_private_file_download.js'));
});

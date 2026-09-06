'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const { webcrypto } = require('node:crypto');
const projection = require('../android/app/src/main/assets/chatgpt_web_private_history_projection.js');
const download = require('../android/app/src/main/assets/chatgpt_web_private_file_download.js');
const request = require('../android/app/src/main/assets/chatgpt_web_private_json_request.js');

const PROJECT = 'g-p-0123456789abcdef0123456789abcdef';
const LIBRARY = 'libfile_synthetic_image';
const ID = 'file-synthetic-image';
const SIGNED = 'https://files.oaiusercontent.com/image?sig=synthetic';

function fixture(pointer = 'sediment://' + ID) {
  let account = 'Bearer synthetic-image-download';
  const calls = [], queued = [], receipts = [];
  const image = { content_type: 'image_asset_pointer', asset_pointer: pointer, width: 100, height: 100 };
  const message = { id: 'message-image', author: { role: 'user' },
    content: { parts: ['fixture', image] }, metadata: { attachments: [] } };
  const payload = { messages: [message] };
  const bridge = { onmessage: null, postMessage(raw) {
    const value = JSON.parse(raw);
    if (value.cancel) return;
    queued.push(value);
    queueMicrotask(() => bridge.onmessage?.({ data: JSON.stringify({ leaseId: value.leaseId, state: 'queued' }) }));
  } };
  const root = {
    location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/current' },
    __elonChatGptDocumentToken: 'doc_image_1',
    __elonChatGptPrivateHistoryProjection: projection,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: account }) },
    __elonChatGptPrivateJsonRequest: request, elonChatGptFileDownload: bridge,
    AbortController, crypto: webcrypto, setTimeout, clearTimeout,
    fetch: async (url, init) => {
      calls.push({ url: new URL(url), init });
      return Response.json({ status: 'success', file_id: ID, download_url: SIGNED });
    },
  };
  const service = download.create(root);
  const register = (path = '/c/source') => service.register(path, payload, projection.create({}).files(payload));
  const descriptor = (rows = register(), path = '/c/source') => ({ version: 1,
    leaseId: '00000000-0000-4000-8000-000000000001', documentToken: root.__elonChatGptDocumentToken,
    href: root.location.href, path, name: rows[0].name, downloadHandle: rows[0]?.downloadHandle });
  const run = value => service.start(JSON.stringify(value || descriptor()), (...args) => receipts.push(args));
  return { root, calls, queued, receipts, payload, image, message, service, register, descriptor, run,
    setAccount: value => { account = value; } };
}

test('conversation image pointers reuse the official scoped file download and native lease', async () => {
  for (const scheme of ['file-service', 'sediment']) {
    for (const role of ['user', 'assistant']) {
      const f = fixture(scheme + '://' + ID);
      f.message.author.role = role;
      const rows = f.register(), original = f.root.location.href;
      assert.match(rows[0].downloadHandle || '', /^download_[a-f0-9]{32}$/);
      assert.equal(rows[0].name, 'image.png');
      assert.equal(rows[0].kind, 'image');
      assert.equal(rows[0].mediaType, '');
      assert.ok(!JSON.stringify(rows).includes(ID));
      assert.ok(!JSON.stringify(rows).includes(scheme + '://'));
      await f.run(f.descriptor(rows));
      assert.equal(f.calls.length, 1);
      const { url, init } = f.calls[0];
      assert.equal(url.pathname, '/backend-api/files/download/' + ID);
      assert.deepEqual([...url.searchParams], [
        ['check_context_scopes_for_conversation_id', 'source'], ['download_intent', 'true'],
      ]);
      assert.equal(init.method, 'GET');
      assert.equal(init.redirect, 'error');
      assert.equal(f.queued.length, 1);
      assert.deepEqual(Object.keys(f.queued[0]).sort(), ['documentToken', 'leaseId', 'url']);
      assert.deepEqual(f.receipts, [['download_conversation_file', true, 'download_queued']]);
      assert.equal(f.root.location.href, original);
    }
  }
});

test('matching image metadata preserves the original filename and library project resolution', async () => {
  const f = fixture();
  f.message.metadata.attachments.push({ id: ID, name: 'photo.jpg', mime_type: 'image/jpeg', library_file_id: LIBRARY });
  f.payload.gizmo_id = PROJECT;
  const fetch = f.root.fetch;
  f.root.fetch = async (url, init) => {
    if (new URL(url).pathname.endsWith('/simple')) {
      f.calls.push({ url: new URL(url), init });
      return Response.json({ file_id: ID, is_library_file: true, library_file_id: LIBRARY, is_project: false });
    }
    return fetch(url, init);
  };
  const path = '/g/' + PROJECT + '-test/c/source', rows = f.register(path);
  assert.equal(rows[0].name, 'photo.jpg');
  assert.equal(rows[0].mediaType, 'image/jpeg');
  await f.run(f.descriptor(rows, path));
  assert.equal(f.calls.length, 2);
  assert.equal(f.calls[0].url.pathname, '/backend-api/files/' + ID + '/simple');
  assert.equal(f.calls[0].url.searchParams.get('gizmo_id'), PROJECT);
  assert.equal(f.calls[1].url.searchParams.has('gizmo_id'), false);
  assert.equal(f.calls[1].url.searchParams.get('check_context_scopes_for_conversation_id'), 'source');
  assert.equal(f.queued.length, 1);
});

test('ordinary project image pointers keep project and conversation scope together', async () => {
  for (const path of ['/c/source', '/g/' + PROJECT + '/c/source']) {
    const f = fixture();
    f.payload.gizmo_id = PROJECT;
    await f.run(f.descriptor(f.register(path), path));
    assert.equal(f.calls.length, 1);
    assert.equal(f.calls[0].url.searchParams.get('gizmo_id'), PROJECT);
    assert.equal(f.calls[0].url.searchParams.get('check_context_scopes_for_conversation_id'), 'source');
    assert.equal(f.queued.length, 1);
  }
});

test('unknown pointers and shared or connector metadata cannot get an ordinary image download handle', () => {
  for (const pointer of ['https://other.test/image', 'data:image/png;base64,AA', 'file-service://../file',
    'sediment://' + ID + '?gizmo_id=other', 'sediment://' + ID + '#page=1',
    'sediment://' + ID + '/child', 'sediment://', ['sediment://' + ID]]) {
    const f = fixture(pointer);
    assert.equal(f.register()[0].downloadHandle, undefined);
  }
  for (const fields of [{ shared_library_file_id: LIBRARY }, { library_download_id: LIBRARY },
    { context_connector_info: { source_url: 'https://other.test/image' } }, { connector_id: 'test' },
    { library_file_id: 'unknown' }, { mime_type: 'text/html' }, { context_scopes: ['HEALTH'] }]) {
    const f = fixture();
    f.message.metadata.attachments.push({ id: ID, name: 'fixture.png', ...fields });
    assert.equal(f.register()[0].downloadHandle, undefined, JSON.stringify(fields));
  }
});

test('truncated, duplicate or contradictory image metadata cannot silently lose its scope', () => {
  for (const mutate of [
    f => { f.message.metadata.attachments = [{ id: ID }, { id: ID }]; },
    f => { f.message.metadata.attachments = Array.from({ length: 21 }, (_, i) => ({ id: 'file-' + i })); },
    f => { f.message.metadata.attachments = {}; },
    f => { f.image.library_file_id = LIBRARY; },
    f => { f.image.gizmo_id = PROJECT; },
    f => { f.image.context_scopes = ['HEALTH']; },
    f => { f.image.context_connector = {}; },
    f => { f.payload.context_scopes = ['HEALTH']; },
  ]) {
    const f = fixture();
    mutate(f);
    assert.equal(f.register()[0].downloadHandle, undefined);
  }
});

test('image descriptors are private and selected-branch bound, not added to ordinary history messages', () => {
  const f = fixture(), p = projection.create({});
  const plain = JSON.stringify({ messages: p.project(f.payload), index: p.files(f.payload) });
  assert.ok(!plain.includes(ID));
  assert.ok(!plain.includes('asset_pointer'));
  assert.ok(!plain.includes('download_'));
  const other = { id: 'message-other', author: { role: 'user' }, content: { parts: [f.image] } };
  f.payload.messages = undefined;
  f.payload.mapping = {
    start: { parent: null }, selected: { parent: 'start', message: f.message },
    alternate: { parent: 'start', message: other },
  };
  f.payload.current_node = 'selected';
  assert.equal(f.register().length, 1);
  assert.equal(p.fileSource(f.payload, 'message-other:0'), null);
  assert.equal(p.fileSource(f.payload, 'message-image:0').image.asset_pointer, 'sediment://' + ID);
});

test('image selection snapshots cannot be retargeted after a history response changes', async () => {
  const f = fixture(), value = f.descriptor();
  f.image.asset_pointer = 'sediment://file-other';
  f.message.metadata.attachments.push({ id: ID, shared_library_file_id: LIBRARY });
  await f.run(value);
  assert.equal(f.calls[0].url.pathname, '/backend-api/files/download/' + ID);
  assert.equal(f.queued.length, 1);
});

test('cancellation and late account, document or route changes block image download handoff', async () => {
  for (const mutate of [f => f.service.cancel(), f => f.setAccount('Bearer changed-account'),
    f => { f.root.location.href += '-other'; }, f => { f.root.__elonChatGptDocumentToken = 'doc_changed_2'; }]) {
    const f = fixture(), fetch = f.root.fetch;
    f.root.fetch = async (...args) => { const result = await fetch(...args); mutate(f); return result; };
    await f.run();
    assert.equal(f.calls.length, 1);
    assert.equal(f.queued.length, 0);
    assert.equal(f.receipts[0][1], false);
  }
});

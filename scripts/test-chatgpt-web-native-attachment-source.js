'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const sourceModule = require('../android/app/src/main/assets/chatgpt_web_native_attachment_source.js');

function fixture(size = 140000, override) {
  const bytes = Buffer.alloc(size, 47), requests = [];
  const descriptor = { version: 1, leaseId: '00000000-0000-4000-8000-000000000000',
    documentToken: 'doc_synthetic_1', href: 'https://chatgpt.com/', name: 'fixture.txt', size, type: 'text/plain' };
  const bridge = { onmessage: null, postMessage: raw => {
    const request = JSON.parse(raw);
    requests.push(request);
    const response = { requestId: request.requestId, offset: request.offset,
      data: bytes.subarray(request.offset, request.offset + 64 * 1024).toString('base64') };
    if (override) override(response, root, request);
    queueMicrotask(() => bridge.onmessage?.({ data: JSON.stringify(response) }));
  } };
  const root = { location: { href: descriptor.href }, __elonChatGptDocumentToken: descriptor.documentToken,
    elonChatGptAttachmentSource: bridge, atob, File, setTimeout, clearTimeout };
  return { root, descriptor, bytes, requests, source: sourceModule.create(root) };
}

test('native bytes cross the bridge in bounded sequential chunks without file paths or auth', async () => {
  const f = fixture();
  const file = await f.source.read(f.descriptor);
  assert.deepEqual(Buffer.from(await file.arrayBuffer()), f.bytes);
  assert.deepEqual(f.requests.map(item => item.offset), [0, 65536, 131072]);
  assert.equal(f.requests.every(item => Object.keys(item).length === 4), true);
  assert.equal(f.root.elonChatGptAttachmentSource.onmessage, null);
});

test('invalid sizes and MIME types are rejected without native reads', async () => {
  for (const patch of [{ size: 0 }, { size: 8 * 1024 * 1024 + 1 }, { type: 'image/svg+xml' }, { version: 2 }]) {
    const f = fixture();
    await assert.rejects(f.source.read({ ...f.descriptor, ...patch }));
    assert.equal(f.requests.length, 0);
  }
});

test('native normalized image bytes use the same bounded handoff as text', async () => {
  for (const type of ['image/jpeg', 'image/png', 'image/webp']) {
    const f = fixture(140000);
    const file = await f.source.read({ ...f.descriptor, type });
    assert.equal(file.type, type);
    assert.deepEqual(Buffer.from(await file.arrayBuffer()), f.bytes);
    assert.equal(f.requests.length, 3);
  }
});

test('PDF binary bytes retain their MIME type and every chunk boundary without text conversion', async () => {
  const f = fixture(140000);
  f.bytes.set(Buffer.from('%PDF-1.7\n'));
  f.bytes.set([0, 255, 128, 13, 10], 65534);
  const file = await f.source.read({ ...f.descriptor, name: 'fixture.pdf', type: 'application/pdf' });
  assert.equal(file.name, 'fixture.pdf');
  assert.equal(file.type, 'application/pdf');
  assert.deepEqual(Buffer.from(await file.arrayBuffer()), f.bytes);
  assert.deepEqual(f.requests.map(item => item.offset), [0, 65536, 131072]);
  assert.equal(f.root.elonChatGptAttachmentSource.onmessage, null);
});

test('a short chunk, incorrect offset or expired lease is never made into a file', async () => {
  for (const mutate of [response => { response.data = ''; }, response => { response.offset++; },
    response => { response.code = 'attachment_read_expired'; }]) {
    const f = fixture(140000, mutate);
    await assert.rejects(f.source.read(f.descriptor));
    assert.equal(f.requests.length, 1);
  }
});

test('document replacement interrupts the byte handoff before the next chunk', async () => {
  const f = fixture(140000, (_, root) => { root.__elonChatGptDocumentToken = 'doc_replaced_2'; });
  await assert.rejects(f.source.read(f.descriptor));
  assert.equal(f.requests.length, 1);
});

test('cancellation restores the prior reply handler and does not consume more bytes', async () => {
  const controller = new AbortController();
  const f = fixture(140000, () => controller.abort());
  const prior = () => {};
  f.root.elonChatGptAttachmentSource.onmessage = prior;
  await assert.rejects(f.source.read(f.descriptor, controller.signal));
  assert.equal(f.requests.length, 1);
  assert.equal(f.root.elonChatGptAttachmentSource.onmessage, prior);
});

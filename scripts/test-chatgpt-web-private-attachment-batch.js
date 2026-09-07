'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fixture = require('./fixtures/chatgpt-attachment-composer.js');
const sender = require('../android/app/src/main/assets/chatgpt_web_private_attachment_send.js');
const transport = require('../android/app/src/main/assets/chatgpt_web_private_attachment_transport.js');
const protocol = require('../android/app/src/main/assets/chatgpt_web_private_attachment_protocol.js');

function batch(count = 3, options = {}) {
  const f = fixture();
  if (options.temporary) f.root.location.href += '?temporary-chat=true';
  const files = Array.from({ length: count }, (_, i) => new File(['synthetic-' + i],
    'fixture-' + i + '.txt', { type: i % 2 ? 'text/markdown' : 'text/plain' }));
  const descriptors = files.map((file, i) => ({ ...f.descriptor, href: f.root.location.href,
    name: file.name, size: file.size, type: file.type,
    leaseId: '00000000-0000-4000-8000-' + String(i).padStart(12, '0'), uploadCopy: i === 1 }));
  const envelope = { version: 2, documentToken: f.root.__elonChatGptDocumentToken,
    href: f.root.location.href, files: descriptors };
  const requests = [], reads = [], receipts = [], changes = [], contexts = [];
  let fallback = 0, activeReads = 0, maxReads = 0, uploads = 0, fileId;
  const api = sender.create(f.root, {
    composer: f.composer,
    source: { read: async descriptor => {
      activeReads++; maxReads = Math.max(maxReads, activeReads);
      reads.push(descriptor);
      assert.equal(f.store.files$().length, 0, 'no intermediate ready state can release text send');
      if (options.read) await options.read(descriptor, f, api);
      activeReads--;
      return files[descriptors.indexOf(descriptor) < 0
        ? descriptors.findIndex(item => item.leaseId === descriptor.leaseId) : descriptors.indexOf(descriptor)];
    } },
    createTransport: config => {
      const actual = transport.create(f.root, { ...config, protocol,
        acquireHeaders: f.root.__elonChatGptPrivateTransport.acquireSameOriginRequestHeaders,
        request: async (_, url, init) => {
          requests.push({ url, init });
          if (url.endsWith('/files')) {
            uploads++;
            if (options.failAt === uploads) throw Error('synthetic network failure');
            fileId = 'file-batch-' + uploads;
            return { payload: { status: 'success', file_id: fileId,
              upload_url: 'https://uploads.oaiusercontent.com/' + fileId + '?sig=synthetic' } };
          }
          if (url.endsWith('/process_upload_stream')) return { text: JSON.stringify({ file_id: fileId,
            event: 'file.processing.completed', progress: 100 }) };
          return {};
        } });
      return { ...actual, upload: (...args) => { contexts.push(args[1]); return actual.upload(...args); } };
    },
  });
  return { ...f, api, files, descriptors, envelope, requests, reads, receipts, changes, contexts,
    maxReads: () => maxReads, fallback: () => fallback,
    start: () => api.start(JSON.stringify(envelope), (...args) => receipts.push(args),
      value => changes.push(value), () => { fallback++; }) };
}

for (const count of [2, 9]) {
  test('complete private batch is uploaded sequentially and associated once: ' + count, async () => {
    const f = batch(count); await f.start();
    assert.equal(f.reads.length, count); assert.equal(f.maxReads(), 1);
    assert.equal(f.requests.length, 3 * count);
    assert.equal(f.fallback(), 0);
    assert.deepEqual(f.receipts, [['request_attachment_upload', true, 'private_attachment_associated']]);
    assert.deepEqual(f.changes, [true]);
    const rows = f.composer.merge([]);
    assert.equal(rows.length, count);
    assert.deepEqual(rows.map(row => row.name), f.files.map(file => file.name));
    assert.equal(f.contexts[1].checkForReusableLibraryFile, false);
    assert.equal(f.contexts[0].checkForReusableLibraryFile, undefined);
    assert.equal(f.requests.some(request => request.url.includes('/conversation')), false);
    const submit = f.api.prepareSubmit(f.store);
    assert.equal(submit.readyFiles.length, count); assert.equal(submit.current(), true);
    assert.deepEqual(submit.readyFiles.map(item => item.file), f.files);
    assert.ok(submit.readyFiles.every(item => Object.isFrozen(item) && Object.isFrozen(item.fileSpec)));
    assert.equal(submit.consumeAccepted(), true);
    assert.deepEqual(f.store.files$(), []);
    assert.equal(submit.consumeAccepted(), false);
  });
}

test('a batch can remove one selected attachment and submit the remaining exact owned files', async () => {
  const f = batch(); await f.start();
  const old = f.api.prepareSubmit(f.store), rows = f.composer.merge([]);
  assert.equal(f.composer.remove(rows[1].id), true);
  assert.equal(old.current(), false);
  const next = f.api.prepareSubmit(f.store);
  assert.deepEqual(next.readyFiles.map(item => item.file.name), [f.files[0].name, f.files[2].name]);
  const later = { status: 'ready', fileId: 'foreign' };
  f.store.files$.set([...f.store.files$(), later]);
  assert.equal(next.current(), false);
  assert.equal(next.consumeAccepted(), true);
  assert.deepEqual(f.store.files$(), [later], 'accepted cleanup touches only its owned batch');
});

test('a second-file failure cannot send a partial batch, replay through DOM, or claim completion', async () => {
  const f = batch(3, { failAt: 2 }); await f.start();
  assert.equal(f.requests.length, 4);
  assert.equal(f.reads.length, 2);
  assert.equal(f.fallback(), 0);
  assert.equal(f.receipts.length, 1); assert.equal(f.receipts[0][1], false);
  assert.deepEqual(f.changes, []); assert.deepEqual(f.store.files$(), []);
  assert.equal(f.api.prepareSubmit(f.store), null);
});

test('cancellation or identity changes before the next file prevent further writes and association', async () => {
  for (const change of [(f, api) => api.cancel(), f => f.setAccount('Bearer another-account')]) {
    let count = 0;
    const f = batch(3, { read: (_, owner, api) => { if (++count === 2) change(owner, api); } });
    await f.start();
    assert.equal(f.requests.length, 3);
    assert.equal(f.fallback(), 0); assert.equal(f.receipts[0][1], false);
    assert.deepEqual(f.store.files$(), []);
  }
});

test('every temporary batch file retains the same non-library privacy context', async () => {
  const f = batch(3, { temporary: true }); await f.start();
  assert.equal(f.receipts[0][1], true);
  const creates = f.requests.filter(request => request.url.endsWith('/files'));
  assert.ok(creates.every(request => JSON.parse(request.init.body).store_in_library === false));
  assert.ok(f.store.files$().every(item => item.isTemporaryChat === true && item.storeInLibrary === false));
});

test('malformed, oversized, duplicate or mixed-document envelopes cannot touch any byte or private request', async () => {
  for (const change of [f => { f.envelope.files = []; }, f => { f.envelope.files = new Array(10).fill(f.descriptors[0]); },
    f => { f.descriptors[1].leaseId = f.descriptors[0].leaseId; },
    f => { f.descriptors[1].documentToken = 'doc_other'; },
    f => { f.descriptors[1].href += 'other'; }, f => { f.descriptors[1].uploadCopy = 'yes'; },
    f => { f.descriptors[1].size = 0; }, f => { f.descriptors[1].size = 8 * 1024 * 1024 + 1; },
    f => { f.descriptors[1].size = '123'; }, f => { f.descriptors[1].name = '../invalid.txt'; },
    f => { f.descriptors[1].name = ''; }, f => { f.descriptors[1].name = 'x'.repeat(121); },
    f => { f.descriptors[1].type = 'video/mp4'; },
    f => { f.envelope.files = {}; }]) {
    const f = batch(); change(f); await f.start();
    assert.equal(f.reads.length, 0); assert.equal(f.requests.length, 0);
    assert.equal(f.fallback(), 0); assert.equal(f.receipts[0][1], false);
  }
});

test('association validates every result before modifying the official file store', () => {
  const f = batch(), binding = f.composer.capture();
  const completed = f.files.map((file, i) => ({ file, leaseId: f.descriptors[i].leaseId,
    result: { ...f.result(binding), fileName: file.name, mimeType: file.type, fileSize: file.size } }));
  completed[1].result.fileName = 'wrong.txt';
  assert.throws(() => f.composer.associateMany(binding, completed));
  assert.deepEqual(f.store.files$(), []);
});

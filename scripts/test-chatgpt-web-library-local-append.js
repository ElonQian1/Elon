'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const setup = require('./fixtures/chatgpt-library-append');
const senderModule = require('../android/app/src/main/assets/chatgpt_web_private_attachment_send');

function fixture(options = {}) {
  const f = setup(), uploads = [], reads = [], replies = [], changes = [];
  const conversation = { attachmentBaseLimit: type => type === 1 ? 4 : 10,
    attachmentMaxUploads: type => options.remaining ?? (type === 1 ? 4 : 10),
    attachmentPendingCount: files => files.filter(item => item.isBigPaste !== true &&
      item.mountedLibraryFileId == null && item.sharedLibraryFileReference == null).length,
    attachmentConfiguredLimit: () => options.perTurn ?? 10 };
  const shared = { attachmentUploadType: { Multimodal: 1, Interpreter: 2 } };
  const cached = { conversation, shared, composer: f.namespace };
  f.root.__elonChatGptPrivateRuntimeBindings.peek = role => cached[role];
  f.root.__elonChatGptPrivateRuntimeBindings.load = async role => cached[role];
  f.props.isFileUploadEnabled = true;
  let fallbacks = 0, attempt = 0;
  const sender = senderModule.create(f.root, { composer: f.composer,
    source: { read: async descriptor => {
      reads.push(descriptor); await options.onRead?.(f);
      return new File(['synthetic bytes'], descriptor.name, { type: descriptor.type });
    } },
    image: { available: () => true, prepare: async file => ({ file, dimensions: { width: 8, height: 8 } }) },
    createTransport: config => ({ cancel() {}, dispose() {}, async upload(file, context, binding) {
      uploads.push({ file, context, binding });
      await options.onUpload?.(f, uploads.length);
      if (options.failAt === uploads.length) throw Error('synthetic_upload_failure');
      assert.equal(config.isCurrent(binding), !options.invalidateUpload);
      return { ...f.result(binding), fileId: 'file-picked-' + uploads.length,
        fileName: file.name, mimeType: file.type, fileSize: file.size,
        ...(/^image\//.test(file.type) ? { imageDimensions: { width: 8, height: 8 } } : {}),
        ...options.result?.(binding) };
    } }) });
  async function start(count = 1, type = 'text/plain') {
    const files = Array.from({ length: count }, (_, i) => ({ ...f.descriptor, type,
      name: 'picked-' + i + (type.startsWith('image/') ? '.png' : '.txt'),
      leaseId: '00000000-0000-4000-8000-' + (++attempt).toString().padStart(12, '0') }));
    const descriptor = count === 1 ? files[0] : { version: 2, href: f.root.location.href,
      documentToken: f.root.__elonChatGptDocumentToken, files };
    await sender.start(JSON.stringify(descriptor), (...args) => replies.push(args),
      value => changes.push(value), () => { fallbacks++; });
  }
  return { ...f, sender, start, uploads, reads, replies, changes, cached, conversation,
    fallbacks: () => fallbacks };
}

test('Library then a local batch keeps exact original entries and one complete submit lease', async () => {
  const f = fixture(); await f.attach(f.source(1));
  const original = f.store.files$()[0], originalId = f.sender.merge([])[0].id;
  await f.start(2);
  assert.equal(f.fallbacks(), 0);
  assert.equal(f.replies.at(-1)?.[1], true);
  assert.equal(f.uploads.length, 2);
  assert.equal(f.store.files$().length, 3);
  assert.equal(f.store.files$()[0], original);
  assert.equal(f.sender.merge([])[0].id, originalId);
  assert.equal(new Set(f.sender.merge([]).map(item => item.id)).size, 3);
  const lease = f.sender.prepareSubmit(f.store);
  assert.equal(lease.readyFiles.length, 3);
  assert.equal(lease.current(), true);
  assert.equal(lease.consumeAccepted(), true);
  assert.equal(f.store.files$().length, 0);
  assert.deepEqual(f.changes, [true]);
});

for (const change of ['remove', 'model', 'account', 'document', 'route', 'foreign']) {
  test('pending local append cannot replace attachments after ' + change, async () => {
    const f = fixture({ onRead(state) {
      if (change === 'remove') state.composer.remove(state.composer.merge([])[0].id);
      if (change === 'model') state.setModel('different-model');
      if (change === 'account') state.setAccount('Bearer different-synthetic-account');
      if (change === 'document') state.root.__elonChatGptDocumentToken = 'doc_changed';
      if (change === 'route') state.root.location.href = 'https://chatgpt.com/c/00000000-0000-4000-8000-000000000000';
      if (change === 'foreign') state.store.files$.set([...state.store.files$(), { status: 'ready', fileId: 'foreign' }]);
    } });
    await f.attach(f.source(1)); const original = f.store.files$()[0];
    await f.start();
    assert.equal(f.fallbacks(), 0); assert.equal(f.uploads.length, 0);
    assert.equal(f.replies.at(-1)?.[1], false);
    assert.deepEqual(f.store.files$(), change === 'remove' ? [] : change === 'foreign'
      ? [original, { status: 'ready', fileId: 'foreign' }] : [original]);
  });
}

test('partial batch failure preserves Library attachment without publishing the first upload', async () => {
  const f = fixture({ failAt: 2 }); await f.attach(f.source(1));
  const original = f.store.files$()[0]; await f.start(2);
  assert.equal(f.uploads.length, 2); assert.equal(f.fallbacks(), 0);
  assert.deepEqual(f.store.files$(), [original]); assert.equal(f.replies.at(-1)?.[1], false);
});

for (const options of [{ remaining: 2 }, { perTurn: 2 }]) {
  test('whole batch respects current official quota ' + JSON.stringify(options), async () => {
    const f = fixture(options); await f.attach(f.source(1)); await f.start(2);
    assert.equal(f.fallbacks(), 0); assert.equal(f.reads.length, 0);
    assert.equal(f.replies.at(-1)?.[1], false); assert.equal(f.store.files$().length, 1);
  });
}

test('image-specific limit counts already staged Library entries', async () => {
  const f = fixture(); await f.attach(f.source(1)); await f.start(4, 'image/png');
  assert.equal(f.reads.length, 0); assert.equal(f.replies.at(-1)?.[1], false);
  assert.equal(f.store.files$().length, 1);
});

test('unknown quota runtime keeps prior attachments and does not fall through to another picker', async () => {
  const f = fixture(); await f.attach(f.source(1)); delete f.cached.shared.attachmentUploadType;
  await f.start(); assert.equal(f.fallbacks(), 0); assert.equal(f.reads.length, 0);
  assert.equal(f.replies.at(-1)?.[1], false); assert.equal(f.store.files$().length, 1);
});

test('reuse of an already selected backing file is an idempotent selection', async () => {
  const f = fixture({ result: () => ({ stage: 'reused', fileId: 'file-1', reusedFileName: 'fixture-1.txt',
    metadata: { libraryPersistenceResult: 'library', libraryFileId: 'libfile_1' } }) });
  await f.attach(f.source(1)); const original = f.store.files$()[0]; await f.start();
  assert.equal(f.replies.at(-1)?.[1], true); assert.equal(f.fallbacks(), 0);
  assert.deepEqual(f.store.files$(), [original]);
});

test('local upload appended repeatedly retains earlier local and Library ownership', async () => {
  const f = fixture(); await f.attach(f.source(1)); await f.start(); await f.start();
  assert.equal(f.replies.filter(row => row[1]).length, 2);
  assert.equal(f.fallbacks(), 0); assert.equal(f.sender.prepareSubmit(f.store).readyFiles.length, 3);
});

test('late upload completion after removal cannot republish the old collection', async () => {
  const f = fixture({ invalidateUpload: true, onUpload(state) {
    state.composer.remove(state.composer.merge([])[0].id);
  } });
  await f.attach(f.source(1)); await f.start();
  assert.equal(f.replies.at(-1)?.[1], false); assert.equal(f.fallbacks(), 0);
  assert.deepEqual(f.store.files$(), []); assert.equal(f.uploads.length, 1);
});

test('explicit cancel during byte read preserves prior selection and sends no upload', async () => {
  let release;
  const f = fixture({ onRead: () => new Promise(resolve => { release = resolve; }) });
  await f.attach(f.source(1)); const original = f.store.files$()[0];
  const pending = f.start();
  while (!release) await new Promise(resolve => setImmediate(resolve));
  f.sender.cancel(); release(); await pending;
  assert.equal(f.uploads.length, 0); assert.equal(f.replies.at(-1)?.[1], false);
  assert.equal(f.fallbacks(), 0); assert.deepEqual(f.store.files$(), [original]);
});

test('quota changes during batch upload prevent atomic publication', async () => {
  const f = fixture({ onUpload() { f.conversation.attachmentMaxUploads = () => 1; } });
  await f.attach(f.source(1)); const original = f.store.files$()[0]; await f.start(2);
  assert.equal(f.uploads.length, 1); assert.equal(f.replies.at(-1)?.[1], false);
  assert.deepEqual(f.store.files$(), [original]); assert.equal(f.fallbacks(), 0);
});

test('a new append invalidates the old submit lease while preserving all file identity', async () => {
  const f = fixture(); await f.attach(f.source(1));
  const old = f.sender.prepareSubmit(f.store); await f.start();
  assert.equal(old.current(), false);
  const next = f.sender.prepareSubmit(f.store);
  assert.equal(next.readyFiles.length, 2); assert.equal(next.readyFiles[0].file, old.readyFiles[0].file);
});

test('unlimited banner props do not skip official image quota or invent zero capacity', async () => {
  const f = fixture(); await f.attach(f.source(1));
  f.props.maxLibraryAttachmentCount = undefined; f.props.maxTotalLibraryAttachmentCount = undefined;
  await f.start(); assert.equal(f.replies.at(-1)?.[1], true);
  await f.start(3, 'image/png'); assert.equal(f.replies.at(-1)?.[1], false);
  assert.equal(f.store.files$().length, 2);
});

test('a contradictory processed backing ID is not treated as reusable', async () => {
  const f = fixture({ result: () => ({ fileId: 'file-1' }) });
  await f.attach(f.source(1)); const original = f.store.files$()[0]; await f.start();
  assert.equal(f.replies.at(-1)?.[1], false); assert.deepEqual(f.store.files$(), [original]);
});

test('all-or-nothing append readback only rolls back newly added entries', async () => {
  const f = fixture(); await f.attach(f.source(1)); const original = f.store.files$()[0];
  const external = { fileId: 'foreign', status: 'ready' }, write = f.store.files$.set;
  f.store.files$.set = files => write(files.length > 1 && !files.includes(external) ? [...files, external] : files);
  await f.start(); assert.equal(f.replies.at(-1)?.[1], false);
  assert.deepEqual(f.store.files$(), [original, external]);
});

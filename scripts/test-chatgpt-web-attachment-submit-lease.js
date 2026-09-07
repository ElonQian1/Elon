'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const composerModule = require('../android/app/src/main/assets/chatgpt_web_private_attachment_composer.js');
const senderModule = require('../android/app/src/main/assets/chatgpt_web_private_attachment_send.js');
const protocol = require('../android/app/src/main/assets/chatgpt_web_private_attachment_protocol.js');

function fixture({ temporary = false, image = false, reused = false } = {}) {
  let values = [], identity = 'Bearer synthetic-only', model = 'synthetic-model';
  const files$ = () => values;
  files$.set = next => { values = next; };
  const store = { files$, readyFiles$: () => values.filter(file => file.status === 'ready'),
    hasUploadInProgress$: () => values.some(file => file.status === 'uploading') };
  const props = { conversation: {}, onCreateNewCompletion() {}, currentModelId: 'synthetic-model',
    entrySurface: 'chat_composer', isLibraryEnabled: true, value: store };
  const top = { stateNode: {} }; top.stateNode.current = top;
  const input = { isConnected: true, __reactFiber$fixture: { memoizedProps: props, return: top } };
  const root = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/' + (temporary ? '?temporary-chat=true' : '') },
    document: { querySelector: key => key === '#upload-files' ? input : {} },
    __elonChatGptDocumentToken: 'doc_submit_lease',
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: identity }) },
    __elonChatGptComposer: { currentModel: () => model }, __elonChatGptPrivateAttachmentProtocol: protocol };
  const composer = composerModule.create(root), binding = composer.capture();
  const file = new File(['synthetic bytes'], image ? 'fixture.png' : 'fixture.txt', { type: image ? 'image/png' : 'text/plain' });
  composer.associate(binding, file, { ok: true, associated: false, binding,
    stage: reused ? 'reused' : 'processed', fileId: 'file-synthetic',
    fileName: file.name, fileSize: file.size, mimeType: file.type,
    isTemporaryChat: temporary, ...(image ? { imageDimensions: { width: 12, height: 8 } } : {}),
    ...(reused ? { reusedFileName: 'original.txt', metadata: {
      libraryPersistenceResult: 'library', libraryFileId: 'library-synthetic', fileTokenSize: 8,
    } } : {}),
  }, 'synthetic-lease');
  const sender = senderModule.create(root, { composer });
  return { root, store, props, top, composer, sender, binding, file, attached: values[0],
    setIdentity: value => { identity = value; }, setModel: value => { model = value; } };
}

for (const config of [{}, { temporary: true }, { image: true }, { reused: true }]) {
  test('prepared lease retains official ready-file data: ' + JSON.stringify(config), () => {
    const f = fixture(config), lease = f.sender.prepareSubmit(f.store);
    assert.ok(lease);
    assert.equal(lease.current(), true);
    assert.equal(lease.readyFiles.length, 1);
    assert.deepEqual(lease.readyFiles[0], f.attached);
    assert.notEqual(lease.readyFiles[0], f.attached);
    assert.equal(lease.readyFiles[0].file, f.file);
    assert.ok(Object.isFrozen(lease.readyFiles));
    assert.ok(Object.isFrozen(lease.readyFiles[0].fileSpec));
    assert.equal(f.store.files$().length, 1, 'preparation does not remove or submit');
    assert.equal(lease.consumeAccepted(), true);
    assert.equal(f.store.files$().length, 0);
    assert.equal(lease.current(), false);
    assert.equal(lease.consumeAccepted(), false);
    assert.deepEqual(f.composer.merge([]), []);
  });
}

test('only the current sender-owned object can be prepared', () => {
  const f = fixture();
  assert.equal(f.sender.prepareSubmit({ ...f.store }), null);
  const another = composerModule.create(f.root);
  assert.equal(another.prepareSubmit(f.store), null);
  f.store.files$.set([{ ...f.attached }]);
  assert.equal(f.sender.prepareSubmit(f.store), null, 'matching IDs are not ownership');
});

for (const [name, change] of Object.entries({
  document: f => { f.root.__elonChatGptDocumentToken = 'doc_another'; },
  account: f => f.setIdentity('Bearer another-account'),
  route: f => { f.root.location.href += '?temporary-chat=true'; },
  model_label: f => f.setModel('another-model'),
  model_slug: f => { f.props.currentModelId = 'another-model'; },
  library_setting: f => { f.props.isLibraryEnabled = false; },
  extra_file: f => f.store.files$.set([f.attached, { status: 'ready' }]),
  upload: f => { f.store.hasUploadInProgress$ = () => true; },
  replaced: f => f.store.files$.set([{ ...f.attached }]),
  removed: f => f.composer.remove(f.composer.merge([])[0].id),
})) {
  test(name + ' invalidates the pre-dispatch lease', () => {
    const f = fixture(), lease = f.sender.prepareSubmit(f.store);
    change(f);
    assert.equal(lease.current(), false);
    assert.equal(f.sender.prepareSubmit(f.store), null);
  });
}

for (const [name, change] of Object.entries({
  file_id: f => { f.attached.fileSpec.id = 'file-another'; },
  size: f => { f.attached.fileSpec.size++; },
  mime: f => { f.attached.fileSpec.mimeType = 'application/pdf'; },
  source: f => { f.attached.source = 'connector'; },
  library_info: f => { f.attached.libraryFileInfo = { libraryProvider: 'other' }; },
  bytes: f => { f.attached.file = new File(['changed bytes'], f.file.name, { type: f.file.type }); },
})) {
  test(name + ' mutation cannot alter a captured ready-file snapshot', () => {
    const f = fixture(), lease = f.sender.prepareSubmit(f.store);
    const captured = JSON.stringify(lease.readyFiles);
    change(f);
    assert.equal(lease.current(), false);
    assert.equal(JSON.stringify(lease.readyFiles), captured);
    assert.equal(lease.consumeAccepted(), false);
    assert.equal(f.store.files$().length, 1);
  });
}

test('accepted cleanup preserves later user files and tolerates a newly allocated URL', () => {
  const f = fixture(), lease = f.sender.prepareSubmit(f.store), later = { status: 'ready', fileId: 'user-later' };
  f.root.location.href += 'c/11111111-2222-3333-4444-555555555555';
  f.composer.merge([]); // A snapshot can observe navigation before acceptance arrives.
  f.store.files$.set([f.attached, later]);
  assert.equal(lease.consumeAccepted(), true);
  assert.deepEqual(f.store.files$(), [later]);
});

for (const change of [f => f.setIdentity('Bearer another-account'),
  f => { f.root.__elonChatGptDocumentToken = 'doc_another'; }]) {
  test('accepted cleanup cannot mutate another document or identity', () => {
    const f = fixture(), lease = f.sender.prepareSubmit(f.store);
    change(f);
    assert.equal(lease.consumeAccepted(), false);
    assert.equal(f.store.files$().length, 1);
  });
}

test('missing composer port leaves compatibility available without throwing', () => {
  const f = fixture(), sender = senderModule.create(f.root, { composer: {} });
  assert.equal(sender.prepareSubmit(f.store), null);
});

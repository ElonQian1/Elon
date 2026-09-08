'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const composerModule = require('../android/app/src/main/assets/chatgpt_web_private_attachment_composer.js');
const protocol = require('../android/app/src/main/assets/chatgpt_web_private_attachment_protocol.js');

function fixture(options = {}) {
  let files = [];
  const files$ = () => files; files$.set = value => { files = value; };
  const store = { files$, readyFiles$: files$, hasUploadInProgress$: () => false };
  const props = { conversation: {}, onCreateNewCompletion() {}, currentModelId: 'synthetic-model',
    entrySurface: 'chat_composer', ...options.props };
  const top = { memoizedProps: props, stateNode: {} }; top.stateNode.current = top;
  const leaf = { memoizedProps: { value: store }, return: top };
  top.child = leaf;
  const input = { isConnected: true, __reactFiber$test: leaf };
  const root = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/' + (options.temporary ? '?temporary-chat=true' : '') },
    document: { querySelector: selector => selector === '#upload-files' ? input : {} },
    __elonChatGptDocumentToken: 'doc_fixture_1',
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ authorization: 'Bearer synthetic-identity' }) },
    __elonChatGptComposer: { currentModel: () => 'synthetic-model' },
    __elonChatGptPrivateAttachmentProtocol: protocol, setTimeout, clearTimeout };
  const composer = composerModule.create(root);
  return { root, props, input, top, composer, store,
    file: new File(['synthetic file'], 'fixture.txt', { type: 'text/plain' }) };
}

test('official current composer library intent determines upload and picker persistence', () => {
  const f = fixture({ props: { isLibraryEnabled: true } });
  const binding = f.composer.capture();
  const context = f.composer.uploadContext(binding, f.file);
  assert.equal(context.storeInLibrary, true);
  assert.equal(context.libraryPersistenceMode, 'opportunistic');
  assert.equal(context.isTemporaryChat, false);
  const picker = f.composer.pickerReservationContext(binding, 'document');
  assert.equal(picker.storeInLibrary, true);
  assert.equal(picker.libraryPersistenceMode, context.libraryPersistenceMode);
  assert.equal(protocol.prepare(f.file, context).library_persistence_mode, 'opportunistic');
  assert.equal(protocol.processBody('file-synthetic', f.file, context).metadata.store_in_library, true);
});

test('missing, disabled or different-surface library policy does not opt into persistence', () => {
  for (const props of [{}, { isLibraryEnabled: false }, { isLibraryEnabled: 'true' },
    { isLibraryEnabled: true, entrySurface: 'other' }]) {
    const f = fixture({ props }); const binding = f.composer.capture();
    assert.equal(f.composer.uploadContext(binding, f.file).storeInLibrary, false);
    assert.equal(f.composer.pickerReservationContext(binding, 'document').storeInLibrary, false);
  }
});

test('temporary scope overrides even an enabled library flag without mutating official settings', () => {
  const f = fixture({ temporary: true, props: { isLibraryEnabled: true } });
  const binding = f.composer.capture();
  for (const context of [f.composer.uploadContext(binding, f.file), f.composer.pickerReservationContext(binding, 'document')]) {
    assert.equal(context.storeInLibrary, false);
    assert.equal(context.libraryPersistenceMode, undefined);
    assert.equal(context.isTemporaryChat, true);
  }
  assert.equal(f.props.isLibraryEnabled, true);
});

test('library policy changes invalidate an in-flight upload owner', () => {
  const f = fixture({ props: { isLibraryEnabled: true } });
  const binding = f.composer.capture();
  f.props.isLibraryEnabled = false;
  assert.equal(f.composer.current(binding), false);
  assert.throws(() => f.composer.uploadContext(binding, f.file), /composer_changed/);
});

test('stale React alternate cannot opt into library storage', () => {
  const f = fixture({ props: { isLibraryEnabled: false } });
  const stale = { memoizedProps: { ...f.props, isLibraryEnabled: true }, stateNode: f.top.stateNode };
  f.input.__reactFiber$test = { return: stale, alternate: f.top.child };
  const composer = composerModule.create(f.root, { resolveStore: () => f.store });
  assert.equal(composer.uploadContext(composer.capture(), f.file).storeInLibrary, false);
});

test('opportunistic persistence is restricted to explicitly enabled non-project non-temporary uploads', () => {
  const f = fixture();
  for (const patch of [{ storeInLibrary: false }, { isTemporaryChat: true }, { isProjectThread: true }]) {
    assert.throws(() => protocol.prepare(f.file, { useCase: 'ace_upload', indexForRetrieval: false,
      storeInLibrary: true, libraryPersistenceMode: 'opportunistic', ...patch }), /unsupported_upload_context/);
  }
});

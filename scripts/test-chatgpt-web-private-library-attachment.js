'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fixture = require('./fixtures/chatgpt-attachment-composer');
const libraryModule = require('../android/app/src/main/assets/chatgpt_web_private_library_attachment');
const senderModule = require('../android/app/src/main/assets/chatgpt_web_private_attachment_send');
const HANDLE = 'library_' + 'a'.repeat(32);

function setup() {
  const f = fixture();
  f.root.File = File;
  f.root.__elonChatGptPrivateLibraryAttachment = libraryModule;
  Object.assign(f.fiber.memoizedProps, { conversation: {}, onCreateNewCompletion() {},
    currentModelId: 'synthetic-model', entrySurface: 'chat_composer', isLibraryEnabled: true });
  const source = { kind: 'file', id: 'libfile_fixture', file_id: 'file-fixture', name: 'fixture.txt',
    mime_type: 'text/plain', file_size_bytes: 45 };
  let fresh = true;
  f.root.__elonChatGptPrivateLibraryCatalog = { selectAttachment: handle => handle === HANDLE
    ? { source: { ...source }, current: () => fresh } : null };
  const sender = senderModule.create(f.root, { composer: f.composer });
  const responses = [], changes = [];
  const attach = (id = 'mcp_library1', overrides = {}) => sender.attachLibrary({
    requestId: id, value: JSON.stringify({ fileHandle: HANDLE }), selected: true, ...overrides,
  }, (...value) => responses.push(value), value => changes.push(value));
  return { ...f, source, sender, responses, changes, attach, expire: () => { fresh = false; } };
}

test('existing library file enters the existing owner without upload or automatic text send', async () => {
  const f = setup();
  f.root.fetch = () => { throw new Error('unexpected network write'); };
  await f.attach();
  assert.deepEqual(f.responses, [['attach_library_file', true, 'library_attachment_associated']]);
  const [item] = f.store.readyFiles$();
  assert.equal(item.file.size, 0, 'metadata-only File, not downloaded bytes');
  assert.equal(item.fileSpec.size, 45);
  assert.equal(item.libraryFileId, 'libfile_fixture');
  assert.equal(item.fileId, 'file-fixture');
  assert.equal(item.source, 'library');
  const lease = f.sender.prepareSubmit(f.store);
  assert.equal(lease.current(), true);
  assert.equal(lease.readyFiles[0].fileSpec.id, 'file-fixture');
  assert.equal(lease.consumeAccepted(), true);
  assert.equal(f.store.files$().length, 0);
  assert.equal(f.sender.merge([]).length, 0);
});

test('native removal and duplicate request use one attachment owner', async () => {
  const f = setup();
  await Promise.all([f.attach(), f.attach()]);
  assert.equal(f.store.files$().length, 1);
  assert.equal(f.changes.length, 1);
  const [item] = f.sender.merge([]);
  assert.equal(item.removable, true);
  assert.equal(f.sender.remove(item.id, () => {}, () => {}), true);
  await f.attach();
  assert.equal(f.store.files$().length, 0, 'old receipt is not a new attach instruction');
  await f.attach('mcp_library2');
  assert.equal(f.store.files$().length, 1);
});

test('special file variants cannot be treated as ordinary native library attachments', async () => {
  for (const patch of [{ file_id: 'invalid' }, { id: 'invalid' }, { external_account: {} }, { saved_entity: {} },
    { library_artifact_type: 'canvas' }, { cloud_doc_url: 'https://example.com/doc' }, { is_project: true },
    { trashed_at: 1 }, { name: '../file.txt' }, { mime_type: 'video/mp4' }, { file_size_bytes: -1 }]) {
    const f = setup(); Object.assign(f.source, patch);
    await f.attach();
    assert.equal(f.responses[0][1], false, JSON.stringify(patch));
    assert.equal(f.store.files$().length, 0);
  }
});

test('ordinary existing conversation reads its scope before attachment', async () => {
  const f = setup(), id = '00000000-0000-4000-8000-000000000001';
  f.root.location.href += 'c/' + id;
  let reads = 0;
  f.root.__elonChatGptPrivateTransport.readAttachmentContext = async () => { reads++; return { conversationId: id, ordinary: true }; };
  await f.attach();
  assert.equal(reads, 1);
  assert.equal(f.responses[0][1], true);
});

test('unknown scopes, disabled library, occupied input and missing confirmation preserve composer', async () => {
  for (const mutate of [
    f => { f.root.location.href += '?temporary-chat=true'; },
    f => { f.fiber.memoizedProps.isLibraryEnabled = false; },
    f => { f.store.files$.set([{ status: 'ready', tempId: 'other' }]); },
  ]) {
    const f = setup(); mutate(f); const before = f.store.files$();
    await f.attach();
    assert.equal(f.responses[0][1], false);
    assert.equal(f.store.files$(), before);
  }
  const f = setup(); await f.attach('mcp_library1', { selected: false });
  assert.equal(f.responses[0][1], false);
});

test('account, route, model, file metadata or cancellation changes cannot publish a late selection', async () => {
  for (const mutate of [f => f.setAccount('Bearer changed-account-token'), f => { f.root.location.href += '?temporary-chat=true'; },
    f => f.setModel('other'), f => f.expire(), f => f.sender.cancel()]) {
    const f = setup(), id = '00000000-0000-4000-8000-000000000001';
    f.root.location.href += 'c/' + id;
    let resolve;
    f.root.__elonChatGptPrivateTransport.readAttachmentContext = () => new Promise(done => { resolve = done; });
    const pending = f.attach();
    mutate(f); resolve({ conversationId: id, ordinary: true }); await pending;
    assert.equal(f.store.files$().length, 0);
    assert.equal(f.responses[0][1], false);
  }
});

test('images use the same observed library reference shape, not invented upload metadata', async () => {
  const f = setup(); Object.assign(f.source, { name: 'image.png', mime_type: 'image/png' });
  await f.attach();
  const item = f.store.readyFiles$()[0];
  assert.equal(item.fileSpec.width, 512);
  assert.equal(item.fileSpec.height, 512);
  assert.equal(Object.hasOwn(item, 'autoReused'), false);
  assert.equal(f.responses[0][1], true);
});

test('library command router calls the existing attachment sender and its snapshot callback', () => {
  const fs = require('node:fs'), vm = require('node:vm');
  let called = 0, changed = 0;
  const root = { __elonChatGptPrivateAttachmentSend: { attachLibrary(command, respond, notify) {
    called++; notify(); respond(command.action, true, 'library_attachment_associated');
  } } };
  vm.runInNewContext(fs.readFileSync(require.resolve('../android/app/src/main/assets/chatgpt_web_adapter_conversation_directory_requests'), 'utf8'), { window: root });
  const receipts = [], command = { action: 'attach_library_file' };
  const handler = root.__elonChatGptConversationDirectoryRequests.create({ attachmentChanged: () => changed++ });
  assert.equal(handler.handleCommand(command, (...value) => receipts.push(value)), true);
  assert.equal(called, 1); assert.equal(changed, 1);
  const stale = root.__elonChatGptConversationDirectoryRequests.create({});
  stale.handleCommand(command, (...value) => receipts.push(value));
  assert.equal(called, 1, 'old adapter wiring cannot publish a file without a native snapshot callback');
  assert.equal(receipts[1][1], false);
});

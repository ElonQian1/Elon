'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const setup = require('./fixtures/chatgpt-library-append');

test('production snapshot invalidation cannot cancel a committed library receipt', async () => {
  const f = setup(), receipts = [], notifications = [];
  for (const [index, patch] of [[1, {}], [2, {}], [3, { id: 'external-gdrive:file:synthetic123' }]]) {
    await f.sender.attachLibrary({ selected: true, requestId: 'mcp_notify' + index,
      value: JSON.stringify({ fileHandle: f.source(index, patch) }) },
    (...receipt) => receipts.push(receipt), () => {
      // The adapter invalidates the text context by cancelling attachment work.
      f.sender.cancel();
      notifications.push(receipts.length);
    });
    assert.equal(receipts.length, index);
    assert.deepEqual(receipts.at(-1), ['attach_library_file', true, 'library_attachment_associated']);
    assert.equal(f.sender.prepareSubmit(f.store).readyFiles.length, index);
  }
  assert.deepEqual(notifications, [1, 2, 3], 'commit receipts precede native snapshot notification');
  assert.equal(f.requests(), 1);
});

test('sequential library references share one native list and exact submit lease', async () => {
  const f = setup();
  await f.attach(f.source(1));
  const first = f.store.files$()[0], oldLease = f.sender.prepareSubmit(f.store);
  assert.equal((await f.attach(f.source(2)))[1], true);
  assert.equal((await f.attach(f.source(3, { name: 'image.png', mime_type: 'image/png' })))[1], true);
  assert.equal(f.store.files$()[0], first);
  assert.equal(f.sender.merge([]).length, 3);
  assert.equal(oldLease.current(), false);
  const lease = f.sender.prepareSubmit(f.store);
  assert.deepEqual(lease.readyFiles.map(item => item.fileId), ['file-1', 'file-2', 'file-3']);
  assert.ok(lease.readyFiles.every(item => item.file.size === 0));
  assert.equal(lease.consumeAccepted(), true);
  assert.equal(f.store.files$().length, 0);
  assert.equal(f.loads(), 1, 'subsequent checks use the existing versioned module cache');
  assert.equal(f.requests(), 0, 'ordinary references do not download or upload bytes');
});

test('library append retains a staged private local upload and each can be removed independently', async () => {
  const f = setup(), binding = f.composer.capture();
  f.composer.associate(binding, f.file, f.result(binding), 'local');
  const first = f.store.files$()[0];
  assert.equal((await f.attach(f.source(1)))[1], true);
  assert.equal(f.store.files$()[0], first);
  assert.equal(f.sender.prepareSubmit(f.store).readyFiles.length, 2);
  const second = f.sender.merge([])[1];
  assert.equal(f.sender.remove(second.id, () => {}, () => {}), true);
  assert.deepEqual(f.store.files$(), [first]);
  assert.equal((await f.attach(f.source(2)))[1], true);
  assert.equal(f.sender.remove(f.sender.merge([])[0].id, () => {}, () => {}), true);
  assert.deepEqual(f.sender.prepareSubmit(f.store).readyFiles.map(item => item.fileId), ['file-2']);
});

test('duplicate selections by file or library ID are no-ops, including at capacity', async () => {
  const f = setup(), first = f.source(1);
  await f.attach(first, 'mcp_original'); f.props.maxTotalLibraryAttachmentCount = 1;
  const before = f.store.files$();
  for (const handle of [first, f.source(2, { file_id: 'file-1' }), f.source(3, { id: 'libfile_1' })]) {
    assert.equal((await f.attach(handle))[1], true);
    assert.equal(f.store.files$(), before);
  }
  assert.equal(f.changes.length, 1);
  assert.equal(f.loads(), 0, 'duplicates neither refresh policy nor make requests');
  f.sender.remove(f.sender.merge([])[0].id, () => {}, () => {});
  await f.attach(first, 'mcp_original');
  assert.equal(f.store.files$().length, 0, 'old receipt cannot reattach a removed item');
});

test('replaying an earlier receipt after appending cannot replace the expanded owner', async () => {
  const f = setup(), first = f.source(1);
  await f.attach(first, 'mcp_first'); await f.attach(f.source(2), 'mcp_second');
  const before = f.store.files$(), lease = f.sender.prepareSubmit(f.store);
  assert.equal((await f.attach(first, 'mcp_first'))[1], true);
  assert.equal(f.store.files$(), before);
  assert.equal(lease.current(), true);
  assert.equal(f.changes.length, 2);
});

test('distinct append requests remain single-flight while the runtime is loading', async () => {
  const f = setup(); await f.attach(f.source(1));
  let resolve;
  f.root.__elonChatGptPrivateRuntimeBindings.load = () => new Promise(done => { resolve = done; });
  const before = f.store.files$(), pending = f.attach(f.source(2), 'mcp_pending');
  assert.equal((await f.attach(f.source(3), 'mcp_other'))[2], 'library_attachment_busy');
  f.sender.cancel(); resolve(f.namespace);
  assert.equal((await pending)[1], false);
  assert.equal(f.store.files$(), before);
});

test('concrete cloud reference appends after local library references and does not rematerialize a duplicate', async () => {
  const f = setup(); await f.attach(f.source(1));
  const handle = f.source(2, { id: 'external-gdrive:file:synthetic123', file_id: 'file-1' });
  assert.equal((await f.attach(handle))[1], true);
  assert.equal(f.requests(), 1, 'stale copied backing ID cannot suppress materialization');
  assert.deepEqual(f.sender.prepareSubmit(f.store).readyFiles.map(item => item.fileId), ['file-1', 'file-materialized']);
  assert.equal((await f.attach(handle))[1], true);
  assert.equal(f.requests(), 1);
  assert.equal(f.store.files$().length, 2);
});

for (const kind of ['limit', 'official_rejection', 'missing_policy', 'unowned_file', 'uploading', 'wrong_store', 'disabled']) {
  test('append preserves existing files when ' + kind, async () => {
    const f = setup(); await f.attach(f.source(1));
    if (kind === 'limit') f.props.maxTotalLibraryAttachmentCount = 1;
    if (kind === 'official_rejection') f.reject();
    if (kind === 'missing_policy') delete f.props.maxLibraryAttachmentCount;
    if (kind === 'unowned_file') f.store.files$.set([...f.store.files$(), { status: 'ready', fileId: 'foreign' }]);
    if (kind === 'uploading') f.store.files$()[0].status = 'uploading';
    if (kind === 'wrong_store') f.fiber.dependencies.firstContext.memoizedValue = {};
    if (kind === 'disabled') f.props.composerDisabled = true;
    const before = f.store.files$();
    assert.equal((await f.attach(f.source(2)))[1], false);
    assert.equal(f.store.files$(), before);
    assert.equal(f.requests(), 0);
  });
}

for (const kind of ['removed', 'metadata', 'account', 'route', 'model', 'document', 'selection', 'cancel', 'lower_limit']) {
  test('late cloud preparation cannot publish after ' + kind, async () => {
    const f = setup(); await f.attach(f.source(1));
    let finish, entered;
    const started = new Promise(resolve => { entered = resolve; });
    f.root.fetch = () => { entered(); return new Promise(resolve => { finish = resolve; }); };
    const pending = f.attach(f.source(2, { id: 'external-gdrive:file:synthetic123' }));
    await started;
    if (kind === 'removed') f.sender.remove(f.sender.merge([])[0].id, () => {}, () => {});
    if (kind === 'metadata') f.store.files$()[0].fileSpec.name = 'changed.txt';
    if (kind === 'account') f.setAccount('Bearer changed-synthetic-identity');
    if (kind === 'route') f.root.location.href += 'c/00000000-0000-4000-8000-000000000001';
    if (kind === 'model') f.setModel('changed');
    if (kind === 'document') f.root.__elonChatGptDocumentToken = 'doc_changed';
    if (kind === 'selection') f.expire();
    if (kind === 'cancel') f.sender.cancel();
    if (kind === 'lower_limit') f.props.maxTotalLibraryAttachmentCount = 1;
    const before = f.store.files$();
    finish(Response.json({ file_id: 'file-materialized', file_name: 'cloud.txt', mime_type: 'text/plain', file_size_bytes: 60 }));
    assert.equal((await pending)[1], false);
    assert.equal(f.store.files$(), before);
    assert.ok(f.store.files$().every(item => item.fileId !== 'file-materialized'));
  });
}

test('failed readback rolls back only newly added entries, not prior or external files', async () => {
  const f = setup(); await f.attach(f.source(1));
  const first = f.store.files$()[0], other = { status: 'ready', fileId: 'external' };
  const set = f.store.files$.set;
  f.store.files$.set = next => set(next.length === 2 && next[1].fileId === 'file-2' ? [...next, other] : next);
  assert.equal((await f.attach(f.source(2)))[1], false);
  assert.deepEqual(f.store.files$(), [first, other]);
});

test('nine-item native batch ceiling is retained without claiming it is the website limit', async () => {
  const f = setup(); f.props.maxLibraryAttachmentCount = 50; f.props.maxTotalLibraryAttachmentCount = 50;
  for (let i = 1; i <= 9; i++) assert.equal((await f.attach(f.source(i)))[1], true);
  assert.equal((await f.attach(f.source(10)))[1], false);
  assert.equal(f.sender.prepareSubmit(f.store).readyFiles.length, 9);
});

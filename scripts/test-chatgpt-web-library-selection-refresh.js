'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const composerFixture = require('./fixtures/chatgpt-library-append');
const catalogModule = require('../android/app/src/main/assets/chatgpt_web_private_library_catalog');

const file = (id = 'libfile_selection', patch = {}) => ({ kind: 'file', id,
  file_id: 'file-selection', name: 'selection.txt', mime_type: 'text/plain', file_size_bytes: 45, ...patch });

async function setup(t, value = {}, initial = file()) {
  let now = Date.now(), payload = { items: [initial] }, sequence = 0;
  t.mock.method(Date, 'now', () => now);
  const f = composerFixture(), calls = [], snapshots = [], receipts = [];
  f.root.__elonChatGptPrivateJsonRequest = { async request(root, url, init, budget) {
    calls.push({ url, init, budget });
    if (payload instanceof Error) throw payload;
    return { payload: typeof payload === 'function' ? await payload(init) : payload };
  } };
  const catalog = f.root.__elonChatGptPrivateLibraryCatalog = catalogModule.create(f.root);
  const list = (input = value) => catalog.list({ requestId: 'mcp_list' + ++sequence,
    value: JSON.stringify(input) }, event => snapshots.push(event), (...v) => receipts.push(v));
  await list();
  const handle = snapshots.at(-1).items[0].handle;
  return { ...f, catalog, handle, calls, snapshots, receipts, list,
    advance: ms => { now += ms; }, setPayload: next => { payload = next; },
    selection: () => catalog.selectAttachment(handle),
    attachSelected: () => f.attach(handle),
  };
}

test('a fresh native selection attaches without an extra catalog request', async t => {
  const f = await setup(t);
  assert.equal((await f.attachSelected())[1], true);
  assert.equal(f.calls.length, 1);
  assert.equal(f.store.readyFiles$().length, 1);
});

test('an aged visible selection revalidates exactly once and then reaches the existing attachment owner', async t => {
  const f = await setup(t, { query: 'selection' });
  f.advance(61000);
  assert.ok(f.selection(), 'visible metadata must survive TTL for bounded read-only revalidation');
  assert.equal(f.selection().current(), false);
  assert.equal(f.catalog.selectMutation(f.handle), null, 'write-selection TTL is not relaxed');
  assert.deepEqual(await f.attachSelected(), ['attach_library_file', true, 'library_attachment_associated']);
  assert.equal(f.calls.length, 2);
  assert.equal(f.calls[1].url, f.calls[0].url);
  assert.equal(f.calls[1].init.method, 'GET');
  assert.equal(f.calls[1].init.credentials, 'same-origin');
  assert.equal(f.calls[1].init.redirect, 'error');
  assert.equal(f.calls[1].budget.timeoutMs, 10000);
  assert.equal(f.calls[1].budget.maxBytes, 2 * 1024 * 1024);
  assert.equal(f.store.readyFiles$()[0].fileId, 'file-selection');
  assert.equal(f.changes.length, 1);
  assert.equal(f.snapshots.length, 1, 'do not replace the visible list or selected handle');
  assert.equal(f.catalog.selectMutation(f.handle), null, 'one read cannot authorize destructive operations');
  assert.equal(await f.selection().refresh(new AbortController().signal), true);
  assert.equal(f.calls.length, 2, 'the revalidated item has its own warm lease');
});

test('one selected item does not renew siblings or the whole list', async t => {
  const f = await setup(t);
  f.setPayload({ items: [file(), file('libfile_second', { file_id: 'file-second' })] });
  await f.list({ operation: 'refresh' });
  const [first, second] = f.snapshots.at(-1).items;
  f.advance(61000);
  assert.equal(await f.catalog.selectAttachment(first.handle).refresh(new AbortController().signal), true);
  assert.equal(f.catalog.selectAttachment(second.handle).current(), false);
  assert.equal(f.catalog.selectMutation(first.handle), null);
  const before = f.calls.length;
  await f.list();
  assert.equal(f.calls.length, before + 1, 'normal stale list refresh remains independent');
});

test('unchanged structured scope metadata survives a new JSON response and object key order', async t => {
  const initial = file(undefined, { context_scopes: [{ type: 'conversation', permissions: { read: true, write: false } }],
    preview_file: { file_id: 'file-preview', dimensions: [12, 34] } });
  const f = await setup(t, {}, initial);
  f.advance(61000);
  const reread = JSON.parse(JSON.stringify(initial));
  reread.context_scopes[0].permissions = { write: false, read: true };
  f.setPayload({ items: [reread] });
  assert.deepEqual(await f.attachSelected(), ['attach_library_file', true, 'library_attachment_associated']);
  assert.equal(f.calls.length, 2);
  assert.equal(f.store.readyFiles$().length, 1);
  assert.equal(f.catalog.selectMutation(f.handle), null);
});

for (const [name, change] of Object.entries({
  permission: row => { row.context_scopes[0].permissions.read = false; },
  missing: row => { delete row.context_scopes[0].permissions.write; },
  added: row => { row.context_scopes[0].permissions.share = true; },
  type: row => { row.context_scopes[0].permissions.read = 'true'; },
  shape: row => { row.context_scopes = { 0: row.context_scopes[0] }; },
  order: row => { row.preview_file.dimensions.reverse(); },
  preview: row => { row.preview_file.file_id = 'file-other'; },
})) test('structured revalidation rejects actual ' + name + ' changes', async t => {
  const initial = file(undefined, { context_scopes: [{ permissions: { read: true, write: false } }],
    preview_file: { file_id: 'file-preview', dimensions: [12, 34] } });
  const f = await setup(t, {}, initial);
  const reread = JSON.parse(JSON.stringify(initial)); change(reread);
  f.advance(61000); f.setPayload({ items: [reread] });
  assert.equal((await f.attachSelected())[1], false);
  assert.equal(f.store.files$().length, 0);
});

test('structured revalidation has a depth and work bound', async t => {
  const initial = file(undefined, { context_scopes: {} });
  let nested = initial.context_scopes;
  for (let i = 0; i < 40; i++) nested = nested.next = {};
  const f = await setup(t, {}, initial);
  f.advance(61000); f.setPayload({ items: [JSON.parse(JSON.stringify(initial))] });
  assert.equal((await f.attachSelected())[1], false);
  assert.equal(f.store.files$().length, 0);
  const wide = file(undefined, { context_scopes: Array.from({ length: 5000 }, (_, i) => i) });
  f.setPayload({ items: [wide] }); await f.list({ operation: 'refresh' });
  const handle = f.snapshots.at(-1).items[0].handle;
  f.advance(61000); f.setPayload({ items: [JSON.parse(JSON.stringify(wide))] });
  assert.equal(await f.catalog.selectAttachment(handle).refresh(new AbortController().signal), false);
});

test('later-page selections retain the actual search and cursor request that observed them', async t => {
  const f = await setup(t, { query: 'selection & query' });
  f.setPayload({ items: [file()], cursor: 'opaque-cursor-synthetic' });
  await f.list({ query: 'selection & query', operation: 'refresh' });
  f.setPayload({ items: [file('libfile_second', { file_id: 'file-second' })] });
  await f.list({ query: 'selection & query', operation: 'next' });
  const laterUrl = f.calls.at(-1).url;
  const selected = f.catalog.selectAttachment(f.snapshots.at(-1).items[1].handle);
  f.advance(61000);
  assert.equal(await selected.refresh(new AbortController().signal), true);
  assert.equal(f.calls.at(-1).url, laterUrl);
  assert.equal(new URL(laterUrl).searchParams.get('cursor'), 'opaque-cursor-synthetic');
  const wire = JSON.stringify(f.snapshots);
  for (const secret of ['lookupUrl', 'observedAt', 'backend-api', 'opaque-cursor-synthetic', 'libfile_second']) {
    assert.equal(wire.includes(secret), false, secret);
  }
});

test('folder selection revalidation uses its observed parent scope', async t => {
  const f = await setup(t);
  f.setPayload({ items: [{ kind: 'directory', id: 'directory-parent', name: 'Folder' }] });
  await f.list({ operation: 'refresh' });
  const directoryHandle = f.snapshots.at(-1).items[0].handle;
  f.setPayload({ items: [file(undefined, { parent_directory_id: 'directory-parent' })] });
  await f.list({ directoryHandle });
  const url = f.calls.at(-1).url;
  const selection = f.catalog.selectAttachment(f.snapshots.at(-1).items[0].handle);
  f.advance(61000);
  assert.equal(await selection.refresh(new AbortController().signal), true);
  assert.equal(f.calls.at(-1).url, url);
  assert.equal(new URL(url).searchParams.get('parent_directory_id'), 'directory-parent');
});

test('a fresh selection crossing TTL during scope preparation keeps its bounded operation lease', async t => {
  const f = await setup(t), id = '00000000-0000-4000-8000-000000000001';
  f.root.location.href += 'c/' + id;
  let complete;
  f.root.__elonChatGptPrivateTransport.readAttachmentContext = () => new Promise(resolve => { complete = resolve; });
  f.advance(59000);
  const pending = f.attachSelected();
  f.advance(2000);
  complete({ conversationId: id, ordinary: true });
  assert.equal((await pending)[1], true);
  assert.equal(f.calls.length, 1);
  assert.equal(f.store.files$().length, 1);
});

test('duplicate clicks join a single stale read and publish one attachment', async t => {
  const f = await setup(t);
  f.advance(61000);
  let complete;
  f.setPayload(() => new Promise(resolve => { complete = resolve; }));
  const first = f.attach(f.handle, 'mcp_same'), second = f.attach(f.handle, 'mcp_same');
  assert.equal(f.calls.length, 2);
  assert.equal(f.responses.length, 0);
  complete({ items: [file()] });
  await Promise.all([first, second]);
  assert.ok(f.responses.every(row => row[1] === true));
  assert.equal(f.store.files$().length, 1);
  assert.equal(f.changes.length, 1);
});

test('loading another page does not extend an older item lease', async t => {
  const f = await setup(t);
  f.setPayload({ items: [file()], cursor: 'opaque-cursor-synthetic' });
  await f.list({ operation: 'refresh' });
  const first = f.snapshots.at(-1).items[0].handle;
  f.advance(61000);
  f.setPayload({ items: [file('libfile_later')] });
  await f.list({ operation: 'next' });
  assert.equal(f.catalog.selectAttachment(first).current(), false);
});

for (const [name, items] of Object.entries({
  missing: [], duplicate: [file(), file()], renamed: [file(undefined, { name: 'changed.txt' })],
  backing: [file(undefined, { file_id: 'file-other' })], media: [file(undefined, { mime_type: 'application/pdf' })],
  size: [file(undefined, { file_size_bytes: 99 })], trashed: [file(undefined, { trashed_at: 1 })],
  folder: [file(undefined, { parent_directory_id: 'other-directory' })],
  project: [file(undefined, { project_id: 'g-other' })],
  external: [file(undefined, { external_account: {} })],
  malformed: null,
})) test('revalidation rejects ' + name + ' metadata before staging any attachment', async t => {
  const f = await setup(t);
  f.advance(61000);
  f.setPayload({ items });
  assert.equal((await f.attachSelected())[1], false);
  assert.equal(f.calls.length, 2, 'the selected item is actually re-read before refusal');
  assert.equal(f.store.files$().length, 0);
  assert.equal(f.changes.length, 0);
});

test('unrelated server timestamps do not invalidate the chosen file', async t => {
  const f = await setup(t);
  f.advance(61000);
  f.setPayload({ items: [file(undefined, { updated_at: 123456, last_accessed_at: 567890 })] });
  assert.equal((await f.attachSelected())[1], true);
});

for (const [name, change] of Object.entries({
  account: f => f.setAccount('Bearer synthetic-another-account'),
  document: f => { f.root.document = { ...f.root.document }; },
  token: f => { f.root.__elonChatGptDocumentToken = 'doc_other'; },
  route: f => { f.root.location.href += 'c/other'; },
  origin: f => { f.root.location.origin = 'https://example.invalid'; },
  transport: f => { f.root.__elonChatGptPrivateTransport = { ...f.root.__elonChatGptPrivateTransport }; },
  disposed: f => f.catalog.dispose(),
  mutation: f => { f.root.__elonChatGptPrivateLibraryMutations = { busy: () => true }; },
})) test(name + ' changes reject stale selections before and after their read', async t => {
  for (const during of [false, true]) {
    const f = await setup(t), selected = f.selection();
    f.advance(61000);
    let complete;
    f.setPayload(() => new Promise(resolve => { complete = resolve; }));
    if (!during) change(f);
    const pending = selected.refresh(new AbortController().signal);
    if (during) { change(f); complete({ items: [file()] }); }
    assert.equal(await pending, false);
    assert.equal(f.calls.length, during ? 2 : 1);
    assert.equal(selected.current(), false);
    t.mock.restoreAll();
  }
});

test('list replacement invalidates an in-flight item without reviving its old handle', async t => {
  const f = await setup(t), selected = f.selection();
  f.advance(61000);
  let complete;
  f.setPayload(() => new Promise(resolve => { complete = resolve; }));
  const pending = selected.refresh(new AbortController().signal);
  f.setPayload({ items: [file()] });
  await f.list({ operation: 'refresh' });
  complete({ items: [file()] });
  assert.equal(await pending, false);
  assert.equal(f.catalog.selectAttachment(f.handle), null);
});

test('refresh errors preserve known rows and fail the action without claiming unsupported', async t => {
  const f = await setup(t);
  f.advance(61000);
  f.setPayload(new Error('http_503'));
  assert.deepEqual(await f.attachSelected(), ['attach_library_file', false, 'library_attachment_unconfirmed']);
  assert.equal(f.snapshots[0].items.length, 1);
  assert.equal(f.calls.length, 2);
  assert.equal(f.store.files$().length, 0);
});

test('cancelled revalidation never stages or notifies even if the GET returns late', async t => {
  const f = await setup(t);
  f.advance(61000);
  let complete;
  f.setPayload(() => new Promise(resolve => { complete = resolve; }));
  const pending = f.attachSelected();
  assert.equal(f.calls.length, 2);
  f.sender.cancel();
  assert.equal((await pending)[1], false);
  assert.equal(f.calls[1].init.signal.aborted, true);
  complete({ items: [file()] });
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(f.store.files$().length, 0);
  assert.equal(f.changes.length, 0);
  assert.equal(f.selection().current(), false);
});

test('revalidation shares the existing whole-operation deadline and single-flight receipt', async t => {
  const f = await setup(t), timers = new Map();
  let tick = 0, complete;
  f.root.performance = { now: () => tick };
  f.root.setTimeout = (fn, ms) => { timers.set(ms, fn); return ms; };
  f.root.clearTimeout = id => timers.delete(id);
  f.advance(61000);
  f.setPayload(() => new Promise(resolve => { complete = resolve; }));
  const first = f.attach(f.handle, 'mcp_same'), duplicate = f.attach(f.handle, 'mcp_same');
  assert.equal(f.calls.length, 2);
  tick = 24000;
  timers.get(24000)();
  await Promise.all([first, duplicate]);
  assert.ok(f.responses.every(row => row[1] === false));
  complete({ items: [file()] });
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(f.store.files$().length, 0);
  assert.equal(f.changes.length, 0);
  assert.equal(timers.size, 0);
});

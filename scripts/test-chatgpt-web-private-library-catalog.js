'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const { webcrypto } = require('node:crypto');
const catalog = require('../android/app/src/main/assets/chatgpt_web_private_library_catalog.js');
const download = require('../android/app/src/main/assets/chatgpt_web_private_file_download.js');

test('attachment selections expose only verified ordinary rows and expire after a refresh or mutation', async () => {
  const f = fixture();
  f.root.__elonChatGptPrivateLibraryAttachment = require('../android/app/src/main/assets/chatgpt_web_private_library_attachment');
  f.root.__elonChatGptPrivateAttachmentProtocol = require('../android/app/src/main/assets/chatgpt_web_private_attachment_protocol');
  f.setNext({ items: [{ ...file(), file_id: 'file-synthetic' }, { ...file('libfile_cloud'), external_account: {} }] });
  await f.list();
  const [ordinary, cloud] = f.events[0].items;
  assert.equal(ordinary.canAttach, true);
  assert.equal(cloud.canAttach, false);
  const selection = f.service.selectAttachment(ordinary.handle);
  assert.equal(selection.current(), true);
  f.service.selectMutation(ordinary.handle).settle(true, 'rename', 'renamed.txt');
  assert.equal(selection.current(), false);
  assert.equal(f.service.selectAttachment(ordinary.handle), null);
  await f.list({ operation: 'refresh' });
  const fresh = f.service.selectAttachment(f.events.at(-1).items[0].handle);
  assert.equal(fresh.current(), true);
  await f.list({ operation: 'refresh' });
  assert.equal(fresh.current(), false);
});

const folder = (id = 'directory-synthetic', name = 'Folder') => ({ kind: 'directory', id, name });
const file = (id = 'libfile_synthetic') => ({ kind: 'file', id, name: 'fixture.txt', mime_type: 'text/plain', file_size_bytes: 7 });
function fixture() {
  const calls = [], events = [], results = [];
  let auth = 'Bearer synthetic-library-catalog', next = { items: [folder(), file()], cursor: null }, sequence = 0;
  const root = {
    location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/unchanged' },
    __elonChatGptDocumentToken: 'doc_synthetic_catalog', crypto: webcrypto, AbortController, setTimeout, clearTimeout,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: auth }) },
    __elonChatGptPrivateJsonRequest: { async request(_, url, init, budget) {
      calls.push({ url: new URL(url), init, budget });
      if (next instanceof Error) throw next;
      return { payload: typeof next === 'function' ? await next(init) : next };
    } },
    elonChatGptFileDownload: { postMessage() {} },
  };
  root.__elonChatGptPrivateFileDownload = download.create(root);
  const service = catalog.create(root);
  const list = (value = {}, requestId = 'mcp_' + ++sequence) => service.list({ value: JSON.stringify(value), requestId },
    (type, value) => events.push({ type, ...value }), (...value) => results.push(value));
  return { root, calls, events, results, service, list, setNext: value => { next = value; },
    setAuth: value => { auth = value; } };
}

test('library nodes use the evidenced GET contract without changing conversation or DOM', async () => {
  const f = fixture();
  await f.list();
  const { url, init, budget } = f.calls[0];
  assert.equal(url.pathname, '/backend-api/files/library/nodes');
  assert.deepEqual(Object.fromEntries(url.searchParams), { hydrate_folder_thumbnails: 'true',
    include_folder_counts: 'true', include_saved_entities: 'true' });
  assert.equal(init.method, 'GET');
  assert.equal(init.credentials, 'same-origin');
  assert.equal(init.redirect, 'error');
  assert.equal(budget.timeoutMs, 10000);
  assert.equal(f.root.location.href, 'https://chatgpt.com/c/unchanged');
  assert.equal(f.events[0].type, 'library_files_snapshot');
  assert.equal(f.events[0].items.length, 2);
  assert.match(f.events[0].items[1].downloadHandle, /^download_[a-f0-9]{32}$/);
  const json = JSON.stringify(f.events);
  for (const secret of ['directory-synthetic', 'libfile_synthetic', 'Bearer', 'authorization', 'cursor']) {
    assert.ok(!json.includes(secret), secret);
  }
});

test('folder navigation uses opaque handles; root return reuses cache and does not navigate the page', async () => {
  const f = fixture();
  await f.list();
  const directoryHandle = f.events[0].items[0].handle;
  f.setNext({ items: [file('libfile_nested')], cursor: null });
  await f.list({ directoryHandle });
  assert.equal(f.calls[1].url.searchParams.get('parent_directory_id'), 'directory-synthetic');
  assert.deepEqual(f.events[1].breadcrumbs, [{ handle: directoryHandle, name: 'Folder' }]);
  await f.list();
  assert.equal(f.calls.length, 2);
  assert.equal(f.results.at(-1)[2], 'library_cached');
  assert.equal(f.events.at(-1).items.length, 2);
  assert.deepEqual(f.events.at(-1).breadcrumbs, []);
});

test('pagination deduplicates node identities and stops repeating cursor loops', async () => {
  const f = fixture();
  f.setNext({ items: [file()], cursor: 'opaque-cursor-synthetic' });
  await f.list();
  f.setNext({ items: [file(), file('libfile_second')], cursor: 'opaque-cursor-synthetic' });
  await f.list({ operation: 'next' });
  assert.equal(f.calls[1].url.searchParams.get('cursor'), 'opaque-cursor-synthetic');
  assert.equal(f.events.at(-1).items.length, 2);
  assert.equal(f.events.at(-1).hasMore, false);
  assert.equal(f.events.at(-1).partial, true);
  await f.list({ operation: 'next' });
  assert.equal(f.calls.length, 2);
  assert.equal(f.results.at(-1)[2], 'library_page_expired');
});

test('refresh emits cached content first and preserves it on failure, without claiming absence', async () => {
  const f = fixture();
  await f.list();
  f.setNext(new Error('http_503'));
  await f.list({ operation: 'refresh' });
  assert.equal(f.events.length, 2);
  assert.equal(f.events.at(-1).stale, true);
  assert.equal(f.events.at(-1).items.length, 2);
  assert.deepEqual(f.results.at(-1), ['list_library_files', false, 'library_read_failed']);
});

test('search query is scoped and URL encoded; switching query does not reuse another result', async () => {
  const f = fixture();
  await f.list({ query: '  a & b  ' });
  assert.equal(f.calls[0].url.searchParams.get('q'), 'a & b');
  await f.list({ query: 'other' });
  await f.list({ query: 'a & b' });
  assert.equal(f.calls.length, 2);
  assert.equal(f.events.at(-1).query, 'a & b');
});

test('malformed rows remain partial and do not create fake empty-success metadata', async () => {
  const f = fixture();
  f.setNext({ items: [file(), { ...file(), id: 'bad id' }, { kind: 'saved_entity', id: 'saved', name: 'Saved' },
    { ...file(), id: 'bad-size', file_size_bytes: -3 }], cursor: null });
  await f.list();
  assert.equal(f.events[0].partial, true);
  assert.equal(f.events[0].items.length, 1);
  f.setNext({ items: {}, cursor: null });
  await f.list({ operation: 'refresh' });
  assert.equal(f.results.at(-1)[2], 'library_response_invalid');
});

test('identity changes discard folder handles and cached content before another request', async () => {
  const f = fixture();
  await f.list();
  const directoryHandle = f.events[0].items[0].handle;
  f.setAuth('Bearer synthetic-second-account');
  await f.list({ directoryHandle });
  assert.equal(f.results.at(-1)[2], 'library_selection_expired');
  assert.equal(f.events.length, 1);
  f.setNext({ items: [], cursor: null });
  await f.list();
  assert.equal(f.calls.length, 2);
  assert.equal(f.events.at(-1).items.length, 0);
});

test('late responses after route or identity change never populate the native catalogue', async () => {
  for (const change of [f => { f.root.location.href = 'https://chatgpt.com/c/other'; },
    f => f.setAuth('Bearer synthetic-other')]) {
    const f = fixture();
    let resolve;
    f.setNext(() => new Promise(done => { resolve = done; }));
    const pending = f.list();
    change(f);
    resolve({ items: [file()], cursor: null });
    await pending;
    assert.equal(f.events.length, 0);
    assert.equal(f.results.at(-1)[2], 'library_cancelled');
  }
});

test('replacement and explicit close abort only the owned read request', async () => {
  const f = fixture();
  let resolve;
  f.setNext(() => new Promise(done => { resolve = done; }));
  const pending = f.list({}, 'mcp_first');
  f.setNext({ items: [], cursor: null });
  await f.list({ query: 'new' });
  assert.equal(f.calls[0].init.signal.aborted, true);
  resolve({ items: [file()], cursor: null });
  await pending;
  assert.equal(f.events.length, 1);
  f.setNext(() => new Promise(done => { resolve = done; }));
  const closing = f.list({ operation: 'refresh' }, 'mcp_closing');
  assert.equal(f.service.cancel('mcp_other'), false);
  assert.equal(f.service.cancel('mcp_closing'), true);
  resolve({ items: [file()], cursor: null });
  await closing;
  assert.equal(f.events.length, 1);
});

test('three failures cool down repeated reads without destroying known rows', async () => {
  const f = fixture();
  await f.list();
  f.setNext(new Error('http_503'));
  for (let i = 0; i < 4; i++) await f.list({ operation: 'refresh' });
  assert.equal(f.calls.length, 4);
  assert.equal(f.results.at(-1)[2], 'library_retry_later');
  assert.ok(f.events.every(event => event.items.length === 2));
});

test('unready identity, unobserved folders and malformed requests do not call endpoints', async () => {
  const f = fixture();
  for (const value of [{ directoryHandle: 'directory-synthetic' }, { operation: 'delete' }, { query: 'x'.repeat(201) }]) {
    await f.list(value);
  }
  await f.list({ directoryHandle: 'library_' + 'a'.repeat(32) });
  f.setAuth('');
  await f.list();
  assert.equal(f.calls.length, 0);
  assert.equal(f.events.length, 0);
  assert.equal(f.results.at(-1)[2], 'library_identity_not_ready');
});

test('ordinary library downloads reuse the byte owner without inventing conversation scope', async () => {
  const f = fixture();
  const calls = [];
  f.root.__elonChatGptPrivateLibraryDownload = { async run(_, job, isCurrent, authorize, libraryId) {
    assert.ok(isCurrent(job));
    calls.push({ path: job.entry.path, libraryId });
    return 'download_saved';
  } };
  await f.list();
  const row = f.events[0].items[1], receipts = [];
  await f.root.__elonChatGptPrivateFileDownload.start(JSON.stringify({ version: 1, path: '/library',
    leaseId: '00000000-0000-4000-8000-000000000001', documentToken: f.root.__elonChatGptDocumentToken,
    href: f.root.location.href, name: row.name, downloadHandle: row.downloadHandle }), (...args) => receipts.push(args));
  assert.deepEqual(calls, [{ path: '/library', libraryId: 'libfile_synthetic' }]);
  assert.deepEqual(receipts, [['download_conversation_file', true, 'download_saved']]);
  assert.equal(f.calls.length, 1);
});

test('unsupported external and saved library entities are visible but never treated as ordinary downloads', async () => {
  const f = fixture();
  f.setNext({ items: [file(), { ...file('external-gdrive:file:synthetic'), external_account: {} },
    { ...file('libfile_saved'), saved_entity: {} }, { ...file('libfile_cloud'), cloud_doc_url: 'https://example.invalid' }], cursor: null });
  await f.list();
  assert.match(f.events[0].items[0].downloadHandle, /^download_/);
  assert.ok(f.events[0].items.slice(1).every(row => row.downloadHandle === ''));
});

test('a different folder parent is not silently shown as the selected directory', async () => {
  const f = fixture();
  await f.list();
  const directoryHandle = f.events[0].items[0].handle;
  f.setNext({ items: [{ ...file(), parent_directory_id: 'different-directory' }], cursor: null });
  await f.list({ directoryHandle });
  assert.equal(f.events.at(-1).items.length, 0);
  assert.equal(f.events.at(-1).partial, true);
  await f.list({ directoryHandle, query: 'fixture' });
  assert.equal(f.events.at(-1).items.length, 1);
});

test('large catalogues stop at an explicit partial bound instead of polling or allocating unlimited rows', async () => {
  const f = fixture();
  f.setNext({ items: Array.from({ length: 501 }, (_, i) => file('libfile_' + i)), cursor: 'more' });
  await f.list();
  assert.equal(f.events[0].items.length, 500);
  assert.equal(f.events[0].hasMore, false);
  assert.equal(f.events[0].partial, true);
  assert.equal(f.calls.length, 1);
});

test('closed library owner cannot produce more snapshots or reads', async () => {
  const f = fixture();
  await f.list();
  f.service.dispose();
  await f.list();
  assert.equal(f.calls.length, 1);
  assert.equal(f.events.length, 1);
  assert.equal(f.results.at(-1)[1], false);
});

function coldIdentity(f) {
  f.setAuth('');
  let resolve, attempts = 0;
  const waiting = new Promise(done => { resolve = done; });
  f.root.__elonChatGptPrivateTransport.acquireSameOriginRequestHeaders = () => { attempts++; return waiting; };
  return { resolve: (headers = { Authorization: 'Bearer synthetic-warmed-account' }) => {
    f.setAuth(headers?.Authorization || '');
    resolve(headers);
  }, attempts: () => attempts };
}

test('cold library reads wait for the shared identity owner; warm cache never reacquires or rereads', async () => {
  const f = fixture(), auth = coldIdentity(f);
  const pending = f.list();
  assert.equal(f.results.length, 0);
  assert.equal(f.calls.length, 0);
  auth.resolve();
  await pending;
  assert.equal(f.results.at(-1)[2], 'library_ready');
  await f.list();
  assert.equal(f.results.at(-1)[2], 'library_cached');
  assert.equal(auth.attempts(), 1);
  assert.equal(f.calls.length, 1);
});

test('closing during identity preparation cancels immediately and ignores later readiness', async () => {
  for (const close of [f => f.service.cancel('mcp_cold'), f => f.service.cancelActiveRead(), f => f.service.dispose()]) {
    const f = fixture(), auth = coldIdentity(f);
    const pending = f.list({}, 'mcp_cold');
    assert.equal(f.service.cancel('mcp_other'), false);
    close(f);
    await pending;
    assert.equal(f.results.at(-1)[2], 'library_cancelled');
    auth.resolve();
    await Promise.resolve();
    assert.equal(f.calls.length, 0);
    assert.equal(f.events.length, 0);
  }
});

test('superseded cold reads share preparation but only the newest selection can publish', async () => {
  const f = fixture(), auth = coldIdentity(f);
  const first = f.list({ query: 'old' });
  const second = f.list({ query: 'new' });
  await first;
  assert.equal(f.results.at(-1)[2], 'library_cancelled');
  auth.resolve();
  await second;
  assert.equal(f.events.length, 1);
  assert.equal(f.events[0].query, 'new');
  assert.equal(f.calls.length, 1);
});

test('identity preparation cannot cross navigation, document, transport or account changes', async () => {
  for (const change of [f => { f.root.location.href = 'https://chatgpt.com/c/other'; },
    f => { f.root.__elonChatGptDocumentToken = 'doc_replaced_catalog'; },
    f => { f.root.__elonChatGptPrivateTransport = { ...f.root.__elonChatGptPrivateTransport }; },
    f => f.setAuth('Bearer synthetic-different-account')]) {
    const f = fixture(), auth = coldIdentity(f);
    const pending = f.list();
    auth.resolve();
    change(f);
    await pending;
    assert.equal(f.results.at(-1)[2], 'library_cancelled');
    assert.equal(f.calls.length, 0);
    assert.equal(f.events.length, 0);
  }
});

test('missing, rejected or timed-out identity never becomes an empty successful library', async () => {
  for (const outcome of ['missing', 'rejected', 'timeout']) {
    const f = fixture(), auth = coldIdentity(f);
    let timeout;
    f.root.setTimeout = (callback, ms) => { assert.equal(ms, 7000); timeout = callback; return 1; };
    f.root.clearTimeout = () => {};
    if (outcome === 'rejected') f.root.__elonChatGptPrivateTransport.acquireSameOriginRequestHeaders = async () => {
      throw new Error('auth_cooldown');
    };
    const pending = f.list();
    if (outcome === 'missing') auth.resolve(null);
    if (outcome === 'timeout') timeout();
    await pending;
    assert.equal(f.results.at(-1)[2], 'library_identity_not_ready');
    auth.resolve();
    await Promise.resolve();
    assert.equal(f.events.length, 0);
    assert.equal(f.calls.length, 0);
  }
});

test('a mutation that starts during identity preparation prevents a conflicting catalog refresh', async () => {
  const f = fixture(), auth = coldIdentity(f);
  const pending = f.list();
  f.root.__elonChatGptPrivateLibraryMutations = { busy: () => true };
  auth.resolve();
  await pending;
  assert.equal(f.results.at(-1)[2], 'library_mutation_busy');
  assert.equal(f.calls.length, 0);
});

test('identity wait and catalog read share a deadline shorter than the native receipt watcher', async t => {
  let now = Date.now();
  t.mock.method(Date, 'now', () => now);
  const f = fixture(), auth = coldIdentity(f);
  const pending = f.list();
  now += 6500;
  auth.resolve();
  await pending;
  assert.equal(f.calls[0].budget.timeoutMs, 7500);

  const expired = fixture(), late = coldIdentity(expired);
  const waiting = expired.list();
  now += 14001;
  late.resolve();
  await waiting;
  assert.equal(expired.calls.length, 0);
  assert.equal(expired.events.length, 0);
  assert.equal(expired.results.at(-1)[2], 'library_read_failed');
});

test('the existing auth owner shares one session read across superseded cold library requests', async () => {
  const f = fixture();
  const read = f.root.__elonChatGptPrivateJsonRequest.request;
  let sessionReads = 0, complete;
  f.root.fetch = () => {};
  f.root.__elonChatGptPrivateAuthContextEnabled = true;
  f.root.__elonChatGptPrivateJsonRequest.request = async (root, url, init, budget) => {
    if (url !== '/api/auth/session') return read(root, url, init, budget);
    sessionReads++;
    assert.equal(init.credentials, 'include');
    assert.equal(budget.timeoutMs, 5000);
    return new Promise(done => { complete = done; });
  };
  const auth = require('../android/app/src/main/assets/chatgpt_web_private_auth_context').create(f.root);
  f.root.__elonChatGptPrivateTransport = {
    copySameOriginRequestHeaders: auth.copyRequestHeaders,
    acquireSameOriginRequestHeaders: auth.acquireRequestHeaders,
  };
  const old = f.list({ query: 'old' });
  const fresh = f.list({ query: 'current' });
  complete({ payload: { accessToken: 'synthetic-session-access-token' } });
  await Promise.all([old, fresh]);
  await f.list({ query: 'current' });
  assert.equal(sessionReads, 1);
  assert.equal(f.calls.length, 1);
  assert.equal(f.results.at(-1)[2], 'library_cached');
  assert.ok(f.events.every(event => event.query === 'current'));
});

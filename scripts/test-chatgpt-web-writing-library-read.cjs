'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const fs = require('node:fs'), path = require('node:path');
const { harness } = require('./fixtures/chatgpt-writing-library-harness.cjs');
const reader = require('../android/app/src/main/assets/chatgpt_web_writing_library_read.js');
const turn = () => new Promise(resolve => setImmediate(resolve));

function setup(missing = false) {
  const h = harness();
  h.change({ hydratedFromLibrary: false });
  if (missing) h.state.session = undefined;
  const read = { calls: [], hydrated: [], owner: { id: 'account-a', normalizedAccountUserId: 'user-a',
    features: ['library_shared_content_available'] },
    payload: { library_file_id: h.lid, content: 'Original\n', current_version: 4, content_backing_kind: 'habitat' } };
  h.shared.writingLibraryAccount = () => read.owner;
  h.shared.writingLibraryReadHeaders = id => ({ 'X-Test-Account': encodeURIComponent(id) });
  h.page.__elonChatGptWritingLibraryRead = reader;
  h.page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders = () => ({
    'x-test-account': 'stale-account', 'X-Test-Session': 'synthetic' });
  h.store.hydrateSessionFromLibrary = input => {
    read.hydrated.push(input);
    if (read.result) return read.result;
    h.change({ libraryFileId: input.libraryFileId, draftContent: input.content, lastSavedContent: input.content,
      baseVersionNumber: input.versionNumber, hydratedFromLibrary: true, dirty: false, hasPendingEditorChanges: false,
      latestIssuedSaveSequence: h.state.session?.latestIssuedSaveSequence ?? 0, inFlightSaveSequence: null, draftSource: 'seed' });
    return 'applied';
  };
  const request = h.page.__elonChatGptPrivateJsonRequest.request;
  h.page.__elonChatGptPrivateJsonRequest.request = async (page, url, init, limits) => {
    if (!url.includes('/files/library/')) return request(page, url, init, limits);
    read.calls.push({ url, init, limits });
    if (read.hold) await new Promise(resolve => { read.resume = resolve; });
    read.onRead?.();
    if (read.error) throw Error(read.error);
    return { payload: read.payload };
  };
  return { ...h, read };
}

for (const missing of [true, false]) test((missing ? 'missing' : 'seed-only') + ' session reads versioned text once, then reuses the official save queue', async () => {
  const h = setup(missing), prepared = await h.prepare();
  assert.equal(prepared.code, 'writing_ready');
  assert.equal(h.read.calls.length, 1);
  const call = h.read.calls[0];
  assert.equal(call.url, '/backend-api/files/library/shared/files/libfile_a/text');
  assert.equal(call.init.method, 'GET'); assert.equal(call.init.credentials, 'same-origin');
  assert.equal(call.init.cache, 'no-store'); assert.equal(call.init.redirect, 'error');
  assert.equal(call.init.headers['x-test-account'], 'account-a');
  assert.equal(Object.keys(call.init.headers).filter(k => k.toLowerCase() === 'x-test-account').length, 1);
  assert.equal(call.init.headers['cache-control'], 'no-cache, no-store');
  assert.equal(call.init.body, undefined);
  assert.deepEqual(call.limits, { timeoutMs: 6000, maxBytes: 1024 * 1024, mode: 'json' });
  assert.deepEqual(h.read.hydrated, [{ libraryFileId: h.lid, content: 'Original\n', versionNumber: 4 }]);
  assert.equal((await h.save(prepared.ticket)).code, 'writing_saved');
  assert.equal((await h.save(prepared.ticket, 'Another edit')).code, 'writing_saved');
  assert.equal((await h.prepare()).code, 'writing_ready');
  assert.equal(h.read.calls.length, 1); assert.equal(h.state.posts.length, 2);
  assert.equal(h.state.session.draftContent, 'Another edit'); assert.equal(h.state.retains, 0);
});

test('a clean hydrated session never needs a Library read or its feature flag', async () => {
  const h = setup(); h.change({ hydratedFromLibrary: true }); h.read.owner.features = [];
  assert.equal((await h.prepare()).code, 'writing_ready');
  assert.equal(h.read.calls.length, 0);
});

test('a light account fallback and sediment text use the same reviewed versioned protocol', async () => {
  const h = setup(true); h.shared.writingLibraryAccount = () => null; h.shared.mq = () => h.read.owner;
  h.read.payload.content_backing_kind = 'sediment'; h.read.payload.current_version = 0;
  assert.equal((await h.prepare()).code, 'writing_ready');
  assert.equal(h.state.session.baseVersionNumber, 0);
});

test('missing account, feature, scope headers or hydration method never fetches or changes a draft', async () => {
  for (const change of [h => { h.read.owner = null; }, h => { h.read.owner.features = []; },
    h => { h.read.owner.normalizedAccountUserId = ''; }, h => { h.shared.writingLibraryReadHeaders = () => undefined; },
    h => { h.shared.writingLibraryReadHeaders = () => ({}); }, h => { h.store.hydrateSessionFromLibrary = null; }]) {
    const h = setup(); change(h); const before = JSON.stringify(h.state.session);
    assert.equal((await h.prepare()).ok, false);
    assert.equal(h.read.calls.length, 0); assert.equal(h.read.hydrated.length, 0);
    assert.equal(JSON.stringify(h.state.session), before); assert.equal(h.state.posts.length, 0);
  }
});

test('invalid versioned response, legacy-only text and mismatched content cannot authorize a save', async () => {
  for (const patch of [{ library_file_id: 'libfile_other' }, { current_version: undefined }, { current_version: '4' },
    { current_version: -1 }, { current_version: 1.5 }, { current_version: Number.MAX_SAFE_INTEGER + 1 },
    { content_backing_kind: 'unknown' }, { content: null }, { content: 'x'.repeat(120001) }, { content: 'Newer version' }]) {
    const h = setup(true); Object.assign(h.read.payload, patch);
    assert.equal((await h.prepare()).ok, false);
    assert.equal(h.read.calls.length, 1); assert.equal(h.read.hydrated.length, 0);
    assert.equal(h.state.session, undefined); assert.equal(h.state.posts.length, 0);
  }
});

test('version conflict preserves the existing seed and returns the existing native conflict code', async () => {
  const h = setup(), before = JSON.stringify(h.state.session); h.read.payload.content = 'Changed in Library';
  assert.equal((await h.prepare()).code, 'writing_version_conflict');
  assert.equal(JSON.stringify(h.state.session), before);
});

test('dirty, editing or saving seeds cannot be overwritten by a Library read', async () => {
  for (const patch of [{ dirty: true }, { hasPendingEditorChanges: true }, { inFlightSaveSequence: 1 },
    { draftContent: 'Unsaved edit' }, { lastSavedContent: 'Different content' }, { libraryFileId: 'libfile_other' }]) {
    const h = setup(); h.change(patch); const before = JSON.stringify(h.state.session);
    assert.equal((await h.prepare()).code, 'writing_web_edit_pending');
    assert.equal(h.read.calls.length, 0); assert.equal(JSON.stringify(h.state.session), before);
  }
});

test('async reads reject account, runtime, route and session changes before hydration', async () => {
  for (const change of [h => { h.read.owner.id = 'account-b'; }, h => { h.read.owner.normalizedAccountUserId = 'user-b'; },
    h => { h.read.owner.features = []; }, h => { h.state.current = false; },
    h => { h.page.location.href += '?other'; }, h => { h.page.document = {}; },
    h => { h.page.__elonChatGptDocumentToken = 'doc_new'; },
    h => { h.bindings.peek = () => ({}); }, h => h.change({ hasPendingEditorChanges: true }),
    h => { h.state.session.draftContent = 'In-place edit'; }, h => h.change({ latestIssuedSaveSequence: 5 })]) {
    const h = setup(); h.read.hold = true;
    const pending = h.prepare(); await turn(); assert.equal(h.read.calls.length, 1);
    change(h); const before = JSON.stringify(h.state.session); h.read.resume();
    assert.equal((await pending).ok, false);
    assert.equal(h.read.hydrated.length, 0); assert.equal(JSON.stringify(h.state.session), before);
    assert.equal(h.state.posts.length, 0);
  }
});

test('a changed conversation block during hydration cannot yield a native save ticket', async () => {
  const h = setup(); h.read.onRead = () => { h.state.message.metadata.writing_blocks[h.bid].content = 'Other local edit'; };
  assert.equal((await h.prepare()).code, 'writing_web_edit_pending');
  assert.equal(h.state.posts.length, 0);
});

test('official hydration rejection or missing acknowledgement is not promoted to readiness', async () => {
  for (const result of ['skipped_stale_version', 'skipped_local_changes', 'unexpected']) {
    const h = setup(); h.read.result = result;
    assert.equal((await h.prepare()).code, result === 'skipped_local_changes' ? 'writing_web_edit_pending' : 'writing_version_conflict');
    assert.equal(h.state.session.hydratedFromLibrary, false); assert.equal(h.state.posts.length, 0);
  }
  const h = setup(); h.store.hydrateSessionFromLibrary = () => 'applied';
  assert.equal((await h.prepare()).code, 'writing_version_conflict');
});

test('HTTP and timeout failures do not probe legacy endpoints, retry or poison a later explicit attempt', async () => {
  for (const error of ['http_404', 'http_403', 'timeout', 'invalid_json', 'too_large']) {
    const h = setup(true); h.read.error = error;
    assert.equal((await h.prepare()).ok, false);
    assert.equal(h.read.calls.length, 1); assert.equal(h.read.hydrated.length, 0); assert.equal(h.state.posts.length, 0);
    h.read.error = null;
    assert.equal((await h.prepare()).code, 'writing_ready'); assert.equal(h.read.calls.length, 2);
  }
});

test('expired reads cannot hydrate late even when an HTTP wrapper completes after the deadline', async () => {
  const h = setup(true); let now = 100;
  h.read.onRead = () => { now = 200; };
  await assert.rejects(reader.hydrate(h.page, { shared: h.shared, bindings: h.bindings, current: () => true, now: () => now },
    { libraryFileId: h.lid, content: 'Original\n' }, h.store, 150), /writing_timeout/);
  assert.equal(h.read.calls[0].limits.timeoutMs, 50); assert.equal(h.read.hydrated.length, 0);
});

test('production adapter loads the reader before the coordinator and context', () => {
  const source = fs.readFileSync(path.join(__dirname,
    '../android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebAdapterAssets.kt'), 'utf8');
  const names = ['chatgpt_web_writing_library_read.js', 'chatgpt_web_writing_library_session.js', 'chatgpt_web_writing_block_context.js'];
  for (let i = 0; i < names.length; i++) {
    assert.ok(source.indexOf(names[i]) >= 0);
    if (i) assert.ok(source.indexOf(names[i - 1]) < source.indexOf(names[i]));
  }
});

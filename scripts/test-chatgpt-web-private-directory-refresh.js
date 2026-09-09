'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const { assets, fixture, response, deferred, flush } = require('./fixtures/chatgpt-directory-refresh');

test('global directory fetch is private, single-flight and independent of the composer', async () => {
  const read = deferred(), f = fixture(() => read.promise);
  const first = f.controller.refresh(), second = f.controller.refresh();
  assert.equal(first, second);
  await flush();
  assert.equal(f.calls.length, 2);
  assert.equal(f.calls[0].url, '/backend-api/conversations?offset=0&limit=28&order=updated&is_archived=false');
  assert.equal(f.calls[0].init.credentials, 'same-origin');
  assert.equal(f.calls[0].init.method, 'GET');
  read.resolve(response({ items: [{ id: 'fixture', title: 'Fixture' }] }));
  const result = await first;
  assert.equal(result.ok, true);
  assert.equal(result.code, 'directory_partial');
  assert.equal(result.complete, false);
  assert.equal(f.accepted.length, 2);
  assert.deepEqual(f.accepted[0][0], { family: 'conversations', projectId: '', replace: false });
});

test('cold identity is acquired without a DOM login flag', async () => {
  const f = fixture(), owner = f.root.__elonChatGptPrivateTransport;
  let ready = false;
  owner.copySameOriginRequestHeaders = () => ready ? { ...f.headers } : null;
  owner.acquireSameOriginRequestHeaders = async () => { ready = true; return { ...f.headers }; };
  assert.equal((await f.controller.refresh()).ok, true);
  assert.equal(f.calls.length, 2);
});

test('missing runtime identity cannot send an unauthenticated read or report absent capability', async () => {
  const f = fixture();
  delete f.root.__elonChatGptDocumentToken;
  assert.equal((await f.controller.refresh()).code, 'directory_identity_not_ready');
  assert.equal(f.calls.length, 0);
});

for (const kind of ['account', 'document', 'origin']) {
  test(`late directory response after ${kind} changes cannot update the cache`, async () => {
    const read = deferred(), f = fixture(() => read.promise);
    const pending = f.controller.refresh();
    await flush();
    if (kind === 'account') f.headers['chatgpt-account-id'] = 'another-account';
    if (kind === 'document') f.root.__elonChatGptDocumentToken = 'doc_replaced_fixture';
    if (kind === 'origin') f.root.location.origin = 'https://example.com';
    read.resolve(response({ items: [] }));
    assert.equal((await pending).ok, false);
    assert.equal(f.accepted.length, 0);
  });
}

test('identity switch during acquisition cannot make a request with old context', async () => {
  const auth = deferred(), f = fixture();
  f.root.__elonChatGptPrivateTransport.acquireSameOriginRequestHeaders = () => auth.promise;
  const pending = f.controller.refresh();
  f.headers['chatgpt-account-id'] = 'another-account';
  auth.resolve({ ...f.headers });
  assert.equal((await pending).code, 'directory_context_changed');
  assert.equal(f.calls.length, 0);
});

test('cancel releases a hanging credential wait without cancelling the shared identity owner', async () => {
  const f = fixture();
  f.root.__elonChatGptPrivateTransport.acquireSameOriginRequestHeaders = () => new Promise(() => {});
  const pending = f.controller.refresh();
  f.controller.cancel();
  assert.equal((await pending).code, 'directory_cancelled');
  assert.equal(f.calls.length, 0);
});

test('timeout settles an ignored abort and a subsequent refresh can succeed', async () => {
  const read = deferred(), f = fixture(() => read.promise);
  f.root.setTimeout = (callback, ms) => setTimeout(callback, ms === 4000 ? 5 : ms);
  assert.equal((await f.controller.refresh()).code, 'directory_timeout');
  read.resolve(response({ items: [] }));
  await flush();
  assert.equal(f.accepted.length, 0);
  assert.equal((await f.controller.refresh()).ok, true);
  assert.equal(f.calls.length, 4);
});

function requestsFixture(refresh, flags = {}) {
  const events = [], receipts = [], dom = [];
  const window = { ...flags };
  vm.runInNewContext(fs.readFileSync(path.join(assets, 'chatgpt_web_adapter_conversation_directory_requests.js'), 'utf8'), { window });
  const controller = window.__elonChatGptConversationDirectoryRequests.create({
    conversationAdapter: { requestList: () => dom.push('list') },
    privateDirectory: { refresh, snapshot: () => ({ conversations: [{ id: 'fixture', title: 'Fixture' }], projects: [] }) },
    emitEvent: value => events.push(value), optional: (_, action) => action(),
  });
  return { controller, events, receipts, dom, respond: (...args) => receipts.push(args) };
}

test('real directory dispatcher uses private refresh rather than the absent sidebar', async () => {
  const f = requestsFixture(async () => ({ ok: true, code: 'directory_page_ready' }));
  f.controller.requestList({}, f.respond);
  await flush();
  assert.equal(f.dom.length, 0);
  assert.equal(f.events[0].collection.source, 'official_private');
  assert.equal(f.events[0].collection.complete, false);
  assert.equal(f.receipts[0][1], true);
});

test('private readiness failure does not trigger a DOM fallback or claim no sidebar capability', async () => {
  const f = requestsFixture(async () => ({ ok: false, code: 'directory_identity_not_ready' }));
  f.controller.requestList({}, f.respond);
  await flush();
  assert.equal(f.dom.length, 0);
  assert.equal(f.events.length, 0);
  assert.deepEqual(f.receipts[0], ['list_conversations', false, 'directory_identity_not_ready']);
});

test('explicitly disabled private transport preserves the existing official directory path', async () => {
  const f = requestsFixture(() => { throw new Error('disabled transport called'); }, {
    __elonChatGptPrivateConversationPrefetchEnabled: false,
  });
  f.controller.requestList({}, f.respond);
  await flush();
  assert.deepEqual(f.dom, ['list']);
});

test('cancelled refresh cannot deliver a late receipt to a different selection', async () => {
  const read = deferred(), f = requestsFixture(() => read.promise);
  f.controller.requestList({}, f.respond);
  await flush();
  f.controller.cancel();
  read.resolve({ ok: true });
  await flush();
  assert.equal(f.events.length, 0);
  assert.equal(f.receipts.length, 0);
});

test('production assets load the refresh owner before the directory', () => {
  const source = fs.readFileSync(path.join(__dirname,
    '../android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebAdapterAssets.kt'), 'utf8');
  assert(source.indexOf('chatgpt_web_private_directory_refresh.js') >= 0);
  assert(source.indexOf('chatgpt_web_private_directory_refresh.js') < source.indexOf('chatgpt_web_private_conversation_directory.js'));
  const names = Array.from(source.matchAll(/"([a-z0-9_]+\.js)"/g), match => match[1]);
  assert.doesNotThrow(() => new vm.Script(names.map(name => fs.readFileSync(path.join(assets, name), 'utf8')).join('\n')));
});

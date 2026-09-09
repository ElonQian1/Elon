'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const { assets, fixture, response, emptyPage, flush, deferred } = require('./fixtures/chatgpt-directory-refresh');

function dispatcher(root, directory, fallback) {
  const events = [], replies = [];
  vm.runInNewContext(fs.readFileSync(path.join(assets, 'chatgpt_web_adapter_conversation_directory_requests.js'), 'utf8'),
    { window: root });
  const controller = root.__elonChatGptConversationDirectoryRequests.create({
    conversationAdapter: { requestList: fallback || (() => assert.fail('Unexpected DOM')) },
    privateDirectory: directory, optional: (_, fn) => fn(), emitEvent: event => events.push(event),
  });
  const request = (requestId, projectScopeId = '') => controller.requestList({ requestId, projectScopeId }, (...args) => replies.push(args));
  return { controller, events, replies, request };
}
const snapshot = () => ({ conversations: [{ id: 'fixture-row', title: 'Synthetic fixture' }], projects: [] });

test('requested snapshots carry their owner and explicit continuation, passive updates cannot complete it', async () => {
  const f = fixture();
  const d = dispatcher(f.root, { refresh: async () => ({ ok: true, code: 'directory_partial', pages: 4, continueRefresh: true }), snapshot });
  d.request('directory_native_fixture');
  await flush();
  assert.equal(d.events[0].requestId, 'directory_native_fixture');
  assert.equal(d.events[0].continueRefresh, true);
  d.controller.emitSnapshot(null);
  assert.equal(d.events[1].requestId, null);
  assert.equal(d.events[1].continueRefresh, false);
});

test('superseded and cancelled responses cannot publish another refresh completion', async () => {
  const f = fixture(), waits = [deferred(), deferred()];
  let index = 0;
  const d = dispatcher(f.root, { refresh: () => waits[index++].promise, snapshot });
  d.request('directory_native_old');
  await flush();
  d.request('directory_native_new');
  await flush();
  waits[0].resolve({ ok: true });
  await flush();
  assert.equal(d.events.length, 0);
  d.controller.cancel();
  waits[1].resolve({ ok: true });
  await flush();
  assert.equal(d.replies.length, 0);
  assert.equal(d.events.length, 0);
});

test('explicit DOM mode uses the same request ownership and cancellation fence', () => {
  const f = fixture();
  f.root.__elonChatGptPrivateConversationPrefetchEnabled = false;
  let emit, respond;
  const d = dispatcher(f.root, {}, (_command, e, r) => { emit = e; respond = r; });
  d.request('directory_native_dom');
  emit({ type: 'conversation_snapshot', conversations: [] });
  assert.equal(d.events[0].requestId, 'directory_native_dom');
  d.controller.cancel();
  emit({ type: 'conversation_snapshot', conversations: [] });
  respond('list_conversations', true, '');
  assert.equal(d.events.length, 1);
  assert.equal(d.replies.length, 0);
});

test('bounded time budget advertises continuation but completed/capped/missing-cursor reads do not', async () => {
  let time = 0;
  const f = fixture(async url => {
    if (url.includes('/snorlax/')) return response(emptyPage(url));
    time += 3900;
    const offset = Number(new URL(url, 'https://chatgpt.com').searchParams.get('offset'));
    return response({ items: [{ id: 'row-' + offset, title: 'Synthetic fixture' }], offset, limit: 28, total: 140 });
  });
  f.root.performance = { now: () => time };
  assert.equal((await f.controller.refresh()).continueRefresh, true);
  assert.equal((await f.controller.refresh()).continueRefresh, false);
  const capped = fixture(async url => url.includes('/snorlax/') ? response(emptyPage(url)) : response({
    items: Array.from({ length: 28 }, (_, n) => ({ id: 'row-' + new URL(url, 'https://chatgpt.com').searchParams.get('offset') + '-' + n, title: 'Fixture' })),
    offset: Number(new URL(url, 'https://chatgpt.com').searchParams.get('offset')), limit: 28, total: 1000,
  }));
  const result = await capped.controller.refresh();
  assert.equal(result.truncated, true);
  assert.equal(result.continueRefresh, false);
  const unknown = fixture(async url => url.includes('/snorlax/') ? response({ items: [] }) : response(emptyPage(url)));
  assert.equal((await unknown.controller.refresh()).continueRefresh, false);
});

test('project contents retain their own failed cursor and resume it independently', async () => {
  let fail = true;
  const f = fixture(async url => {
    if (!url.includes('/gizmos/g-p-')) return response(emptyPage(url));
    const cursor = new URL(url, 'https://chatgpt.com').searchParams.get('cursor');
    if (cursor !== '0' && fail) return response({}, 503);
    return response({ items: [{ id: cursor === '0' ? 'first' : 'second', title: 'Fixture', owner: {} }],
      cursor: cursor === '0' ? 'opaque-next' : null });
  });
  assert.equal((await f.controller.refresh('g-p-fixture')).ok, false);
  await f.controller.refresh();
  fail = false;
  assert.equal((await f.controller.refresh('g-p-fixture')).complete, true);
  const calls = f.calls.filter(c => c.url.includes('/gizmos/g-p-'));
  assert.deepEqual(calls.map(c => new URL(c.url, 'https://chatgpt.com').searchParams.get('cursor')), ['0', 'opaque-next', 'opaque-next']);
  assert.deepEqual(JSON.parse(f.accepted.at(-1)[1]).items.map(i => i.id), ['first', 'second']);
});

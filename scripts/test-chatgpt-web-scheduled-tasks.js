'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { webcrypto } = require('node:crypto');
const bridge = require('../android/app/src/main/assets/chatgpt_web_scheduled_tasks.js');

function setup() {
  const fixture = { user: 'user_one', account: 'personal', auth: 'opaque-runtime-one', calls: [], results: [], events: [] };
  const page = { location: { origin: 'https://chatgpt.com' }, crypto: webcrypto, __elonChatGptDocumentToken: 'doc_test_123',
    __elonChatGptPrivateAuthContext: { acquireRequestHeaders: async () => ({}) },
    __elonChatGptPrivateConversationShareContract: { create: () => ({ identity: () => fixture.auth }) },
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ 'chatgpt-account-id': fixture.account }) },
    __elonChatGptPrivateJsonRequest: { request: async (_, path, options) => {
      assert.equal(path, '/api/auth/session'); assert.equal(options.method, 'GET');
      return { payload: { user: { id: fixture.user } } };
    } },
    __elonChatGptPrivateTasks: { create: () => ({ run: async input => {
      fixture.calls.push(input); await fixture.wait;
      return fixture.result || { ok: true, code: 'tasks_ready', data: { items: [{ id: 'task_a', title: 'Selected topic' }], nextCursor: null } };
    } }) },
  };
  const api = bridge.create(page);
  fixture.run = input => new Promise(resolve => {
    const respond = (action, ok, code) => { fixture.results.push({ action, ok, code }); setImmediate(() => resolve({ ok, code })); };
    respond.requestId = 'mcp_abc123';
    assert.equal(api.handle('scheduled_tasks', { value: JSON.stringify(input) }, respond, event => fixture.events.push(event)), true);
  });
  fixture.page = page; fixture.api = api;
  return fixture;
}
const list = { operation: 'list', filter: 'scheduled', force: false };
test('production command reads without DOM or composer and keeps contents out of receipts', async () => {
  const f = setup(); assert.equal((await f.run(list)).ok, true);
  assert.equal(f.events[0].requestId, 'mcp_abc123'); assert.match(f.events[0].scope, /^[a-f0-9]{64}$/);
  assert.equal(f.events[0].result.data.items[0].title, 'Selected topic');
  assert.equal(JSON.stringify(f.results).includes('Selected topic'), false);
  assert.deepEqual(f.calls, [list]);
});
test('stable owner hash survives token rotation, but rejects another user or workspace', async () => {
  const f = setup(); await f.run(list); const scope = f.events[0].scope;
  f.auth = 'opaque-runtime-two'; await f.run({ ...list, expectedScope: scope });
  assert.equal(f.events[1].scope, scope); assert.deepEqual(f.calls[1], list);
  f.auth = 'opaque-runtime-three'; f.account = 'workspace_two';
  assert.equal((await f.run({ ...list, expectedScope: scope })).code, 'tasks_account_changed');
  f.auth = 'opaque-runtime-four'; f.account = 'personal'; f.user = 'user_two';
  assert.equal((await f.run({ ...list, expectedScope: scope })).code, 'tasks_account_changed');
  assert.equal(f.calls.length, 2);
});
test('auth changes during private read discard payload instead of sharing another account', async () => {
  const f = setup(); let release; f.wait = new Promise(resolve => { release = resolve; });
  const done = f.run(list);
  while (!f.calls.length) await new Promise(resolve => setImmediate(resolve));
  f.auth = 'different'; release();
  assert.equal((await done).code, 'tasks_context_changed'); assert.equal(f.events.length, 0);
});
test('parallel commands fail quickly and a finished command releases admission', async () => {
  const f = setup(); let release; f.wait = new Promise(resolve => { release = resolve; });
  const first = f.run(list); assert.equal((await f.run(list)).code, 'tasks_busy');
  release(); await first; assert.equal((await f.run(list)).ok, true);
});
test('unknown errors are fixed codes and never log private exception messages', async () => {
  const f = setup(); f.page.__elonChatGptPrivateAuthContext.acquireRequestHeaders = async () => { throw Error('private credential content'); };
  assert.equal((await f.run(list)).code, 'tasks_unavailable'); assert.equal(f.events.length, 0);
  assert.equal(f.api.handle('other', {}, () => {}, () => {}), false);
});

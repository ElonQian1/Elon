'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const policy = require('../android/app/src/main/assets/chatgpt_web_private_tasks_policy.js');
const transport = require('../android/app/src/main/assets/chatgpt_web_private_tasks.js');
const probe = require('../android/app/src/main/assets/chatgpt_web_private_tasks_probe.js');
const CID = '11111111-1111-4111-8111-111111111111';
const at = '2026-09-23T00:00:00Z';
const task = (extra = {}) => ({ id: 'task_fixture', title: 'Fixture topic', prompt: 'Private instructions',
  is_enabled: true, next_run_times: [], timing_mode: 'condition_watch',
  source_conversation_id: CID, conversation_id: CID, updated_at: at, last_run_time: null, ...extra });
const update = (extra = {}) => ({ id: 'message_fixture', created_at: at, content_text: '**Fixture** update',
  content: { content_type: 'text', parts: ['**Fixture** update'] }, metadata: {}, ...extra });
const list = (extra = {}) => ({ operation: 'list', filter: 'scheduled', force: false, ...extra });
function fixture() {
  let account = 'fixture_scope', now = 1000, next = () => ({ items: [task()], cursor: null });
  const requests = [], invalidations = [];
  const page = { location: { origin: 'https://chatgpt.com' }, document: {}, __elonChatGptDocumentToken: 'doc_fixture_token',
    __elonChatGptPrivateTasksPolicy: policy,
    __elonChatGptPrivateConversationShareContract: { create: () => ({ identity: () => account }) },
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({
      authorization: 'Bearer synthetic-fixture-only', cookie: 'never_forward', 'x-unrelated': 'never_forward',
    }) },
    __elonChatGptPrivateAuthContext: { invalidate: reason => invalidations.push(reason) },
    __elonChatGptPrivateJsonRequest: { request: async (_, url, init, limits) => {
      requests.push({ url, init, limits }); return { payload: await next() };
    } },
  };
  const api = transport.create(page, { now: () => now });
  return { api, page, requests, invalidations, setAccount: value => { account = value; },
    advance: ms => { now += ms; }, response: value => { next = typeof value === 'function' ? value : () => value; } };
}
test('normalizes scheduled tasks without instructions, credentials, or inferred completion', () => {
  const value = policy.page({ items: [task()], cursor: null });
  assert.equal(value.complete, true); assert.equal(value.items[0].enabled, true);
  assert.equal(value.items[0].timingMode, 'condition_watch'); assert.equal(value.items[0].nextRunAt, null);
  assert.equal(value.items[0].updatesPath, '/c/' + CID);
  assert.equal(JSON.stringify(value).includes('Private instructions'), false);
});
test('requires a confirmed page shape; partial pages are not complete', () => {
  assert.equal(policy.page({ items: [], cursor: 'next/page+1==' }).complete, false);
  for (const value of [null, {}, { items: [] }, { items: [], cursor: '' }, { items: [task(), task()], cursor: null },
    { items: [task({ is_enabled: 'true' })], cursor: null }, { items: [task({ next_run_times: [null] })], cursor: null },
    { items: [task({ source_conversation_id: 'https://wrong.test' })], cursor: null }]) {
    assert.throws(() => policy.page(value), /tasks_response_invalid/);
  }
});
test('distinguishes latest useful update, last run failure, attention and genuine no-update', () => {
  const data = policy.latest({ latest_update: update(), last_backing_run: { status: 'failed' } });
  assert.equal(data.state, 'update'); assert.equal(data.lastRunFailed, true);
  assert.equal(data.update.fromLatestRun, false); assert.deepEqual(data.update.content, update().content);
  assert.equal(policy.latest(null).state, 'no_update');
  assert.equal(policy.latest({ latest_update: null, last_backing_run: { status: 'completed' } }).lastRunFailed, false);
  assert.equal(policy.latest(update({ content_text: null, metadata: { automation_requires_user_action: true } })).state, 'requires_action');
  for (const value of [undefined, {}, [], update({ content_text: '' }), update({ metadata: null }), update({ created_at: 'tomorrow' })]) {
    assert.throws(() => policy.latest(value), /tasks_response_invalid/);
  }
});
test('uses only bounded same-origin GETs, caches reads and encodes observed opaque cursors', async () => {
  const f = fixture(); const first = await f.api.run(list()); assert.equal(first.ok, true);
  first.data.items[0].title = 'Changed by consumer';
  assert.equal((await f.api.run(list())).data.items[0].title, 'Fixture topic');
  assert.equal(f.requests.length, 1);
  const { init, limits } = f.requests[0];
  assert.equal(init.method, 'GET'); assert.equal(init.body, undefined); assert.equal(init.headers.cookie, undefined);
  assert.equal(init.headers['x-unrelated'], undefined); assert.equal(init.redirect, 'error');
  assert.equal(limits.timeoutMs, 7000); assert.equal(limits.maxBytes, 1024 * 1024);
  await f.api.run(list({ cursor: 'next/page+1==' }));
  assert.equal(new URL(f.requests[1].url, 'https://chatgpt.com').searchParams.get('cursor'), 'next/page+1==');
  f.advance(60001); await f.api.run(list()); assert.equal(f.requests.length, 3);
});
test('does not wait for a DOM composer, React runtime, or conversation route', async () => {
  const f = fixture(); f.page.location.pathname = '/scheduled';
  assert.equal((await f.api.run(list())).ok, true);
  f.response(null);
  assert.equal((await f.api.run({ operation: 'latest', id: 'task_fixture', force: false })).data.state, 'no_update');
  assert.equal(f.requests[1].url, '/backend-api/automation/task_fixture/latest_backing_run?include_snapshot=true');
});
test('rejects arbitrary endpoints, writes, invalid identifiers and filters before requesting', async () => {
  const f = fixture();
  for (const input of [list({ filter: 'unknown' }), list({ url: '/api/other' }), list({ force: 1 }),
    { operation: 'remove', id: 'task_fixture', force: true }, { operation: 'latest', id: '../task', force: false }]) {
    assert.equal((await f.api.run(input)).code, 'tasks_request_invalid');
  }
  assert.equal(f.requests.length, 0);
});
test('coalesces simultaneous reads and never shares mutable responses', async () => {
  const f = fixture(); let release;
  f.response(() => new Promise(resolve => { release = resolve; }));
  const first = f.api.run(list()), second = f.api.run(list({ force: true }));
  assert.equal(f.requests.length, 1); release({ items: [task()], cursor: null });
  const [a, b] = await Promise.all([first, second]); a.data.items[0].title = 'changed';
  assert.equal(b.data.items[0].title, 'Fixture topic');
});
test('retains a good snapshot on timeout or malformed response, without declaring empty success', async () => {
  const f = fixture(); await f.api.run(list()); f.advance(60001);
  f.response(() => { throw Error('request_timeout'); });
  const failed = await f.api.run(list()); assert.equal(failed.ok, false); assert.equal(failed.stale.items.length, 1);
  await f.api.run(list()); assert.equal(f.requests.length, 2);
  f.response({ items: 'broken' }); assert.equal((await f.api.run(list({ force: true }))).code, 'tasks_response_invalid');
  f.response({ items: [], cursor: null }); assert.equal((await f.api.run(list({ force: true }))).data.items.length, 0);
});
test('rate limits also constrain explicit refresh; manual retry can recover transient failure', async () => {
  const f = fixture(); f.response(() => { throw Error('http_429'); });
  assert.equal((await f.api.run(list())).code, 'tasks_rate_limited');
  await f.api.run(list({ force: true })); assert.equal(f.requests.length, 1);
  f.advance(60000); f.response({ items: [], cursor: null });
  assert.equal((await f.api.run(list({ force: true }))).ok, true);
});
test('account or document replacement rejects late data and does not invalidate the new account', async () => {
  for (const replace of [f => f.setAccount('other_scope'), f => { f.page.document = {}; },
    f => { f.page.__elonChatGptDocumentToken = 'doc_changed_token'; }]) {
    const f = fixture(); let release;
    f.response(() => new Promise(resolve => { release = resolve; }));
    const first = f.api.run(list()); replace(f); release({ items: [task()], cursor: null });
    assert.equal((await first).code, 'tasks_context_changed');
    f.response({ items: [], cursor: null }); assert.equal((await f.api.run(list())).data.items.length, 0);
    assert.equal(f.invalidations.length, 0);
  }
});
test('authentication rejection removes cached private data; absent auth never makes a request', async () => {
  const f = fixture(); await f.api.run(list()); f.response(() => { throw Error('http_401'); });
  const result = await f.api.run(list({ force: true }));
  assert.equal(result.code, 'tasks_auth_required'); assert.equal(result.stale, undefined);
  assert.equal(f.invalidations.length, 1); f.setAccount(null);
  assert.equal((await f.api.run(list())).code, 'tasks_auth_unavailable'); assert.equal(f.requests.length, 2);
});
test('caps concurrent distinct reads and bounded cache cardinality', async () => {
  const f = fixture(); const releases = [];
  f.response(() => new Promise(resolve => releases.push(resolve)));
  const reads = Array.from({ length: 4 }, (_, i) => f.api.run(list({ cursor: 'cursor' + i })));
  assert.equal((await f.api.run(list({ cursor: 'extra' }))).code, 'tasks_busy');
  releases.forEach(release => release({ items: [], cursor: null })); await Promise.all(reads);
  f.response({ items: [], cursor: null });
  for (let i = 0; i < 16; i++) await f.api.run(list({ cursor: 'new' + i }));
  const before = f.requests.length; await f.api.run(list({ cursor: 'cursor0' })); assert.equal(f.requests.length, before + 1);
});
test('MCP probe returns only bounded structural evidence, never titles, ids or result text', async () => {
  const f = fixture(); let index = 0;
  f.page.__elonChatGptPrivateTasks = { create: () => f.api };
  f.response(() => ++index === 1 ? { items: [task()], cursor: null } : update());
  const p = probe.create(f.page);
  const result = await new Promise(resolve => assert.equal(p.handle('private_protocol_probe', { value: 'scheduled_tasks' },
    (_, ok, detail) => resolve({ ok, detail })), true));
  assert.equal(result.ok, true); const data = JSON.parse(result.detail);
  assert.equal(data.catalogCount, 1); assert.equal(data.cacheHit, true); assert.equal(data.latestState, 'update');
  assert.doesNotMatch(result.detail, /Fixture|task_fixture|message_fixture|Bearer|instructions/);
});
test('assets load after the identity contract and diagnostic code accepts the versioned receipt', () => {
  const root = path.join(__dirname, '../android/app/src/main');
  const assets = fs.readFileSync(path.join(root, 'kotlin/com/elon/app/chatgptweb/ChatGptWebAdapterAssets.kt'), 'utf8');
  assert.ok(assets.indexOf('chatgpt_web_private_tasks.js') > assets.indexOf('chatgpt_web_private_conversation_share_contract.js'));
  assert.ok(assets.indexOf('chatgpt_web_private_tasks_policy.js') < assets.indexOf('chatgpt_web_private_tasks.js'));
  assert.match(fs.readFileSync(path.join(root, 'kotlin/com/elon/app/chatgptweb/ChatGptWebPrivateProtocolEvidence.kt'), 'utf8'), /elon\.scheduled_tasks_probe\.v1/);
});

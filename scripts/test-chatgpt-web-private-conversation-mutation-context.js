'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const { fixture, response, deferred } = require('./fixtures/chatgpt-conversation-mutation.js');
const flush = () => new Promise(resolve => setImmediate(resolve));
const path = '/c/fixture-conversation';
const changes = {
  document: f => { f.root.__elonChatGptDocumentToken = 'doc_replaced_fixture'; },
  account: f => { f.headers['chatgpt-account-id'] = 'another-synthetic-account'; },
  credentials: f => { f.headers.Authorization = 'Bearer replacement-synthetic-auth'; },
  origin: f => { f.root.location.origin = 'https://example.com'; },
  owner: f => { f.root.__elonChatGptPrivateTransport = { ...f.privateTransport }; },
};

for (const [label, change] of Object.entries(changes)) {
  test(`no write after ${label} changes during credential acquisition`, async () => {
    const auth = deferred(), f = fixture(async () => response(200, {}));
    f.privateTransport.acquireSameOriginRequestHeaders = () => auth.promise;
    const pending = f.transport.rename(path, 'Fixture title');
    change(f);
    auth.resolve({ ...f.headers });
    const result = await pending;
    assert.equal(result.code, 'mutation_context_changed');
    assert.equal(result.attempted, false);
    assert.equal(f.calls.length, 0);
    assert.equal(f.accepted.length, 0);
  });
}

test('missing document or real account context cannot authorize a private write', async () => {
  for (const label of ['document', 'account']) {
    const f = fixture(async () => response(200, {}));
    if (label === 'document') delete f.root.__elonChatGptDocumentToken;
    else delete f.headers.Authorization;
    const result = await f.transport.setPinned(path, true);
    assert.equal(result.ok, false);
    assert.equal(result.attempted, false);
    assert.equal(f.calls.length, 0);
  }
});

test('unavailable credential owner returns a structured rejection without throwing', async () => {
  const f = fixture(async () => response(200, {}));
  f.privateTransport.copySameOriginRequestHeaders = () => { throw new Error('fixture disposed'); };
  const result = await f.transport.rename(path, 'Fixture title');
  assert.equal(result.code, 'mutation_auth_unavailable');
  assert.equal(result.attempted, false);
  assert.equal(f.calls.length, 0);
});

test('cold credentials are acquired before one confirmed private write without composer readiness', async () => {
  const f = fixture((_, init) => response(200, init.method === 'GET' ? { title: 'Fixture title' } : {}));
  let ready = false;
  f.privateTransport.copySameOriginRequestHeaders = () => ready ? { ...f.headers } : null;
  f.privateTransport.acquireSameOriginRequestHeaders = async () => {
    ready = true;
    return { ...f.headers };
  };
  assert.equal((await f.transport.rename(path, 'Fixture title')).ok, true);
  assert.deepEqual(f.calls.map(c => c.init.method), ['PATCH', 'GET']);
});

test('late PATCH success cannot update a replacement account or start reconciliation', async () => {
  const pending = deferred(), f = fixture(() => pending.promise);
  const request = f.transport.rename(path, 'Fixture title');
  await flush();
  changes.account(f);
  pending.resolve(response(200, {}));
  const result = await request;
  assert.equal(result.code, 'mutation_context_changed');
  assert.equal(result.attempted, true);
  assert.deepEqual(f.calls.map(c => c.init.method), ['PATCH']);
  assert.equal(f.accepted.length, 0);
});

test('late GET success cannot overwrite a new document cache', async () => {
  const read = deferred(), f = fixture((_, init) => init.method === 'PATCH' ? response(200, {}) : read.promise);
  const request = f.transport.rename(path, 'Fixture title');
  await flush();
  assert.equal(f.accepted.length, 1);
  changes.document(f);
  read.resolve(response(200, { title: 'Fixture title' }));
  const result = await request;
  assert.equal(result.code, 'mutation_context_changed');
  assert.equal(result.attempted, true);
  assert.equal(f.accepted.length, 1);
  assert.deepEqual(f.calls.map(c => c.init.method), ['PATCH', 'GET']);
});

test('changed identity during failed write cannot retry or reconcile with old credentials', async () => {
  const write = deferred(), f = fixture(() => write.promise);
  const request = f.transport.setArchived(path, true);
  await flush();
  changes.credentials(f);
  write.resolve(Promise.reject(new Error('timeout')));
  const result = await request;
  assert.equal(result.code, 'mutation_context_changed');
  assert.equal(result.attempted, true);
  assert.deepEqual(f.calls.map(c => c.init.method), ['PATCH']);
  assert.equal(f.accepted.length, 0);
});

test('same-document conversation navigation does not retarget an explicit sidebar mutation', async () => {
  const auth = deferred(), f = fixture((_, init) => response(200,
    init.method === 'GET' ? { title: 'Fixture title' } : {}));
  f.root.location.href = 'https://chatgpt.com/c/initial-fixture';
  f.privateTransport.acquireSameOriginRequestHeaders = () => auth.promise;
  const request = f.transport.rename(path, 'Fixture title');
  f.root.location.href = 'https://chatgpt.com/c/another-fixture';
  auth.resolve({ ...f.headers });
  const result = await request;
  assert.equal(result.ok, true);
  assert.equal(f.calls[0].url, '/backend-api/conversation/fixture-conversation');
  assert.equal(f.accepted.every(item => item.id === 'fixture-conversation'), true);
});

test('failure cooldown belongs to its document/account, not the next account', async () => {
  let fail = true;
  const f = fixture((_, init) => response(fail ? 500 : 200,
    init.method === 'GET' ? { title: 'Fixture title' } : {}));
  assert.equal((await f.transport.rename(path, 'Fixture title')).code, 'mutation_http_500');
  assert.equal((await f.transport.rename(path, 'Fixture title')).code, 'mutation_circuit_open');
  changes.account(f);
  fail = false;
  assert.equal((await f.transport.rename(path, 'Fixture title')).ok, true);
  assert.deepEqual(f.calls.map(c => c.init.method), ['PATCH', 'PATCH', 'GET']);
});

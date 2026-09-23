'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const { webcrypto } = require('node:crypto');
const policy = require('../android/app/src/main/assets/chatgpt_web_group_project_policy');
const transport = require('../android/app/src/main/assets/chatgpt_web_group_project');
const identityContract = require('../android/app/src/main/assets/chatgpt_web_group_project_identity');
const shareContract = require('../android/app/src/main/assets/chatgpt_web_private_conversation_share_contract');
const directoryPages = require('../android/app/src/main/assets/chatgpt_web_private_directory_pages');
const bindingId = '11111111-1111-4111-8111-111111111111';
const projectId = 'g-p-' + 'a'.repeat(32);
const conversationId = '22222222-2222-4222-8222-222222222222';
const base = { bindingId, generation: 1, name: 'Synthetic group' };
const resource = () => ({ gizmo: { id: projectId, instructions: policy.createBody(base).instructions,
  memory_scope: 'project_v2', current_user_permission: { can_write: true } } });
function fixture() {
  let account = '["user_fixture","workspace_fixture"]';
  const calls = [];
  const identityCalls = [];
  const replies = [];
  let workspace = 'personal', validAccount = true;
  const page = { location: { origin: 'https://chatgpt.com' }, document: {}, __elonChatGptDocumentToken: 'doc_fixture_123',
    crypto: webcrypto,
    __elonChatGptGroupProjectIdentity: identityContract,
    __elonChatGptPrivateDirectoryPages: directoryPages,
    __elonChatGptPrivateConversationShareContract: shareContract,
    __elonChatGptPrivateRuntimeBindings: { observed: () => false, load: () => { throw Error('must not import runtime'); } },
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: 'Bearer synthetic-test-token', 'chatgpt-account-id': JSON.parse(account)[1] }) },
    __elonChatGptPrivateJsonRequest: { request: async (_, path, init, limits) => {
      const [userId, accountId] = JSON.parse(account);
      if (path === '/api/auth/session') {
        identityCalls.push(path);
        return { payload: { user: { id: validAccount ? userId : '' }, account: { id: accountId, structure: workspace }, accessToken: 'synthetic-test-token' } };
      }
      calls.push({ path, init, limits });
      const next = replies.shift();
      if (typeof next === 'function') return next();
      if (next instanceof Error) throw next;
      return { payload: next };
    } },
  };
  return { page, calls, identityCalls, replies, core: transport.create(page, { policy }),
    switchAccount: () => { account = '["other","other_workspace"]'; },
    workspace: value => { workspace = value; }, invalidateAccount: () => { validAccount = false; } };
}
async function input(f, operation, more = {}) {
  const identity = await f.core.run({ operation: 'identity' });
  assert.equal(identity.ok, true);
  assert.match(identity.accountScope, /^[a-f0-9]{64}$/);
  assert.equal(JSON.stringify(identity).includes('user_fixture'), false);
  return { ...base, accountScope: identity.accountScope, operation, ...more };
}
test('create uses reviewed projects contract, private memory, then verifies resource', async () => {
  const f = fixture(), command = await input(f, 'create');
  f.replies.push({ resource: resource() }, resource());
  const result = await f.core.run(command);
  assert.equal(result.ok, true);
  assert.equal(result.projectId, projectId);
  assert.equal(f.calls[0].path, '/backend-api/projects');
  assert.equal(JSON.parse(f.calls[0].init.body).memory_scope, 'project_v2');
  assert.equal(f.calls[0].init.headers['ChatGPT-Account-ID'], 'workspace_fixture');
  assert.equal(f.calls[0].init.headers['chatgpt-account-id'], undefined);
  assert.equal(f.calls[1].init.method, 'GET');
});

test('identity works without a recognized runtime and fails closed on document, account and workspace', async () => {
  assert.equal((await fixture().core.run({ operation: 'identity' })).ok, true);
  for (const reason of ['document', 'account', 'workspace']) {
    const f = fixture();
    if (reason === 'document') f.page.__elonChatGptDocumentToken = '';
    if (reason === 'account') f.invalidateAccount();
    if (reason === 'workspace') f.workspace('workspace');
    assert.deepEqual(await f.core.run({ operation: 'identity' }), reason === 'account' ? {
      ok: false, code: 'project_auth_required',
    } : {
      ok: false, code: 'project_identity_unavailable', identityReason: reason,
    });
    assert.equal(f.calls.length, 0);
  }
});

test('cookie identity is rechecked per operation without loading runtime or account directory', async () => {
  const f = fixture();
  const first = await f.core.run({ operation: 'identity' });
  assert.equal(first.ok, true);
  await f.core.run({ operation: 'identity' });
  assert.equal(f.identityCalls.length, 2);
  assert.deepEqual(await f.core.run({ operation: 'identity' }), first);
  assert.equal(f.identityCalls.length, 3);
  f.page.document = {};
  await f.core.run({ operation: 'identity' });
  assert.equal(f.identityCalls.length, 4);
  f.switchAccount();
  assert.notEqual((await f.core.run({ operation: 'identity' })).accountScope, first.accountScope);
  assert.equal(f.identityCalls.length, 5);
});

test('logout is detected even if previously observed request headers are still cached', async () => {
  const f = fixture(), command = await input(f, 'create');
  f.page.__elonChatGptPrivateJsonRequest.request = async () => ({ payload: {} });
  assert.equal((await f.core.run(command)).code, 'project_auth_required');
  assert.equal(f.calls.length, 0);
});

test('identity endpoint failures cannot masquerade as a deleted project', async () => {
  for (const [raw, code] of [['http_404', 'project_unavailable'], ['http_401', 'project_auth_required'],
    ['http_429', 'project_rate_limited'], ['timeout', 'project_unavailable']]) {
    const f = fixture();
    f.page.__elonChatGptPrivateJsonRequest.request = async () => { throw Error(raw); };
    assert.deepEqual(await f.core.run({ operation: 'identity' }), { ok: false, code,
      ...(code === 'project_unavailable' ? { identityReason: raw === 'timeout' ? 'timeout' : 'http' } : {}) });
    assert.equal(f.calls.length, 0);
  }
});

test('identity GET discarded after account switch; mismatched workspace header never authorizes a write', async () => {
  const f = fixture(), original = f.page.__elonChatGptPrivateJsonRequest.request;
  f.page.__elonChatGptPrivateJsonRequest.request = async (...args) => {
    const result = await original(...args); f.switchAccount(); return result;
  };
  assert.equal((await f.core.run({ operation: 'identity' })).code, 'project_identity_changed');
  const mismatch = fixture();
  mismatch.page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders = () => ({
    Authorization: 'Bearer synthetic-test-token', 'ChatGPT-Account-ID': 'other_workspace',
  });
  assert.equal((await mismatch.core.run({ operation: 'identity' })).code, 'project_identity_changed');
  assert.equal(mismatch.identityCalls.length, 1);
});
test('rename does not affect marker binding; wrong marker or global memory fails closed', () => {
  const data = resource(); data.gizmo.display = { name: 'Renamed' };
  assert.equal(policy.resource(data, projectId, policy.marker(bindingId, 1)).projectId, projectId);
  assert.throws(() => policy.resource(data, projectId, policy.marker(bindingId, 2)));
  data.gizmo.memory_scope = 'global';
  assert.throws(() => policy.resource(data, projectId, policy.marker(bindingId, 1)));
});
test('read differentiates confirmed missing from network, authentication and rate limit', async () => {
  for (const [error, expected] of [['http_404','project_not_found'], ['http_401','project_auth_required'],
    ['http_403','project_auth_required'], ['http_429','project_rate_limited'], ['timeout','project_request_timeout'],
    ['http_422','project_request_rejected'], ['invalid_json','project_response_invalid_json'],
    ['response_too_large','project_response_too_large'], ['http_503','project_service_unavailable']]) {
    const f = fixture(), command = await input(f, 'read', { projectId });
    f.replies.push(Error(error));
    assert.equal((await f.core.run(command)).code, expected);
    assert.equal(f.calls.length, 1);
  }
});
test('unknown create is never retried by transport', async () => {
  const f = fixture(), command = await input(f, 'create'); f.replies.push(Error('timeout'));
  const result = await f.core.run(command);
  assert.equal(result.code, 'project_create_unknown');
  assert.equal(result.notSent, undefined);
  assert.equal(f.calls.length, 1);
});

test('only a proven pre-dispatch create failure reports notSent', async () => {
  const f = fixture(), command = await input(f, 'create');
  f.page.__elonChatGptPrivateJsonRequest.request = async () => { throw Error('timeout'); };
  const result = await f.core.run(command);
  assert.equal(result.notSent, true);
  assert.equal(result.identityReason, 'timeout');
  assert.equal(f.calls.length, 0);
});
test('an already bound project remains usable after its owner edits instructions', async () => {
  const f = fixture(), command = await input(f, 'read', { projectId });
  const changed = resource(); changed.gizmo.instructions = 'Owner updated instructions';
  f.replies.push(changed);
  assert.equal((await f.core.run(command)).projectId, projectId);
});
test('late results after identity or document change never bind a project', async () => {
  for (const change of ['account', 'document']) {
    const f = fixture(), command = await input(f, 'read', { projectId });
    f.replies.push(() => { if (change === 'account') f.switchAccount(); else f.page.document = {}; return { payload: resource() }; });
    assert.equal((await f.core.run(command)).code, 'project_identity_changed');
  }
});
test('reconcile requires unique existing marker and performs no writes', async () => {
  const f = fixture(), command = await input(f, 'reconcile');
  f.replies.push({ items: [{ gizmo: resource() }], cursor: null }, resource());
  assert.equal((await f.core.run(command)).projectId, projectId);
  assert.ok(f.calls.every(c => c.init.method === 'GET'));
  assert.equal(f.calls[0].path, directoryPages.path(directoryPages.initial('projects')));
  const empty = fixture(), retry = await input(empty, 'reconcile');
  empty.replies.push({ items: [], cursor: null });
  assert.equal((await empty.core.run(retry)).code, 'project_create_unresolved');
});

test('missing page dependencies fail explicitly before any provider write', async () => {
  const f = fixture(), command = await input(f, 'create');
  const missingPolicy = transport.create(f.page);
  assert.deepEqual(await missingPolicy.run(command), { ok: false, code: 'project_adapter_unavailable', notSent: true });
  assert.equal(f.calls.length, 0);
  delete f.page.__elonChatGptPrivateDirectoryPages;
  const result = await f.core.run({ ...command, operation: 'reconcile' });
  assert.equal(result.code, 'project_adapter_unavailable');
  assert.equal(f.calls.length, 0);
});
test('existing conversation must explicitly belong to project and retain history', async () => {
  const f = fixture(), command = await input(f, 'read', { projectId, conversationId });
  f.replies.push(resource(), { conversation_id: conversationId, gizmo_id: projectId, is_do_not_remember: false });
  assert.equal((await f.core.run(command)).conversationId, conversationId);
  assert.throws(() => policy.conversation({ conversation_id: conversationId, gizmo_id: 'other' }, conversationId, projectId));
});
test('parallel requests are single flight; account mismatch never reaches network', async () => {
  const f = fixture(), command = await input(f, 'read', { projectId });
  let release;
  f.replies.push(() => new Promise(resolve => { release = () => resolve({ payload: resource() }); }));
  const pending = f.core.run(command);
  assert.equal((await f.core.run(command)).code, 'project_busy');
  while (!release) await new Promise(r => setImmediate(r));
  release(); assert.equal((await pending).ok, true);
  f.switchAccount();
  assert.equal((await f.core.run(command)).code, 'project_identity_changed');
  assert.equal(f.calls.length, 1);
});

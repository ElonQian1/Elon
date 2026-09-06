'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const deletion = require('../android/app/src/main/assets/chatgpt_web_private_conversation_delete.js');
const request = require('../android/app/src/main/assets/chatgpt_web_private_json_request.js');
const asset = name => fs.readFileSync(path.join(__dirname, '../android/app/src/main/assets/', name), 'utf8');
const deferred = () => { let resolve; const promise = new Promise(r => { resolve = r; }); return { promise, resolve }; };
const RUNTIME_URL = 'https://chatgpt.com/cdn/assets/4813494d-hrplraurzfyvxb10.js';

function fixture(fetcher = () => new Response('{}')) {
  const calls = [], accepted = [];
  const gate = { name: '4177111012', value: false, details: { reason: 'Bootstrap:Recognized' } };
  const client = { loadingStatus: 'Ready', getFeatureGate: (name, options) => {
    assert.equal(name, gate.name);
    assert.deepEqual(options, { disableExposureLog: true });
    return gate;
  } };
  const runtime = { loadRuntime: async url => { assert.equal(url, RUNTIME_URL); return { t6: () => client }; } };
  let headers = { Authorization: 'Bearer local-fixture-token', Cookie: 'not-exported',
    'openai-sentinel-proof-token': 'not-replayed', 'chatgpt-account-id': 'fixture-account' };
  const root = {
    location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/active-chat', pathname: '/c/active-chat' },
    __elonChatGptDocumentToken: 'doc_test_123',
    __elonChatGptPrivateConversationMutationsEnabled: true,
    __elonChatGptPrivateJsonRequest: request,
    __elonChatGptPrivateConversationDirectory: {
      snapshot: () => ({ conversations: [{ id: 'target-chat' }] }),
      acceptDeletedState: id => accepted.push(id),
    },
    __elonChatGptPrivateTransport: {
      copySameOriginRequestHeaders: () => headers,
      acquireSameOriginRequestHeaders: async () => headers,
    },
    AbortController, setTimeout, clearTimeout,
    performance: { getEntriesByName: url => url === RUNTIME_URL ? [{}] : [] },
    fetch: async (url, init) => { calls.push({ url, init }); return fetcher(url, init); },
  };
  return { root, calls, accepted, gate, client, runtime, api: deletion.create(root, runtime),
    changeAccount: () => { headers = { Authorization: 'Bearer different-account' }; } };
}

test('one confirmed delete writes only is_visible and never forwards cookie or captured proof', async () => {
  const f = fixture();
  assert.equal((await f.api.start('/c/target-chat', true)).ok, true);
  assert.equal(f.calls.length, 1);
  const { url, init } = f.calls[0];
  assert.equal(url, '/backend-api/conversation/target-chat');
  assert.equal(init.method, 'PATCH');
  assert.deepEqual(JSON.parse(init.body), { is_visible: false });
  assert.equal(init.headers.Cookie, undefined);
  assert.equal(init.headers['openai-sentinel-proof-token'], undefined);
  assert.equal(init.credentials, 'include');
  assert.equal(init.redirect, 'error');
  assert.deepEqual(f.accepted, ['target-chat']);
});

test('current official flag selects DELETE without a legacy request body', async () => {
  const f = fixture(() => new Response(null, { status: 204 }));
  f.gate.value = true;
  assert.equal((await f.api.start('/c/target-chat', true)).ok, true);
  assert.equal(f.calls.length, 1);
  assert.equal(f.calls[0].url, '/backend-api/conversation/id/target-chat');
  assert.equal(f.calls[0].init.method, 'DELETE');
  assert.equal(f.calls[0].init.body, undefined);
});

test('unknown or unloaded official configuration never defaults to legacy deletion', async () => {
  for (const reason of ['Uninitialized', 'NoValues', 'Network:Unrecognized', 'LocalOverride']) {
    const f = fixture(); f.gate.details.reason = reason;
    assert.equal((await f.api.start('/c/target-chat', true)).code, 'delete_configuration_unavailable');
    assert.equal(f.calls.length, 0);
    assert.equal(f.api.busy(), false);
  }
  for (const change of [f => { f.client.loadingStatus = 'Loading'; },
    f => { f.gate.value = 'false'; }, f => { f.gate.details.warnings = ['NoCachedValues']; },
    f => { f.root.performance.getEntriesByName = () => []; }]) {
    const f = fixture(); change(f);
    assert.equal((await f.api.start('/c/target-chat', true)).code, 'delete_configuration_unavailable');
    assert.equal(f.calls.length, 0);
  }
});

test('only a loaded exact module may be reused, and only its module namespace is cached', async () => {
  const f = fixture(); let loads = 0;
  f.runtime.loadRuntime = async () => { loads += 1; return { t6: () => f.client }; };
  f.root.performance.getEntriesByName = () => [];
  assert.equal((await f.api.start('/c/target-chat', true)).ok, false);
  assert.equal(loads, 0);
  f.root.performance.getEntriesByName = () => [{}];
  await f.api.start('/c/target-chat', true);
  f.gate.value = true;
  await f.api.start('/c/target-chat', true);
  assert.equal(loads, 1);
  assert.deepEqual(f.calls.map(call => call.init.method), ['PATCH', 'DELETE']);
});

test('configuration wait is bounded, and context changes during it cannot write', async () => {
  for (const change of [f => f.changeAccount(), f => { f.root.__elonChatGptDocumentToken = 'doc_changed_123'; },
    f => { f.root.location.href = 'https://chatgpt.com/c/changed'; }]) {
    const f = fixture(), pendingRuntime = deferred();
    f.runtime.loadRuntime = () => pendingRuntime.promise;
    const pending = f.api.start('/c/target-chat', true);
    await new Promise(resolve => setImmediate(resolve));
    change(f); pendingRuntime.resolve({ t6: () => f.client });
    assert.equal((await pending).ok, false);
    assert.equal(f.calls.length, 0);
  }
  const f = fixture(), pendingRuntime = deferred();
  f.runtime.loadRuntime = () => pendingRuntime.promise;
  f.root.setTimeout = (callback, ms) => setTimeout(callback, ms === 2000 ? 1 : ms);
  assert.equal((await f.api.start('/c/target-chat', true)).code, 'delete_configuration_unavailable');
  pendingRuntime.resolve({ t6: () => f.client });
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(f.calls.length, 0);
  assert.equal(f.api.busy(), false);
});

test('DELETE rejection or uncertainty never falls through to PATCH', async () => {
  for (const fail of ['http', 'network']) {
    const f = fixture((url, init) => {
      if (init.method === 'DELETE') {
        if (fail === 'network') throw new Error('network');
        return new Response('{}', { status: 404 });
      }
      return new Response('{}', { status: 404 });
    });
    f.gate.value = true;
    const result = await f.api.start('/c/target-chat', true);
    assert.equal(result.ok, false);
    assert.deepEqual(f.calls.map(call => call.init.method), fail === 'http' ? ['DELETE'] : ['DELETE', 'GET']);
    assert.equal(f.accepted.length, 0);
  }
});

test('configuration recovery preserves new drafts and never runs an abandoned deletion', async () => {
  const f = fixture(), pendingRuntime = deferred(), snapshot = currentSnapshot(f);
  f.runtime.loadRuntime = () => pendingRuntime.promise;
  const pending = f.api.start('/c/target-chat', true, () => snapshot);
  await new Promise(resolve => setImmediate(resolve));
  snapshot.draft = 'new unsent text';
  pendingRuntime.resolve({ t6: () => f.client });
  assert.equal((await pending).code, 'delete_draft_present');
  assert.equal(f.calls.length, 0);
  const recover = fixture();
  recover.runtime.loadRuntime = async () => { throw new Error('module unavailable'); };
  assert.equal((await recover.api.start('/c/target-chat', true)).code, 'delete_configuration_unavailable');
  recover.runtime.loadRuntime = async () => ({ t6: () => recover.client });
  recover.gate.value = true;
  assert.equal((await recover.api.start('/c/target-chat', true)).ok, true);
  assert.deepEqual(recover.calls.map(call => call.init.method), ['DELETE']);
});

test('requires confirmation and exact cached target', async () => {
  const f = fixture();
  for (const [target, confirmed, code] of [
    ['/c/target-chat', false, 'user_confirmation_required'],
    ['/c/target-chat?all=true', true, 'invalid_conversation_path'],
    ['/c/missing-chat', true, 'delete_selection_expired'],
    ['/g/g-p-fixture/c/active-chat', true, 'delete_selection_expired'],
  ]) assert.equal((await f.api.start(target, confirmed)).code, code);
  assert.equal(f.calls.length, 0);
});

function currentSnapshot(f) {
  f.root.location.pathname = '/g/g-p-fixture/c/target-chat';
  f.root.location.href = f.root.location.origin + f.root.location.pathname;
  return { url: f.root.location.href, composerReady: true, streaming: false, draft: '',
    attachments: [], dictationActive: false, dictationCaptureActive: false, dictationCapturePending: false };
}

test('current conversation deletion requires a ready idle snapshot without draft or attachments', async () => {
  const missing = fixture(); currentSnapshot(missing);
  assert.equal((await missing.api.start('/c/target-chat', true)).code, 'delete_context_unavailable');
  for (const change of [{ draft: 'unsent' }, { attachments: [{}] }, { streaming: true },
    { composerReady: false }, { dictationActive: true }, { dictationCaptureActive: true }, { dictationCapturePending: true }]) {
    const f = fixture(), snapshot = { ...currentSnapshot(f), ...change };
    assert.equal((await f.api.start('/c/target-chat', true, () => snapshot)).ok, false);
    assert.equal(f.calls.length, 0);
  }
  const ready = fixture(), snapshot = currentSnapshot(ready);
  assert.equal((await ready.api.start('/c/target-chat', true, () => snapshot)).ok, true);
  assert.equal(ready.calls.length, 1);
});

test('draft appearing during authentication is preserved and prevents the write', async () => {
  const gate = deferred(), f = fixture(), snapshot = currentSnapshot(f);
  f.root.__elonChatGptPrivateTransport.acquireSameOriginRequestHeaders = () => gate.promise;
  const pending = f.api.start('/c/target-chat', true, () => snapshot);
  snapshot.draft = 'new unsent draft';
  gate.resolve(f.root.__elonChatGptPrivateTransport.copySameOriginRequestHeaders());
  assert.equal((await pending).code, 'delete_draft_present');
  assert.equal(f.calls.length, 0);
});

test('current deletion sends terminal receipt before any directory-triggered native navigation', async () => {
  const f = fixture(), snapshot = currentSnapshot(f), events = [];
  f.root.__elonChatGptPrivateConversationDirectory.acceptDeletedState = (_, notify) => {
    if (notify !== false) events.push('early-listener');
  };
  f.api.handle('delete_conversation', { value: '/c/target-chat', selected: true },
    (_, ok) => events.push(ok ? 'confirmed' : 'failed'), () => events.push('snapshot'),
    { emitSnapshot: () => events.push('directory') }, () => snapshot);
  await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(events, ['confirmed', 'directory', 'snapshot']);
});

test('duplicate clicks and other mutations cannot overlap the delete owner', async () => {
  const gate = deferred(), f = fixture(() => gate.promise);
  const pending = f.api.start('/c/target-chat', true);
  assert.equal((await f.api.start('/c/target-chat', true)).code, 'delete_busy');
  gate.resolve(new Response('{}')); await pending;
  const other = fixture();
  other.root.__elonChatGptPrivateConversationMutation = { state: () => ({ state: 'busy' }) };
  assert.equal((await other.api.start('/c/target-chat', true)).code, 'delete_busy');
  assert.equal(other.calls.length, 0);
});

test('account or document change during identity acquisition cannot send a delete', async () => {
  for (const kind of ['account', 'document', 'route']) {
    const gate = deferred(), f = fixture();
    const old = f.root.__elonChatGptPrivateTransport.copySameOriginRequestHeaders();
    f.root.__elonChatGptPrivateTransport.acquireSameOriginRequestHeaders = () => gate.promise;
    const pending = f.api.start('/c/target-chat', true);
    if (kind === 'account') f.changeAccount();
    if (kind === 'document') f.root.__elonChatGptDocumentToken = 'doc_replaced_123';
    if (kind === 'route') f.root.location.href = 'https://chatgpt.com/c/another-chat';
    gate.resolve(old);
    assert.equal((await pending).ok, false);
    assert.equal(f.calls.length, 0);
  }
});

test('late success after account change never mutates the new account directory', async () => {
  const gate = deferred(), f = fixture(() => gate.promise);
  const pending = f.api.start('/c/target-chat', true);
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(f.calls.length, 1);
  f.changeAccount(); gate.resolve(new Response('{}'));
  assert.equal((await pending).code, 'delete_result_unconfirmed');
  assert.equal(f.accepted.length, 0);
});

test('uncertain write only reconciles exact is_visible false metadata, never replays', async () => {
  for (const payload of [{ id: 'target-chat', is_visible: false }, { id: 'other-chat', is_visible: false }, {}]) {
    const f = fixture((url, init) => { if (init.method === 'PATCH') throw new Error('network'); return new Response(JSON.stringify(payload)); });
    const result = await f.api.start('/c/target-chat', true);
    assert.equal(result.ok, payload.id === 'target-chat');
    assert.deepEqual(f.calls.map(c => c.init.method), ['PATCH', 'GET']);
    assert.equal(f.accepted.length, result.ok ? 1 : 0);
    if (!result.ok) assert.equal((await f.api.start('/c/target-chat', true)).code, 'delete_cooldown');
  }
});

test('404 reconciliation and explicit HTTP rejection do not become false success', async () => {
  const f = fixture((url, init) => { if (init.method === 'PATCH') throw new Error('network'); return new Response('{}', { status: 404 }); });
  assert.equal((await f.api.start('/c/target-chat', true)).ok, false);
  assert.equal(f.accepted.length, 0);
  const forbidden = fixture(() => new Response('{}', { status: 403 }));
  assert.equal((await forbidden.api.start('/c/target-chat', true)).code, 'delete_http_403');
  assert.equal(forbidden.calls.length, 1);
});

test('hanging auth is bounded and its late arrival never dispatches a write', async () => {
  const gate = deferred(), f = fixture();
  f.root.setTimeout = callback => setTimeout(callback, 1);
  f.root.__elonChatGptPrivateTransport.acquireSameOriginRequestHeaders = () => gate.promise;
  assert.equal((await f.api.start('/c/target-chat', true)).code, 'delete_auth_unavailable');
  gate.resolve(f.root.__elonChatGptPrivateTransport.copySameOriginRequestHeaders());
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(f.calls.length, 0);
  assert.equal(f.api.busy(), false);
});

test('deleting last row emits an empty directory with deletion markers and ignores stale directory responses', async () => {
  const text = JSON.stringify({ items: [{ id: 'target-chat', title: 'Synthetic fixture' }] });
  const location = { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/', pathname: '/' };
  const window = { location, fetch: async () => ({ status: 200, clone: () => ({ text: async () => text }) }) };
  const context = vm.createContext({ window, location, URL, setTimeout, clearTimeout });
  vm.runInContext(asset('chatgpt_web_private_conversation_directory.js'), context);
  vm.runInContext(asset('chatgpt_web_adapter_conversation_directory_requests.js'), context);
  const directory = window.__elonChatGptPrivateConversationDirectory;
  let notifications = 0;
  directory.setListener(() => { notifications += 1; });
  await window.fetch('/backend-api/conversations');
  await new Promise(resolve => setImmediate(resolve));
  const emitted = [];
  const adapter = window.__elonChatGptConversationDirectoryRequests.create({ privateDirectory: directory,
    optional: (fallback, fn) => fn(), emitEvent: event => emitted.push(event) });
  adapter.emitSnapshot(null);
  const beforeDelete = notifications;
  directory.acceptDeletedState('target-chat', false);
  assert.equal(notifications, beforeDelete);
  await window.fetch('/backend-api/conversations');
  await new Promise(resolve => setImmediate(resolve));
  directory.acceptArchivedState('target-chat', false, { title: 'Stale restore' });
  adapter.emitSnapshot(null);
  assert.equal(emitted.at(-1).conversations.length, 0);
  assert.deepEqual(Array.from(emitted.at(-1).deletedConversationIds), ['target-chat']);
  assert.deepEqual(Array.from(emitted.at(-1).removedConversationIds), ['target-chat']);
});

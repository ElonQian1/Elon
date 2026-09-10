'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const bindings = require('../android/app/src/main/assets/chatgpt_web_private_runtime_bindings');
const { attach, CDN, files, expectedExports } = require('./fixtures/chatgpt-runtime-bindings');
const old = {
  shared: CDN + '4813494d-hrplraurzfyvxb10.js',
  conversation: CDN + 'conversation-small-hiw4wce20lu6te81.js',
  composer: CDN + '8b34dbc2-kjj15hg4y6iyx13p.js'
};
function fixture(options = {}) {
  const observed = new Set([CDN + files.shared]), calls = [];
  const page = { location: { origin: 'https://chatgpt.com' }, document: { querySelector: () => null },
    __elonChatGptDocumentToken: 'doc_test', setTimeout, clearTimeout,
    performance: { getEntriesByName: url => observed.has(url) ? [{}] : [] } };
  const api = bindings.create(page, { ...options, loadRuntime: async url => {
    calls.push(url); return options.loadRuntime ? options.loadRuntime(url) : { c6: () => true };
  } });
  return { page, observed, calls, api };
}

test('current aliases resolve the exact observed build, never the unrelated legacy export', async () => {
  const good = () => true, bad = () => { throw Error('wrong export'); };
  const f = fixture({ loadRuntime: () => ({ c6: good, H3: bad }) });
  assert.equal(f.api.observed(old.shared), true);
  const value = await f.api.load(old.shared);
  assert.equal(value.H3, good); assert.equal(value.H3(), true);
  assert.equal(value.c6, undefined);
  assert.deepEqual(f.calls, [CDN + files.shared]);
  assert.equal(Object.getPrototypeOf(value), null); assert.equal(Object.isFrozen(value), true);
});

test('every consumer export preserves its original function or store identity', async () => {
  const modules = Object.fromEntries(Object.entries(expectedExports).map(([role, keys]) => [role,
    Object.fromEntries(Object.keys(keys).map(key => [key, () => key]))]));
  const f = fixture(), current = attach(f.page, modules);
  for (const role of Object.keys(modules)) {
    const value = await current.api.load(old[role]);
    for (const key of Object.keys(modules[role])) assert.equal(value[key], modules[role][key]);
  }
  assert.equal(current.loads.length, 3);
});

test('tool aliases never call the new focus helper or initializer under old names', async () => {
  const read = () => ({ activeSystemHintType: 'search' }), write = () => true;
  const unrelated = () => { throw Error('wrong runtime alias'); };
  const f = fixture({ loadRuntime: () => ({ Yg: read, n_: write, Ng: unrelated, Bg: unrelated }) });
  f.observed.add(CDN + files.composer);
  const module = await f.api.load('composer');
  assert.equal(module.Ng, read); assert.equal(module.Bg, write);
  assert.equal(f.api.tools().owner, 'Kgn');
  f.page.__elonChatGptDocumentToken = 'doc_old'; f.observed.clear(); f.observed.add(old.composer);
  assert.equal(f.api.tools().owner, 'Whn');
});

test('old cached website still uses its original singleton and aliases', async () => {
  const getter = () => true, f = fixture({ loadRuntime: () => ({ H3: getter }) });
  f.observed.clear(); f.observed.add(old.shared);
  assert.equal((await f.api.load(old.shared)).H3, getter);
  assert.deepEqual(f.calls, [old.shared]); assert.equal(f.api.state().profile_id, 'web_20260906');
});

for (const legacy of [false, true]) {
  test('official registered action bindings preserve ' + (legacy ? 'legacy' : 'current') + ' singleton identity', async () => {
    const inspect = key => ({ id: key, isAvailable: true, disabled: false });
    const calls = [], invoke = key => { calls.push(key); return true; };
    const f = fixture({ loadRuntime: () => ({ Ur: inspect, zr: invoke }) });
    if (legacy) { f.observed.clear(); f.observed.add(old.shared); }
    const shared = await f.api.load('shared');
    assert.equal(shared.Ur, inspect);
    assert.equal(shared.zr, invoke);
    assert.equal(shared.Ur('newChat').id, 'newChat');
    assert.equal(shared.zr('newChat'), true);
    assert.deepEqual(calls, ['newChat']);
  });
}

test('on-page static dependency anchor works when resource timing has been truncated', async () => {
  const f = fixture(); f.observed.clear(); f.observed.add(CDN + 'c2675c8c-kconnwitb9zzv81k.js');
  assert.equal(f.api.observed(old.composer), true);
  assert.equal((await f.api.load(old.shared)).H3(), true);
});

test('unknown websites and arbitrary module URLs are never imported', async () => {
  const f = fixture(); f.observed.clear();
  f.observed.add(CDN + '4813494d-unknown.js');
  assert.equal(f.api.observed(old.shared), false);
  await assert.rejects(f.api.load(old.shared), /not_observed/);
  await assert.rejects(f.api.load('https://example.invalid/module.js'), /not_observed/);
  assert.equal(f.calls.length, 0);
});

test('mixed website builds cannot combine auth, conversation and composer stores', async () => {
  const f = fixture(); f.observed.add(old.composer);
  assert.equal(f.api.observed(old.shared), false);
  await assert.rejects(f.api.load(old.shared), /not_observed/);
  assert.equal(f.api.state().error, 'runtime_build_ambiguous'); assert.equal(f.calls.length, 0);
});

test('concurrent consumers and warm revisits share one bounded import', async () => {
  let complete; const f = fixture({ loadRuntime: () => new Promise(resolve => { complete = resolve; }) });
  const a = f.api.load(old.shared), b = f.api.load('shared');
  assert.equal(a, b); await Promise.resolve(); complete({ c6: () => true });
  const value = await a;
  f.observed.clear();
  assert.equal(await f.api.load('shared'), value); assert.equal(f.calls.length, 1);
});

test('synchronous cache reads never import or wait for an unresolved module', async () => {
  let complete; const f = fixture({ loadRuntime: () => new Promise(resolve => { complete = resolve; }) });
  assert.equal(f.api.peek('shared'), null); assert.equal(f.calls.length, 0);
  const request = f.api.load('shared'); await Promise.resolve();
  assert.equal(f.api.peek('shared'), null);
  complete({ c6: () => true }); const value = await request;
  assert.equal(f.api.peek(old.shared), value); assert.equal(f.calls.length, 1);
  assert.equal(f.api.peek('https://example.invalid/module.js'), null);
});

for (const kind of ['document', 'token', 'mixed_build', 'foreign_origin']) {
  test('cached synchronous namespace is unavailable after ' + kind, async () => {
    const f = fixture(); await f.api.load('shared');
    if (kind === 'document') f.page.document = { querySelector: () => null };
    if (kind === 'token') f.page.__elonChatGptDocumentToken = 'doc_replaced';
    if (kind === 'mixed_build') f.observed.add(old.composer);
    if (kind === 'foreign_origin') f.page.location.origin = 'https://example.invalid';
    assert.equal(f.api.peek('shared'), null); assert.equal(f.calls.length, 1);
  });
}

test('failed or late old-context imports cannot become a synchronous guest identity', async () => {
  const pending = [], f = fixture({ loadRuntime: () => new Promise(resolve => pending.push(resolve)) });
  const oldRequest = f.api.load('shared'); await Promise.resolve();
  f.page.__elonChatGptDocumentToken = 'doc_new';
  const newRequest = f.api.load('shared'); await Promise.resolve();
  pending[0]({ i7: () => ({ authStatus: 'logged_out' }), t7: () => null });
  await assert.rejects(oldRequest, /context_changed/); assert.equal(f.api.peek('shared'), null);
  pending[1]({ i7: () => ({ authStatus: 'logged_in' }), t7: () => ({}) });
  const current = await newRequest;
  assert.equal(f.api.peek('shared'), current);
  assert.equal(current.R5().authStatus, 'logged_in');
});

test('late completion from a replaced document cannot populate the new cache', async () => {
  const pending = [];
  const f = fixture({ loadRuntime: () => new Promise(resolve => pending.push(resolve)) });
  const oldRequest = f.api.load('shared'); await Promise.resolve();
  f.page.__elonChatGptDocumentToken = 'doc_other';
  const newRequest = f.api.load('shared'); await Promise.resolve();
  pending[0]({ c6: () => false }); await assert.rejects(oldRequest, /context_changed/);
  pending[1]({ c6: () => true });
  assert.equal((await newRequest).H3(), true); assert.equal(f.api.state().cached_modules, 1);
});

test('module failure cools down instead of repeatedly importing on every click', async () => {
  let time = 0;
  const f = fixture({ now: () => time, loadRuntime: () => { throw Error('network failure'); } });
  await assert.rejects(f.api.load('shared'), /network failure/);
  await assert.rejects(f.api.load('shared'), /cooldown/); assert.equal(f.calls.length, 1);
  time = 10001; await assert.rejects(f.api.load('shared')); assert.equal(f.calls.length, 2);
  assert.equal(f.api.state().error, 'runtime_load_failed');
});

test('a never-settled module times out and a late result does not become usable', async () => {
  let finish; const f = fixture({ timeoutMs: 100, loadRuntime: () => new Promise(resolve => { finish = resolve; }) });
  await assert.rejects(f.api.load('shared'), /runtime_timeout/);
  finish({ c6: () => true }); await Promise.resolve();
  await assert.rejects(f.api.load('shared'), /cooldown/); assert.equal(f.api.state().cached_modules, 0);
});

test('missing exports stay unknown instead of calling the reused old alias', async () => {
  const f = fixture({ loadRuntime: () => ({ H3: () => true }) });
  await assert.rejects(f.api.load('shared'), /exports_unknown/);
});

test('foreign origin invalidates access even with previously cached modules', async () => {
  const f = fixture(); await f.api.load('shared'); f.page.location.origin = 'https://example.invalid';
  assert.equal(f.api.observed(old.shared), false); await assert.rejects(f.api.load('shared'), /not_observed/);
});

test('temporary-chat callback is selected by the same build, not only its minified name', () => {
  const f = fixture();
  assert.equal(f.api.temporary().owner, 'vqt'); assert.match(f.api.temporary().action, /SB\.reset\(c\)/);
  f.page.__elonChatGptDocumentToken = 'doc_older'; f.observed.clear(); f.observed.add(old.shared);
  assert.equal(f.api.temporary().owner, 'AKt');
});

test('production resolver is assembled before every runtime consumer', () => {
  const source = require('./chatgpt-web-adapter-assembly').readAdapterSource();
  const names = [...source.split('private val ADAPTER_ASSETS = listOf(')[1].split(')')[0]
    .matchAll(/"([a-z0-9_]+\.js)"/g)].map(m => m[1]);
  const binding = names.indexOf('chatgpt_web_private_runtime_bindings.js');
  assert.ok(binding >= 0); assert.equal(names.filter(n => n === names[binding]).length, 1);
  for (const name of ['model_state', 'temporary_chat', 'conversation_share_contract', 'conversation_delete',
    'attachment_reservation', 'attachment_library', 'attachment_project', 'regenerate_runtime', 'stop_runtime', 'text_runtime_submit']) {
    assert.ok(names.indexOf('chatgpt_web_private_' + name + '.js') > binding, name);
  }
});

test('September 10 local append quota exports preserve exact official helper identity', async () => {
  const { files: current } = require('./fixtures/chatgpt-runtime-bindings-sep10');
  const types = { Multimodal: 1, Interpreter: 2 }, base = () => 10, max = () => 8;
  const count = () => 2, config = () => 5;
  const namespaces = { [CDN + current.shared]: { Up: types },
    [CDN + current.conversation]: { eQt: base, sQt: max, nQt: count, $Zt: config,
      attachmentBaseLimit: () => { throw Error('unmapped alias'); } } };
  const f = fixture({ loadRuntime: url => namespaces[url] });
  f.observed.clear(); f.observed.add(CDN + current.shared); f.observed.add(CDN + current.conversation);
  assert.equal((await f.api.load('shared')).attachmentUploadType, types);
  const c = await f.api.load('conversation');
  assert.equal(c.attachmentBaseLimit, base); assert.equal(c.attachmentMaxUploads, max);
  assert.equal(c.attachmentPendingCount, count); assert.equal(c.attachmentConfiguredLimit, config);
  assert.equal(c.eQt, undefined);
});

test('unverified older build cannot reuse September 10 local append quota aliases', async () => {
  const f = fixture({ loadRuntime: () => ({ c6: () => true, H3: () => true,
    Up: { Multimodal: 1, Interpreter: 2 }, attachmentUploadType: { Multimodal: 1, Interpreter: 2 } }) });
  const s = await f.api.load('shared'); assert.equal(s.attachmentUploadType, undefined);
  f.observed.clear(); f.observed.add(old.shared); f.page.__elonChatGptDocumentToken = 'doc_legacy';
  assert.equal((await f.api.load('shared')).attachmentUploadType, undefined);
});

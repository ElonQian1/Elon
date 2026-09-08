'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const bindings = require('../android/app/src/main/assets/chatgpt_web_private_runtime_bindings');
const { attach, CDN, expectedExports: prior } = require('./fixtures/chatgpt-runtime-bindings');
const profiles = [require('./fixtures/chatgpt-runtime-bindings-sep9'), require('./fixtures/chatgpt-runtime-bindings-sep9b')];

for (const profile of profiles) test.describe(profile.id, () => {

function fixture(modules = {}) {
  const page = { location: { origin: 'https://chatgpt.com' }, document: { querySelector: () => null },
    __elonChatGptDocumentToken: 'doc_sep9', setTimeout, clearTimeout };
  return { page, ...attach(page, modules, profile) };
}

test('Sep 9 maps every existing consumer contract to the same page-owned identity', async () => {
  const modules = Object.fromEntries(Object.entries(prior).map(([role, keys]) => [role,
    Object.fromEntries(Object.keys(keys).map(key => [key, Object.freeze({ role, key })]))]));
  const f = fixture(modules);
  for (const role of Object.keys(prior)) {
    assert.deepEqual(Object.keys(profile.expectedExports[role]), Object.keys(prior[role]));
    const value = await f.api.load(role);
    for (const key of Object.keys(prior[role])) assert.equal(value[key], modules[role][key]);
    assert.equal(f.api.peek(role), value);
  }
  assert.equal(f.api.state().profile_id, profile.id);
  assert.equal(f.loads.length, 3);
  f.observed.clear();
  assert.equal((await f.api.load('shared')).H3, modules.shared.H3);
  assert.equal(f.loads.length, 3, 'warm access does not reconnect or import another module');
});

test('Sep 9 anchor proves the inspected dependencies after timing eviction', async () => {
  const f = fixture({ shared: { H3: () => true } });
  f.observed.clear(); f.observed.add(CDN + profile.anchor);
  assert.equal(f.api.observed('composer'), true);
  assert.equal((await f.api.load('shared')).H3(), true);
  assert.deepEqual(f.loads, [CDN + profile.files.shared]);
});

test('Sep 9 never accepts unrelated reused auth, tool or stop-state aliases', async () => {
  const f = fixture(), auth = () => true, status = () => ({ value: 3 });
  f.page.__elonChatGptPrivateRuntimeBindings = bindings.create(f.page, { loadRuntime: async () => ({
    [profile.expectedExports.shared.H3]: auth, zS: status, H3: 'wrong auth', c6: 'wrong role', Lx: 'wrong status',
    _k: 'other store', cs: 'other store', ls: 'other store', yk: 'other store', yu: 'other store'
  }) });
  const value = await f.page.__elonChatGptPrivateRuntimeBindings.load('shared');
  assert.equal(value.H3, auth); assert.equal(value.Fx, status);
  assert.equal(value._k, undefined); assert.equal(value.c6, undefined);
});

test('compiled tool and temporary owners stay paired with their exact current build', () => {
  const f = fixture();
  assert.equal(f.api.tools().owner, profile.toolOwner);
  assert.deepEqual(f.api.temporary(), profile.temporary);
  assert.equal(Object.isFrozen(f.api.tools()), true);
  f.observed.add(CDN + '8b34dbc2-nhot65scqrg20d6p.js');
  assert.equal(f.api.tools(), null); assert.equal(f.api.temporary(), null);
  assert.equal(f.api.state().error, 'runtime_build_ambiguous');
});

test('new-build document replacement cannot leak cached old account stores', async () => {
  const f = fixture({ shared: { R5: () => ({ authStatus: 'logged_in' }) } });
  await f.api.load('shared');
  f.page.__elonChatGptDocumentToken = 'doc_replaced'; f.observed.clear();
  assert.equal(f.api.peek('shared'), null);
  await assert.rejects(f.api.load('shared'), /runtime_not_observed/);
  assert.equal(f.loads.length, 1);
});

test('real stop and subsequent submit owners work through Sep 9 bindings without DOM dispatch', async () => {
  const { fixture: stopFixture, flush } = require('./fixtures/chatgpt-runtime-stop');
  const f = stopFixture(), bridge = attach(f.page, f.modules, profile);
  const stop = require('../android/app/src/main/assets/chatgpt_web_private_stop_runtime').create(f.page);
  const result = stop.stop(f.command); await flush();
  assert.equal(result.handled, true);
  assert.deepEqual(f.calls, [['synthetic-client-thread', 'synthetic-request', {
    clientInitiated: true, clientStopReason: 'user_stop_mouse'
  }]]);
  f.settle(); await flush();
  assert.equal(stop.state().pending, true, 'HTTP completion alone does not mean generation stopped');
  f.setState(false, null); f.publish();
  assert.equal((await result.completion).code, 'stop_observed');
  const sends = [];
  f.props.isDisabled = false; f.props.isComposerSubmissionReady = true;
  f.props.isConsumerLockdownModeLoadingForConversation = false;
  f.props.shouldBlockConsumerLockdownModeActionsForConversation = false;
  f.props.submitComposer = (...args) => {
    sends.push(args); return { accepted: true, completion: Promise.resolve(true) };
  };
  f.files.hasUploadInProgress$ = () => false;
  f.page.Event = class { constructor(type) { this.type = type; } preventDefault() {} };
  const sent = f.page.__elonChatGptPrivateTextRuntimeSubmit.submit({
    composer: f.node, requestId: 'mcp_followup', prompt: 'synthetic follow-up', expectedDraft: '',
    readDraft: () => '', clearDraft() {}, beforeSubmit() {}
  });
  assert.equal(sent.handled, true, sent.code);
  assert.equal((await sent.completion).code, 'accepted');
  assert.equal(sends.length, 1); assert.equal(f.calls.length, 1);
  assert.deepEqual(sends[0][1], { kind: 'text_action', text: 'synthetic follow-up' });
  assert.equal(f.loads.length, 0, 'no legacy bypass importer');
  assert.deepEqual(new Set(bridge.loads), new Set([CDN + profile.files.shared, CDN + profile.files.conversation]));
  assert.equal(f.timers.size, 0); assert.equal(f.listeners.size, 0);
});
});

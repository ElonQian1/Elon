'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const assets = path.join(__dirname, '../android/app/src/main/assets');
const modalApi = require(path.join(assets, 'chatgpt_web_new_chat_confirmation'));
const navigationApi = require(path.join(assets, 'chatgpt_web_private_new_conversation'));

function modalFixture(legacy = false) {
  const calls = [];
  const H = { logStructuredEvent() {}, logValueEventWithStatsig() {} }, q = H;
  const sm = { closeModal: value => calls.push(['close', value]) }, Sm = sm;
  const im = { NoAuthNewChat: 'guest' }, ym = im;
  const Zjt = () => {}, Cjt = Zjt, t = 'router';
  const lT = value => calls.push(['navigate', value]), UT = lT;
  const e = legacy ? ()=>{Sm.closeModal(ym.NoAuthNewChat)} : ()=>{sm.closeModal(im.NoAuthNewChat)};
  const clear = legacy
    ? ()=>{let n=`chatgpt_new_chat_modal_new_chat_button_clicked`,r=q.logStructuredEvent(`ChatgptNewChatModalClearConfirmed`,{eventName:n});q.logValueEventWithStatsig({segmentEventName:n,statsigEventName:n,eventId:r,statsigMetadata:{}}),Cjt({location:`New Chat Modal New Chat Button`}),UT(t),e()}
    : ()=>{let n=`chatgpt_new_chat_modal_new_chat_button_clicked`,r=H.logStructuredEvent(`ChatgptNewChatModalClearConfirmed`,{eventName:n});H.logValueEventWithStatsig({segmentEventName:n,statsigEventName:n,eventId:r,statsigMetadata:{}}),Zjt({location:`New Chat Modal New Chat Button`}),lT(t),e()};
  const root = { stateNode: {} }; root.stateNode.current = root;
  const owner = { return: root, type: legacy ? function xca() {} : function Mla() {}, memoizedProps: { onClose: e } };
  const action = { return: owner, memoizedProps: { color: 'primary', size: 'large',
    children: { props: { id: 'cBGbrD' } }, onClick: clear } };
  const host = { return: action };
  root.child = owner; owner.child = action; action.child = host;
  const button = { isConnected: true, __reactFiber$test: host, getAttribute: () => null,
    click() { throw Error('No DOM click allowed'); } };
  const buttons = [button], modal = { isConnected: true, querySelectorAll: () => buttons };
  const shared = { R5: () => ({ authStatus: 'logged_out' }), F5: () => null };
  const state = { profile_id: legacy ? 'web_20260906' : 'web_20260907' };
  const page = { document: { querySelector: () => modal }, __elonChatGptPrivateRuntimeBindings: {
    state: () => state, peek: () => shared } };
  return { api: modalApi.create(page), page, calls, root, owner, action, host, button, buttons, modal, shared, state };
}

for (const legacy of [false, true]) {
  test('verified ' + (legacy ? 'legacy' : 'current') + ' modal calls the official router and close exactly once', () => {
    const f = modalFixture(legacy), captured = f.api.capture();
    assert.ok(captured); assert.deepEqual(f.calls, []);
    assert.equal(f.api.run(captured, 'confirm'), true);
    assert.deepEqual(f.calls, [['navigate', 'router'], ['close', 'guest']]);
  });
  test('cancel only invokes the ' + (legacy ? 'legacy' : 'current') + ' close callback', () => {
    const f = modalFixture(legacy); assert.equal(f.api.run(f.api.capture(), 'cancel'), true);
    assert.deepEqual(f.calls, [['close', 'guest']]);
  });
}

for (const change of ['profile', 'logged_in', 'session_unknown', 'detached', 'disabled', 'label',
  'action', 'close', 'owner', 'uncommitted', 'stale_child', 'ambiguous', 'oversized']) {
  test('unknown or changed modal cannot clear a chat: ' + change, () => {
    const f = modalFixture(), before = f.api.capture();
    if (change === 'profile') f.state.profile_id = 'future';
    if (change === 'logged_in') f.shared.R5 = () => ({ authStatus: 'logged_in' });
    if (change === 'session_unknown') f.shared.F5 = () => ({});
    if (change === 'detached') f.modal.isConnected = false;
    if (change === 'disabled') f.button.disabled = true;
    if (change === 'label') f.action.memoizedProps.children.props.id = 'login';
    if (change === 'action') f.action.memoizedProps.onClick = () => {};
    if (change === 'close') f.owner.memoizedProps.onClose = () => {};
    if (change === 'owner') f.owner.type = function OtherModal() {};
    if (change === 'uncommitted') f.root.stateNode.current = {};
    if (change === 'stale_child') f.action.child = {};
    if (change === 'ambiguous') f.buttons.push(f.button);
    if (change === 'oversized') f.buttons.push(...Array(16).fill(f.button));
    assert.equal(f.api.capture(), null); assert.equal(f.api.run(before, 'confirm'), false);
    assert.deepEqual(f.calls, []);
  });
}

function navigationFixture() {
  let time = 0, serial = 0, ticketSerial = 0, modal = false, messages = 2, revision = 'unchanged', draft = '';
  const tasks = new Map(), results = [], writes = [];
  const page = { location: new URL('https://chatgpt.com/'), __elonChatGptDocumentToken: 'doc_confirmation',
    document: { querySelector: () => modal ? {} : null }, crypto: { getRandomValues: bytes => bytes.fill(++ticketSerial) },
    KeyboardEvent: class {}, setTimeout(fn, ms) { const id = ++serial; tasks.set(id, { at: time + ms, fn }); return id; },
    clearTimeout: id => tasks.delete(id) };
  const shared = { Ur: () => ({ id: 'newChat', isAvailable: true, disabled: false, scope: 'global',
    label: { id: 'keyboardActions.newChat' } }), zr: () => { modal = true; writes.push('open'); return true; } };
  page.__elonChatGptPrivateRuntimeBindings = { observed: () => true, peek: () => shared, load: async () => shared };
  page.__elonChatGptNewChatConfirmation = { capture: () => ({ fixture: true }), run: (_, decision) => {
    writes.push(decision); modal = false;
    if (decision === 'confirm') { messages = 0; revision = 'empty'; }
    return true;
  } };
  const api = navigationApi.create(page, { now: () => time });
  const inspect = () => ({ composerReady: true, messageCount: messages, revision, draft });
  const result = (...args) => results.push(args);
  const fallback = () => { throw Error('No navigation replay'); };
  return { api, page, results, writes, tasks, shared, inspect,
    start: value => api.start(inspect, result, fallback, value),
    request: (decision = 'confirm', ticket) => JSON.stringify({ decision,
      confirmationTicket: ticket || /\[confirmation_id:([a-f0-9]{32})\]/.exec(results[0]?.[2] || '')?.[1] }),
    messages: value => { messages = value; }, revision: value => { revision = value; }, draft: value => { draft = value; },
    modal: value => { modal = value; },
    tick() { const next = [...tasks].sort((a,b) => a[1].at - b[1].at)[0];
      if (next) { tasks.delete(next[0]); time = next[1].at; next[1].fn(); } },
    drain() { let limit = 100; while (tasks.size && limit-- > 0) this.tick(); assert.ok(limit > 0); }
  };
}

test('native approval uses a document-bound single-use ticket and settles the original new-chat action', () => {
  const f = navigationFixture(); f.start();
  assert.match(f.results[0][2], /confirmation_id:[a-f0-9]{32}/);
  assert.deepEqual(f.writes, ['open']); assert.equal(f.api.state().confirmationPending, true);
  const request = f.request(); f.start(request); f.drain();
  assert.deepEqual(f.writes, ['open', 'confirm']); assert.equal(f.results.at(-1)[1], true);
  f.start(request); assert.match(f.results.at(-1)[2], /confirmation_expired/);
  assert.deepEqual(f.writes, ['open', 'confirm']); assert.equal(f.tasks.size, 0);
});

test('native cancellation preserves messages and never issues another newChat action', () => {
  const f = navigationFixture(); f.start(); f.start(f.request('cancel'));
  assert.deepEqual(f.writes, ['open', 'cancel']); assert.equal(f.inspect().messageCount, 2);
  assert.match(f.results.at(-1)[2], /:cancelled/); assert.equal(f.tasks.size, 0);
});

test('cancel still closes the same guest modal after the user edits their draft', () => {
  const f = navigationFixture(); f.start(); f.draft('keep this text'); f.start(f.request('cancel'));
  assert.deepEqual(f.writes, ['open', 'cancel']); assert.equal(f.inspect().draft, 'keep this text');
});

test('an import cannot reset a newly edited draft before presenting confirmation', async () => {
  const f = navigationFixture(); let release;
  f.page.__elonChatGptPrivateRuntimeBindings.peek = () => null;
  f.page.__elonChatGptPrivateRuntimeBindings.load = () => new Promise(resolve => { release = resolve; });
  f.start(); await Promise.resolve(); f.draft('keep this text'); release(f.shared);
  await Promise.resolve(); await Promise.resolve(); await Promise.resolve();
  assert.deepEqual(f.writes, []); assert.match(f.results.at(-1)[2], /context_changed/);
});

for (const change of ['document', 'token', 'route', 'message_count', 'same_count_revision', 'draft']) {
  test('approval cannot discard changed context: ' + change, () => {
    const f = navigationFixture(); f.start(); const request = f.request();
    if (change === 'document') f.page.document = { querySelector: () => ({}) };
    if (change === 'token') f.page.__elonChatGptDocumentToken = 'doc_changed';
    if (change === 'route') f.page.location = new URL('https://chatgpt.com/c/11111111-2222-3333-4444-555555555555');
    if (change === 'message_count') f.messages(4);
    if (change === 'same_count_revision') f.revision('new response with same count');
    if (change === 'draft') f.draft('unsent');
    f.start(request); assert.match(f.results.at(-1)[2], /context_changed/);
    assert.deepEqual(f.writes, ['open']); assert.equal(f.tasks.size, 0);
  });
}

for (const change of ['expired', 'suspend', 'wrong_ticket', 'malformed', 'wrong_decision']) {
  test('invalid native decisions do not invoke the website: ' + change, () => {
    const f = navigationFixture(); f.start(); let request = f.request();
    if (change === 'expired') f.tick();
    if (change === 'suspend') f.api.suspend();
    if (change === 'wrong_ticket') request = f.request('confirm', 'a'.repeat(32));
    if (change === 'malformed') request = '{';
    if (change === 'wrong_decision') request = f.request('yes');
    f.start(request); assert.match(f.results.at(-1)[2], /confirmation_expired/);
    assert.deepEqual(f.writes, ['open']); f.api.suspend();
  });
}

test('a second new-chat click while confirming does not bypass the guest dialog', () => {
  const f = navigationFixture(); f.start(); f.start();
  assert.match(f.results.at(-1)[2], /:busy/); assert.deepEqual(f.writes, ['open']); f.api.suspend();
});

test('a callback throwing after its invocation is not retried', () => {
  const f = navigationFixture(); f.start(); const request = f.request();
  f.page.__elonChatGptNewChatConfirmation.run = () => { f.writes.push('confirm'); throw Error('after dispatch'); };
  f.start(request); f.start(request); assert.deepEqual(f.writes, ['open', 'confirm']);
});

test('unsupported official modal keeps the official confirmation available without a native ticket', () => {
  const f = navigationFixture(); delete f.page.__elonChatGptNewChatConfirmation;
  f.start(); assert.match(f.results[0][2], /confirmation_required/);
  assert.doesNotMatch(f.results[0][2], /confirmation_id/); assert.deepEqual(f.writes, ['open']);
});

test('production confirmation resolver loads before new-chat navigation', () => {
  const catalog = fs.readFileSync(path.join(assets, '../kotlin/com/elon/app/chatgptweb/ChatGptWebAdapterAssets.kt'), 'utf8');
  const index = catalog.indexOf('"chatgpt_web_new_chat_confirmation.js"');
  assert.ok(index > catalog.indexOf('"chatgpt_web_committed_owner_path.js"'));
  assert.ok(index < catalog.indexOf('"chatgpt_web_private_new_conversation.js"'));
});

test('an installed v294 bridge is replaced once without discarding identity or audio owners', () => {
  const adapter = fs.readFileSync(path.join(assets, '../kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt'), 'utf8');
  const version = Number(/ADAPTER_VERSION = (\d+)/.exec(adapter)[1]);
  const identity = {}, audio = {};
  let disposed = 0;
  const window = { __elonChatGptAdapterVersion: 294, __elonChatGptAdapterTargetVersion: version,
    __elonChatGptBridge: { dispose() { disposed++; } }, __elonChatGptConversations: { stale: true },
    __elonChatGptPrivateAuthContext: identity, __elonChatGptPrivateRealtimeVoice: audio };
  const bootstrap = fs.readFileSync(path.join(assets, 'chatgpt_web_adapter_bootstrap.js'), 'utf8');
  const context = { window, location: { origin: 'https://chatgpt.com' } };
  vm.runInNewContext(bootstrap, context);
  assert.equal(disposed, 1); assert.equal(window.__elonChatGptConversations, undefined);
  assert.equal(window.__elonChatGptAdapterVersion, version);
  assert.equal(window.__elonChatGptPrivateAuthContext, identity);
  assert.equal(window.__elonChatGptPrivateRealtimeVoice, audio);
  vm.runInNewContext(bootstrap, context); assert.equal(disposed, 1);
});

for (const reason of ['expired', 'suspend']) {
  test('reopening after ' + reason + ' rebinds the existing guest modal without another registered action', () => {
    const f = navigationFixture(); f.start(); const oldRequest = f.request();
    if (reason === 'expired') f.tick(); else f.api.suspend();
    f.start();
    assert.deepEqual(f.writes, ['open']);
    assert.match(f.results.at(-1)[2], /confirmation_required.*confirmation_id:/);
    const ticket = /confirmation_id:([a-f0-9]{32})/.exec(f.results.at(-1)[2])[1];
    f.start(oldRequest); assert.match(f.results.at(-1)[2], /confirmation_expired/);
    assert.deepEqual(f.writes, ['open']);
    f.start(f.request('confirm', ticket)); f.drain();
    assert.deepEqual(f.writes, ['open', 'confirm']); assert.equal(f.results.at(-1)[1], true);
  });
}

for (const unavailable of ['bindings', 'modal_bridge']) {
  test('an existing guest modal remains authoritative when ' + unavailable + ' is unavailable', () => {
    const f = navigationFixture(); f.modal(true);
    if (unavailable === 'bindings') delete f.page.__elonChatGptPrivateRuntimeBindings;
    else delete f.page.__elonChatGptNewChatConfirmation;
    assert.equal(f.start(), true);
    assert.deepEqual(f.writes, []); assert.match(f.results.at(-1)[2], /confirmation_required/);
    f.api.suspend();
  });
}

test('a guest modal appearing during import is not bypassed by the delayed registered action', async () => {
  const f = navigationFixture(); let release;
  f.page.__elonChatGptPrivateRuntimeBindings.peek = () => null;
  f.page.__elonChatGptPrivateRuntimeBindings.load = () => new Promise(resolve => { release = resolve; });
  f.start(); await Promise.resolve(); f.modal(true); release(f.shared);
  await Promise.resolve(); await Promise.resolve(); await Promise.resolve();
  assert.deepEqual(f.writes, []); assert.match(f.results.at(-1)[2], /confirmation_required/);
  f.api.suspend();
});

test('production conversation adapter passes decisions to the same runtime owner without DOM fallback', () => {
  const calls = [], results = [], value = JSON.stringify({ decision: 'cancel', confirmationTicket: 'a'.repeat(32) });
  const window = { __elonChatGptPrivateNewConversation: { version: 2, start: (...args) => { calls.push(args); return true; } } };
  const document = { querySelector() { throw Error('DOM fallback forbidden for a decision'); },
    querySelectorAll() { throw Error('DOM fallback forbidden for a decision'); } };
  vm.runInNewContext(fs.readFileSync(path.join(assets, 'chatgpt_web_adapter_conversations.js'), 'utf8'), {
    window, document, location: { origin: 'https://chatgpt.com' }
  });
  window.__elonChatGptConversations.newConversation(() => ({}), (...args) => results.push(args), value);
  assert.equal(calls[0][3], value);
  calls.length = 0;
  window.__elonChatGptPrivateNewConversation.version = 1;
  window.__elonChatGptConversations.newConversation(() => ({}), (...args) => results.push(args), value);
  assert.equal(calls.length, 0, 'old runtime must not interpret a decision as another new-chat request');
  assert.equal(results.at(-1)[1], false);
  delete window.__elonChatGptPrivateNewConversation;
  window.__elonChatGptConversations.newConversation(() => ({}), (...args) => results.push(args), value);
  assert.equal(results[0][1], false);
});

test('production command preserves streaming until acceptance and binds unchanged content plus draft', () => {
  const source = fs.readFileSync(path.join(assets, 'chatgpt_web_adapter.js'), 'utf8');
  const start = source.indexOf("    if (action === 'new_conversation') {");
  const end = source.indexOf("    respond(action || 'unknown'", start);
  const execute = new Function('command', 'action', 'comparableText', 'composerValue', 'findComposer',
    'messageAdapter', 'conversationAdapter', 'invalidatePrivateTextContext', 'streamingPolicy',
    'privateStreamTransport', 'respond', 'scheduleSnapshot', source.slice(start, end));
  const calls = [], composer = {}, messages = [{ id: 'fixture', role: 'assistant', text: 'synthetic' }];
  let draft = '', observed, settle;
  const invoke = value => execute({ value }, 'new_conversation', value => value, () => draft, () => composer,
    { readMessages: () => messages }, { newConversation(inspect, callback, decision) {
      observed = { snapshot: inspect(), decision }; settle = callback;
    } }, () => calls.push('invalidate'), { reset: () => calls.push('reset') },
    { reset: () => calls.push('private-reset') }, (...args) => calls.push(args), () => calls.push('snapshot'));
  invoke(); assert.deepEqual(calls, []);
  assert.equal(observed.snapshot.revision, JSON.stringify([['fixture', 'assistant', 'synthetic']]));
  settle('new_conversation', false, 'confirmation'); assert.deepEqual(calls, [['new_conversation', false, 'confirmation']]);
  calls.length = 0; draft = 'new unsent text'; const decision = JSON.stringify({ decision: 'cancel' });
  invoke(decision); assert.equal(observed.snapshot.draft, draft); assert.equal(observed.decision, decision);
  settle('new_conversation', true, 'ready'); assert.deepEqual(calls.slice(0, 3), ['invalidate', 'reset', 'private-reset']);
});

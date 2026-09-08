'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const assets = path.join(__dirname, '../android/app/src/main/assets');
const modulePath = path.join(assets, 'chatgpt_web_private_text_runtime_submit.js');
const moduleApi = require(modulePath);
const source = fs.readFileSync(modulePath, 'utf8');
const orchestrator = fs.readFileSync(path.join(assets, 'chatgpt_web_text_transaction_orchestrator.js'), 'utf8');
const id = '11111111-2222-3333-4444-555555555555';
const runtimeUrl = 'https://chatgpt.com/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js';
const flush = async () => { for (let i = 0; i < 12; i++) await Promise.resolve(); };

test('production loads the shared committed-owner resolver before runtime consumers', () => {
  const catalog = fs.readFileSync(path.join(assets, '../kotlin/com/elon/app/chatgptweb/ChatGptWebAdapterAssets.kt'), 'utf8');
  const resolver = catalog.indexOf('"chatgpt_web_committed_owner_path.js"');
  assert.ok(resolver > 0 && resolver < catalog.indexOf('"chatgpt_web_private_text_runtime_submit.js"'));
  assert.ok(resolver < catalog.indexOf('"chatgpt_web_private_new_conversation.js"'));
});

function fixture(guest, editor = false) {
  const calls = [], timers = new Map();
  let serial = 0, settle, draft = '', serverId = id, credentials = 'Bearer synthetic-only', loaded = true;
  let response = () => ({ accepted: true, completion: new Promise(resolve => { settle = resolve; }) });
  const conversation = { id: 'local-thread', serverId$: () => serverId };
  const controller = { conversation };
  const props = { conversation, composerController: controller, isDisabled: false,
    isNewThread: false, isComposerSubmissionReady: true, currentLeafId: 'current-leaf',
    isConsumerLockdownModeLoadingForConversation: false,
    shouldBlockConsumerLockdownModeActionsForConversation: false,
    submitComposer(...args) { calls.push(args); return response(); } };
  const shared = { getSharedProps: () => props, subscribeToSharedProps() {} };
  const fileStore = { files$: () => [], readyFiles$: () => [], hasUploadInProgress$: () => false };
  const top = { return: null, stateNode: {} }; top.stateNode.current = top;
  const fiber = { return: top, memoizedProps: {}, dependencies: { firstContext: {
    memoizedValue: { store: shared }, next: { memoizedValue: fileStore, next: null }
  } } };
  top.child = fiber;
  const node = { isConnected: true, __reactFiber$test: fiber };
  const page = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/' + id },
    document: { querySelectorAll: () => [] },
    __elonChatGptPrivateTextTransactionsEnabled: true, __elonChatGptDocumentToken: 'doc_runtime_submit',
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: credentials }) },
    performance: { getEntriesByName: url => loaded && url === runtimeUrl ? [{}] : [] },
    Event: class { constructor(type) { this.type = type; } preventDefault() {} },
    setTimeout(fn) { timers.set(++serial, fn); return serial; }, clearTimeout(key) { timers.delete(key); } };
  let bridge;
  const edits = [];
  const view = { dom: node, isDestroyed: false, state: { doc: { toJSON: () => ({
    type: 'doc', content: [{ type: 'paragraph', content: draft ? [{ type: 'text', text: draft }] : [] }]
  }) } } };
  if (guest) {
    page.__elonChatGptPrivateTransport = null;
    page.location.href = 'https://chatgpt.com/'; serverId = null; props.isNewThread = true;
  }
  if (guest || editor) {
    bridge = require('./fixtures/chatgpt-runtime-bindings').attach(page, { shared: guest ? {
      R5: () => { if (guest.throws) throw Error('synthetic bootstrap unavailable'); return { authStatus: guest.authStatus }; },
      F5: guest.missingGetter ? undefined : () => guest.session
    } : {}, composer: editor ? {
      t_: owner => { assert.equal(owner, controller); return view; },
      AS: doc => { assert.equal(doc, view.state.doc); return { content: draft }; },
      VS: (target, value, options) => {
        assert.equal(target, view); assert.deepEqual(options, { scrollIntoView: false });
        edits.push(value); draft = value;
      }
    } : {} });
  }
  page.__elonChatGptPrivateTextRuntimeSubmit = moduleApi.create(page);
  const command = { requestId: 'mcp_test', prompt: 'synthetic prompt', expectedDraft: '', composer: node,
    readDraft: () => draft, clearDraft: () => { draft = ''; }, beforeSubmit() {} };
  return { calls, props, shared, fileStore, fiber, top, node, page, command, controller,
    api: page.__elonChatGptPrivateTextRuntimeSubmit, timers, bridge, guest, view, edits,
    setDraft(value) { draft = value; }, draft: () => draft,
    setServer(value) { serverId = value; }, setIdentity(value) { credentials = value; },
    setLoaded(value) { loaded = value; }, response(value) { response = value; },
    settle(value) { settle(value); }, timeout() { for (const fn of [...timers.values()]) fn(); } };
}

test('official text action dispatches once without filling or clicking the website', async () => {
  const f = fixture(), result = f.api.submit(f.command);
  assert.equal(result.handled, true);
  assert.equal(f.api.state().pending, true);
  assert.equal(f.calls.length, 1);
  const [event, intent, options] = f.calls[0];
  assert.equal(event.type, 'submit');
  assert.deepEqual(intent, { kind: 'text_action', text: f.command.prompt });
  assert.deepEqual(options, { requireDispatchAcceptance: true });
  f.settle(true);
  assert.deepEqual(await result.completion, { status: 'accepted', code: 'accepted', current: true });
  assert.equal(f.api.state().pending, false);
  assert.equal(f.timers.size, 0);
});

test('current website composer accepts the native submit transaction without a legacy module', async () => {
  const f = fixture(), bridge = require('./fixtures/chatgpt-runtime-bindings').attach(f.page, {});
  const result = f.api.submit(f.command);
  assert.equal(result.handled, true); assert.equal(f.calls.length, 1);
  f.settle(true); assert.equal((await result.completion).status, 'accepted');
  assert.equal(bridge.loads.length, 0, 'uses the current committed submit owner without importing another composer');
});

for (const guest of [false, true]) {
  test('ordinary composer retains an inert structured-input host: guest=' + guest, async () => {
    const f = fixture(guest ? { authStatus: 'logged_out', session: null } : undefined);
    await flush();
    f.props.structuredInputHost = {
      canOpen$() { throw Error('host capability must not be invoked'); },
      tryOpen$() { throw Error('host must not be opened'); }
    };
    f.props.structuredInputMessageId = null;
    const result = f.api.submit(f.command);
    assert.equal(result.handled, true);
    assert.equal(f.calls.length, 1);
    f.settle(true);
    assert.equal((await result.completion).status, 'accepted');
    assert.equal(f.timers.size, 0);
  });
}

test('unrecognized structured hosts remain unavailable without invoking their members', () => {
  for (const host of [{}, false, 'host', { canOpen$() {} }, { tryOpen$() {} },
    { canOpen$: true, tryOpen$() {} }, { canOpen$() {}, tryOpen$() {}, unknown: true }]) {
    const f = fixture(); f.props.structuredInputHost = host;
    assert.equal(f.api.submit(f.command).handled, false);
    assert.equal(f.calls.length, 0);
  }
  const f = fixture(); f.props.structuredInputHost = { canOpen$() {}, tryOpen$() {} };
  assert.equal(f.api.submit(f.command).code, 'structured_host_unrecognized',
    'a recognized host still requires an explicit inactive message state');
  assert.equal(f.calls.length, 0);
});

test('active structured input and invalid thread modes still cannot use ordinary submission', () => {
  for (const mutate of [
    p => { p.structuredInputMessageId = 'synthetic-structured-message'; },
    p => { p.structuredInputMessageId = ''; },
    p => { p.isNewThread = undefined; },
    p => { p.isNewThread = 'false'; }
  ]) {
    const f = fixture();
    f.props.structuredInputHost = { canOpen$() {}, tryOpen$() {} };
    mutate(f.props);
    assert.equal(f.api.submit(f.command).handled, false);
    assert.equal(f.calls.length, 0);
  }
});

test('a replaced host or newly active structured input invalidates the captured submit', () => {
  for (const mutate of [
    p => { p.structuredInputHost = { canOpen$() {}, tryOpen$() {} }; },
    p => { p.structuredInputMessageId = 'synthetic-new-structured-input'; },
    p => { p.isNewThread = true; }
  ]) {
    const f = fixture(); f.props.structuredInputHost = { canOpen$() {}, tryOpen$() {} };
    f.props.structuredInputMessageId = null;
    f.command.beforeSubmit = () => mutate(f.props);
    assert.equal(f.api.submit(f.command).code, 'context_changed');
    assert.equal(f.calls.length, 0);
  }
});

test('confirmed guest uses the official text transaction without an authorization header', async () => {
  const f = fixture({ authStatus: 'logged_out', session: null }); await flush();
  assert.equal(f.bridge.loads.length, 2, 'identity and composer modules are warmed once');
  assert.equal(f.api.captureConversation(f.node), null, 'other runtime actions retain their authenticated contract');
  for (let i = 0; i < 2; i++) {
    const result = f.api.submit(f.command); assert.equal(result.handled, true);
    f.settle(true); assert.equal((await result.completion).status, 'accepted');
  }
  assert.equal(f.calls.length, 2); assert.equal(f.bridge.loads.length, 2);
  assert.equal(f.timers.size, 0);
});

test('cold guest identity never waits, queues a send, or replays the DOM fallback', async () => {
  const f = fixture({ authStatus: 'logged_out', session: null });
  assert.deepEqual(f.api.submit(f.command), { handled: false, code: 'identity_unavailable' });
  await flush(); assert.equal(f.calls.length, 0);
  assert.equal(f.api.state().pending, false); assert.equal(f.timers.size, 0);
  const result = f.api.submit(f.command); assert.equal(result.handled, true);
  f.settle(true); assert.equal((await result.completion).status, 'accepted');
});

test('legacy website guest uses its own verified bootstrap and session exports', async () => {
  const f = fixture(), loads = [], sharedUrl = 'https://chatgpt.com/cdn/assets/4813494d-hrplraurzfyvxb10.js';
  f.page.__elonChatGptPrivateTransport = null;
  f.page.performance.getEntriesByName = url => [runtimeUrl, sharedUrl].includes(url) ? [{}] : [];
  f.page.__elonChatGptPrivateRuntimeBindings = require('../android/app/src/main/assets/chatgpt_web_private_runtime_bindings').create(f.page, {
    loadRuntime: async url => { loads.push(url); return { R5: () => ({ authStatus: 'logged_out' }), F5: () => null }; }
  });
  const api = moduleApi.create(f.page); await flush();
  const result = api.submit(f.command); assert.equal(result.handled, true);
  f.settle(true); assert.equal((await result.completion).status, 'accepted');
  assert.deepEqual(loads, [sharedUrl, runtimeUrl]);
});

test('guest homepage retains its official owner after server ID assignment and on the next send', async () => {
  const f = fixture({ authStatus: 'logged_out', session: null }); await flush();
  const first = f.api.submit(f.command);
  f.setServer(id); f.settle(true);
  assert.equal((await first.completion).status, 'accepted');
  const second = f.api.submit(f.command);
  assert.equal(second.handled, true); f.settle(true);
  assert.equal((await second.completion).status, 'accepted');
  assert.equal(f.page.location.href, 'https://chatgpt.com/'); assert.equal(f.calls.length, 2);
});

test('guest root allowance never authorizes another server ID during an existing send', async () => {
  const f = fixture({ authStatus: 'logged_out', session: null }); await flush();
  f.setServer(id);
  const result = f.api.submit(f.command);
  f.setServer('99999999-2222-3333-4444-555555555555'); f.settle(true);
  assert.deepEqual(await result.completion, { status: 'accepted', code: 'accepted', current: false });
});

for (const state of ['invalid_server', 'authenticated_home', 'guest_project', 'guest_temporary', 'guest_other_route']) {
  test('homepage ownership exception remains narrow: ' + state, async () => {
    const f = fixture({ authStatus: 'logged_out', session: null }); await flush(); f.setServer(id);
    if (state === 'invalid_server') f.setServer('not-a-server-id');
    if (state === 'authenticated_home') f.page.__elonChatGptPrivateTransport = {
      copySameOriginRequestHeaders: () => ({ Authorization: 'Bearer synthetic' })
    };
    if (state === 'guest_project') f.page.location.href = 'https://chatgpt.com/g/g-p-' + 'a'.repeat(32) + '/project';
    if (state === 'guest_temporary') f.page.location.href += '?temporary-chat=true';
    if (state === 'guest_other_route') f.page.location.href += 'c/99999999-2222-3333-4444-555555555555';
    assert.equal(f.api.submit(f.command).code, 'conversation_route_mismatch');
    assert.equal(f.calls.length, 0);
  });
}

for (const guest of [false, true]) {
  test('official current draft works independently of explicit-action readiness: guest=' + guest, async () => {
    const f = fixture(guest ? { authStatus: 'logged_out', session: null } : null, true); await flush();
    f.props.isComposerSubmissionReady = false;
    const result = f.api.submit(f.command);
    assert.equal(result.handled, true); assert.deepEqual(f.edits, [f.command.prompt]);
    assert.deepEqual(f.calls[0][1], { kind: 'current_draft' });
    assert.equal(f.props.isComposerSubmissionReady, false, 'never forge official readiness');
    f.setDraft(''); f.settle(true);
    assert.equal((await result.completion).status, 'accepted');
    assert.equal(f.timers.size, 0);
  });
}

test('current draft preserves a new user edit and does not duplicate an already matching draft', async () => {
  const f = fixture(null, true); await flush(); f.props.isComposerSubmissionReady = false;
  f.setDraft(f.command.prompt); f.command.expectedDraft = f.command.prompt;
  const result = f.api.submit(f.command);
  assert.equal(f.edits.length, 0); f.setDraft('next user draft'); f.settle(true);
  assert.equal((await result.completion).status, 'accepted'); assert.equal(f.draft(), 'next user draft');
});

for (const change of ['editor_reset', 'controller_reset', 'guest_login', 'navigation']) {
  test('confirmed draft receipt survives UI lifecycle without mutating its successor: ' + change, async () => {
    const f = fixture({ authStatus: 'logged_out', session: null }, true); await flush();
    f.props.isComposerSubmissionReady = false;
    const result = f.api.submit(f.command);
    if (change === 'editor_reset') f.node.isConnected = false;
    if (change === 'controller_reset') f.props.composerController = { conversation: f.controller.conversation };
    if (change === 'guest_login') f.guest.session = {};
    if (change === 'navigation') f.page.location.href = 'https://chatgpt.com/c/99999999-2222-3333-4444-555555555555';
    f.setDraft('new context draft'); f.settle(true);
    assert.deepEqual(await result.completion, { status: 'accepted', code: 'accepted', current: false });
    assert.equal(f.draft(), 'new context draft'); assert.equal(f.edits.length, 1); assert.equal(f.calls.length, 1);
    assert.equal(f.api.state().pending, false);
  });
}

test('changed UI cannot turn unconfirmed current-draft completion into success', async () => {
  for (const value of [false, undefined, null, { accepted: true }]) {
    const f = fixture(null, true); await flush(); f.props.isComposerSubmissionReady = false;
    const result = f.api.submit(f.command); f.node.isConnected = false; f.settle(value);
    assert.equal((await result.completion).status, 'unknown'); assert.equal(f.calls.length, 1);
  }
});

test('runtime draft acceptance updates the receipt but never starts streaming in a successor page', async () => {
  const f = fixture(null, true); await flush(); f.props.isComposerSubmissionReady = false;
  let streamingStarts = 0, domWrites = 0;
  const events = [];
  vm.runInNewContext(orchestrator, { window: f.page });
  const api = f.page.__elonChatGptTextTransactionOrchestrator.create({
    findComposer: () => f.node, composerValue: f.command.readDraft,
    setComposerValue: () => { domWrites++; throw Error('unexpected DOM mutation'); },
    comparableText: value => value, scheduleSnapshot() {},
    streamingPolicy: { begin() { streamingStarts++; } }
  });
  const respond = (action, ok, detail) => events.push({ action, ok, detail }); respond.requestId = 'mcp_test';
  api.sendPrompt(f.command.prompt, '', respond, true);
  f.node.isConnected = false; f.settle(true); await flush();
  assert.deepEqual(events, [{ action: 'send_prompt', ok: true, detail: 'official_runtime_v1:accepted' }]);
  assert.equal(streamingStarts, 0); assert.equal(domWrites, 0); assert.equal(f.calls.length, 1);
});

for (const fault of ['unmounted_editor', 'destroyed_editor', 'rich_input', 'unknown_ready', 'owner_changed', 'draft_changed']) {
  test('runtime draft handoff rejects before any mutation: ' + fault, async () => {
    const f = fixture(null, true); await flush(); f.props.isComposerSubmissionReady = false;
    if (fault === 'unmounted_editor') f.view.dom = {};
    if (fault === 'destroyed_editor') f.view.isDestroyed = true;
    if (fault === 'rich_input') f.view.state.doc.toJSON = () => ({ type: 'doc', content: [{
      type: 'paragraph', content: [{ type: 'inline_selection_pill' }]
    }] });
    if (fault === 'unknown_ready') f.props.isComposerSubmissionReady = undefined;
    if (fault === 'owner_changed') f.command.beforeSubmit = () => { f.props.currentLeafId = 'changed'; };
    if (fault === 'draft_changed') f.command.beforeSubmit = () => f.setDraft('edited by user');
    assert.equal(f.api.submit(f.command).handled, false);
    assert.equal(f.edits.length, 0); assert.equal(f.calls.length, 0);
  });
}

test('official current-draft rejection and uncertain dispatch never retry a second writer', async () => {
  for (const throws of [false, true]) {
    const f = fixture(null, true); await flush(); f.props.isComposerSubmissionReady = false;
    f.response(() => { if (throws) throw Error('uncertain'); return { accepted: false }; });
    const result = f.api.submit(f.command); assert.equal(result.handled, true);
    assert.equal((await result.completion).status, throws ? 'unknown' : 'rejected');
    assert.equal(f.calls.length, 1);
    if (throws) assert.equal((await f.api.submit(f.command).completion).code, 'busy');
  }
});

test('empty or invalid captured headers cannot stand in for authenticated identity', () => {
  for (const headers of [{}, { 'oai-device-id': 'synthetic' }, { Authorization: '' }, { Authorization: 'Bearer ' }]) {
    const f = fixture(); f.page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders = () => headers;
    assert.equal(f.api.submit(f.command).code, 'identity_unavailable'); assert.equal(f.calls.length, 0);
  }
});

for (const state of [
  { authStatus: 'logged_in', session: null },
  { authStatus: 'loading', session: null },
  { authStatus: undefined, session: null },
  { authStatus: 'logged_out', session: undefined },
  { authStatus: 'logged_out', session: {} },
  { authStatus: 'logged_out', session: null, missingGetter: true },
  { authStatus: 'logged_out', session: null, throws: true }
]) {
  test('missing headers alone are not guest proof: ' + JSON.stringify(state), async () => {
    const f = fixture(state); await flush();
    assert.equal(f.api.submit(f.command).handled, false); assert.equal(f.calls.length, 0);
    assert.equal(f.api.state().pending, false); assert.equal(f.timers.size, 0);
  });
}

for (const [name, change] of Object.entries({
  login: f => { f.guest.session = {}; },
  headers: f => { f.page.__elonChatGptPrivateTransport = { copySameOriginRequestHeaders: () => ({ Authorization: 'Bearer synthetic-login' }) }; },
  document: f => { f.page.document = { querySelectorAll: () => [] }; },
  token: f => { f.page.__elonChatGptDocumentToken = 'doc_changed'; },
  mixed_build: f => { f.bridge.observed.add(runtimeUrl); },
  unavailable_auth: f => { f.guest.authStatus = 'unknown'; }
})) {
  test('guest ' + name + ' before dispatch cannot invoke the writer', async () => {
    const f = fixture({ authStatus: 'logged_out', session: null }); await flush();
    f.command.beforeSubmit = () => change(f);
    assert.equal(f.api.submit(f.command).code, 'context_changed'); assert.equal(f.calls.length, 0);
    await flush();
  });
  test('guest ' + name + ' after dispatch cannot clear the changed context', async () => {
    const f = fixture({ authStatus: 'logged_out', session: null }); await flush();
    f.setDraft(f.command.prompt); f.command.expectedDraft = f.command.prompt;
    const result = f.api.submit(f.command); assert.equal(result.handled, true);
    change(f); f.settle(true);
    assert.deepEqual(await result.completion, { status: 'accepted', code: 'accepted', current: false });
    assert.equal(f.draft(), f.command.prompt); assert.equal(f.calls.length, 1);
    await flush();
  });
}

test('guest invocation uncertainty keeps single-writer ownership', async () => {
  const f = fixture({ authStatus: 'logged_out', session: null }); await flush();
  f.response(() => { throw Error('synthetic dispatch uncertainty'); });
  assert.equal((await f.api.submit(f.command).completion).status, 'unknown');
  assert.equal((await f.api.submit(f.command).completion).code, 'busy');
  assert.equal(f.calls.length, 1);
});

test('matching draft is cleared only after accepted official dispatch', async () => {
  const f = fixture(); f.setDraft(f.command.prompt); f.command.expectedDraft = f.command.prompt;
  const result = f.api.submit(f.command);
  assert.equal(f.draft(), f.command.prompt);
  f.settle(true); await result.completion;
  assert.equal(f.draft(), '');
});

test('a local explicit-draft cleanup exception cannot revoke a confirmed send', async () => {
  const f = fixture(); f.setDraft(f.command.prompt); f.command.expectedDraft = f.command.prompt;
  f.command.clearDraft = () => { throw Error('local cleanup failed'); };
  const result = f.api.submit(f.command); f.settle(true);
  assert.equal((await result.completion).status, 'accepted');
  assert.equal(f.calls.length, 1); assert.equal(f.draft(), f.command.prompt);
});

test('unconfirmed completions expose only a fixed result class, never runtime values', async () => {
  for (const [value, code] of [[false, 'false'], [undefined, 'void'], [null, 'void'],
    [{ private: 'must-not-emit' }, 'shape'], ['must-not-emit', 'shape']]) {
    const f = fixture(), result = f.api.submit(f.command); f.node.isConnected = false; f.settle(value);
    assert.deepEqual(await result.completion, { status: 'unknown', code: 'dispatch_unconfirmed_' + code });
  }
});

test('edits made while dispatching are preserved', async () => {
  const f = fixture(); f.setDraft(f.command.prompt); f.command.expectedDraft = f.command.prompt;
  const result = f.api.submit(f.command);
  f.setDraft('new draft'); f.settle(true);
  assert.equal((await result.completion).status, 'accepted');
  assert.equal(f.draft(), 'new draft');
});

for (const [name, change] of Object.entries({
  disabled: f => { f.page.__elonChatGptPrivateTextTransactionsEnabled = false; },
  unknown_runtime: f => f.setLoaded(false),
  detached: f => { f.node.isConnected = false; },
  uncommitted_fiber: f => { f.top.stateNode.current = {}; },
  identity_unavailable: f => { f.page.__elonChatGptPrivateTransport = null; },
  document_unavailable: f => { f.page.__elonChatGptDocumentToken = ''; },
  wrong_conversation: f => f.setServer('99999999-2222-3333-4444-555555555555'),
  wrong_controller: f => { f.props.composerController = { conversation: {} }; },
  shared_chat: f => { f.page.location.href = 'https://chatgpt.com/share/' + id; },
  unexpected_query: f => { f.page.location.href += '?other=1'; },
  unready: f => { f.props.isComposerSubmissionReady = false; },
  disabled_composer: f => { f.props.isDisabled = true; },
  unknown_lockdown: f => { f.props.isConsumerLockdownModeLoadingForConversation = true; },
  structured_input: f => { f.props.structuredInputHost = {}; },
  pending_files: f => { f.fileStore.files$ = () => [{ status: 'uploading' }]; },
  ready_files: f => { f.fileStore.readyFiles$ = () => [{ status: 'ready' }]; },
  uploading: f => { f.fileStore.hasUploadInProgress$ = () => true; },
  changed_draft: f => f.setDraft('unrelated draft'),
  another_expected_draft: f => { f.setDraft('other'); f.command.expectedDraft = 'other'; },
  blank_prompt: f => { f.command.prompt = ' '; },
  invalid_request: f => { f.command.requestId = ''; },
  branch_changed_before_dispatch: f => { f.command.beforeSubmit = () => { f.props.currentLeafId = 'another'; }; },
  draft_changed_before_dispatch: f => { f.command.beforeSubmit = () => f.setDraft('edited'); },
})) {
  test(name + ' never invokes the official writer', () => {
    const f = fixture(); change(f);
    assert.equal(f.api.submit(f.command).handled, false);
    assert.equal(f.calls.length, 0);
    assert.equal(f.timers.size, 0);
  });
}

test('committed alternate is preferred to stale host fiber props', async () => {
  const f = fixture(), staleTop = { stateNode: { current: {} } };
  f.node.__reactFiber$test = { return: staleTop, alternate: f.fiber };
  const result = f.api.submit(f.command); f.settle(true);
  assert.equal((await result.completion).status, 'accepted');
});

test('bailed-out child follows the committed parent rather than its stale return pointer', async () => {
  const f = fixture(), oldRoot = { stateNode: f.top.stateNode };
  const currentParent = { return: f.top, child: f.fiber, dependencies: f.fiber.dependencies };
  const oldParent = { return: oldRoot, child: f.fiber, alternate: currentParent };
  currentParent.alternate = oldParent;
  oldRoot.child = oldParent; f.top.child = currentParent;
  f.fiber.return = oldParent; f.fiber.dependencies = null;
  const result = f.api.submit(f.command);
  assert.equal(result.handled, true);
  assert.equal(f.calls.length, 1);
  f.settle(true);
  assert.equal((await result.completion).status, 'accepted');
});

test('a return chain reaching current root cannot authorize a child outside that tree', async () => {
  const f = fixture(), obsolete = [];
  const staleStore = { ...f.shared, getSharedProps: () => ({ ...f.props,
    submitComposer() { obsolete.push(true); return { accepted: false }; }
  }) };
  const stale = { return: f.top, alternate: f.fiber, dependencies: { firstContext: {
    memoizedValue: { store: staleStore }, next: { memoizedValue: f.fileStore }
  } } };
  f.fiber.alternate = stale; f.node.__reactFiber$test = stale;
  const result = f.api.submit(f.command);
  assert.equal(result.handled, true);
  assert.equal(obsolete.length, 0, 'stale state must never be invoked');
  assert.equal(f.calls.length, 1);
  f.settle(true); assert.equal((await result.completion).status, 'accepted');
});

for (const [code, change] of Object.entries({
  react_owner_uncommitted: f => { f.top.child = null; },
  react_owner_ambiguous: f => {
    const alternate = { ...f.fiber, alternate: f.fiber };
    f.fiber.alternate = alternate; f.fiber.sibling = alternate;
  },
  react_owner_cycle: f => {
    const parent = { return: null, child: f.fiber };
    f.fiber.return = parent; f.fiber.child = parent; parent.return = f.fiber;
  },
  react_owner_child_limit: f => {
    for (let i = 0; i < 512; i++) f.top.child = { sibling: f.top.child };
  },
  react_owner_depth_limit: f => {
    for (let i = 0; i < 512; i++) {
      const parent = { return: f.fiber.return, child: f.fiber };
      f.fiber.return.child = parent; f.fiber.return = parent;
    }
  }
})) {
  test('current-tree resolution rejects its exact structural fault: ' + code, () => {
    const f = fixture(); change(f);
    assert.deepEqual(f.api.submit(f.command), { handled: false, code });
    assert.equal(f.calls.length, 0); assert.equal(f.timers.size, 0);
  });
}

test('sibling traversal cycles fail closed without polling or invoking a writer', () => {
  const f = fixture(), sibling = {};
  sibling.sibling = sibling; f.top.child = sibling;
  assert.deepEqual(f.api.submit(f.command), { handled: false, code: 'react_owner_cycle' });
  assert.equal(f.calls.length, 0);
});

test('reused children across several alternate parents resolve without exponential search', async () => {
  const f = fixture();
  let current = f.top, stale = { stateNode: f.top.stateNode };
  for (let i = 0; i < 35; i++) {
    const next = { return: current }, previous = { return: stale, alternate: next };
    next.alternate = previous;
    current.child = stale.child = next;
    current = next; stale = previous;
  }
  current.child = stale.child = f.fiber; f.fiber.return = stale;
  const result = f.api.submit(f.command);
  assert.equal(result.handled, true); assert.equal(f.calls.length, 1);
  f.settle(true); assert.equal((await result.completion).status, 'accepted');
});

for (const count of [120, 400, 510]) {
  test('a deep committed composer keeps a bounded path without the old 90-level ceiling: ' + count, () => {
    const f = fixture(); let childrenRead = 0;
    for (let i = 0; i < count; i++) {
      const parent = { return: f.fiber.return, child: f.fiber };
      f.fiber.return.child = parent; f.fiber.return = parent;
    }
    for (let parent = f.fiber.return; parent; parent = parent.return) {
      const child = parent.child;
      Object.defineProperty(parent, 'child', { get() { childrenRead++; return child; } });
    }
    const context = f.api.captureConversation(f.node);
    assert.equal(context?.shared, f.shared);
    assert.equal(context?.controller, f.controller);
    assert.equal(childrenRead, count + 1, 'child membership is scanned once per visited parent');
    assert.equal(f.calls.length, 0); assert.equal(f.timers.size, 0);
  });
}

test('current-tree removal between capture and dispatch preserves the native draft', () => {
  const f = fixture(); f.setDraft(f.command.prompt); f.command.expectedDraft = f.command.prompt;
  f.command.beforeSubmit = () => { f.top.child = null; };
  assert.equal(f.api.submit(f.command).code, 'context_changed');
  assert.equal(f.calls.length, 0); assert.equal(f.draft(), f.command.prompt);
});

test('current-tree removal does not revoke confirmed dispatch or clear another draft', async () => {
  const f = fixture(); f.setDraft(f.command.prompt); f.command.expectedDraft = f.command.prompt;
  const result = f.api.submit(f.command);
  f.top.child = null; f.settle(true);
  assert.deepEqual(await result.completion, { status: 'accepted', code: 'accepted', current: false });
  assert.equal(f.calls.length, 1); assert.equal(f.draft(), f.command.prompt);
});

test('imperatively mounted official editor resolves its nearest committed React host', async () => {
  const f = fixture();
  const host = { isConnected: true, __reactFiber$test: f.fiber };
  delete f.node.__reactFiber$test;
  f.node.parentElement = { isConnected: true, parentElement: host };
  const result = f.api.submit(f.command);
  assert.equal(result.handled, true);
  assert.equal(f.calls.length, 1);
  f.settle(true);
  assert.equal((await result.completion).status, 'accepted');
});

for (const condition of ['uncommitted', 'body', 'detached', 'limit', 'wrong_conversation']) {
  test('imperative editor does not escape its owner boundary: ' + condition, () => {
    const f = fixture(), validHost = { isConnected: true, __reactFiber$test: f.fiber };
    delete f.node.__reactFiber$test;
    f.node.parentElement = validHost;
    if (condition === 'uncommitted') f.node.__reactFiber$stale = { return: { stateNode: { current: {} } } };
    if (condition === 'body') f.page.document.body = validHost;
    if (condition === 'detached') validHost.isConnected = false;
    if (condition === 'limit') for (let i = 0; i < 12; i++) f.node.parentElement = { isConnected: true, parentElement: f.node.parentElement };
    if (condition === 'wrong_conversation') f.setServer('99999999-2222-3333-4444-555555555555');
    assert.equal(f.api.submit(f.command).handled, false);
    assert.equal(f.calls.length, 0);
  });
}

test('reparenting the imperative editor before dispatch invalidates the captured owner', () => {
  const f = fixture(), replacement = fixture();
  delete f.node.__reactFiber$test;
  f.node.parentElement = { isConnected: true, __reactFiber$test: f.fiber };
  f.command.beforeSubmit = () => { f.node.parentElement = replacement.node; };
  assert.equal(f.api.submit(f.command).code, 'context_changed');
  assert.equal(f.calls.length, 0);
  assert.equal(replacement.calls.length, 0);
});

for (const [code, change] of Object.entries({
  react_owner_unavailable: f => { delete f.node.__reactFiber$test; },
  shared_store_unavailable: f => { f.fiber.dependencies.firstContext = f.fiber.dependencies.firstContext.next; },
  file_store_unavailable: f => { f.fiber.dependencies.firstContext.next = null; },
  runtime_not_observed: f => f.setLoaded(false),
  identity_unavailable: f => { f.page.__elonChatGptPrivateTransport = null; },
  conversation_route_mismatch: f => f.setServer('99999999-2222-3333-4444-555555555555'),
  submission_not_ready: f => { f.props.isComposerSubmissionReady = false; },
  attachment_not_owned: f => { f.fileStore.readyFiles$ = () => [{ status: 'ready' }]; },
  draft_mismatch: f => f.setDraft('another draft'),
})) {
  test('pre-dispatch rejection identifies its structural gate: ' + code, () => {
    const f = fixture(); change(f);
    assert.deepEqual(f.api.submit(f.command), { handled: false, code });
    assert.equal(f.calls.length, 0);
  });
}

test('ambiguous context stores are not guessed', () => {
  const f = fixture(); f.fiber.memoizedProps.value = { store: { ...f.shared } };
  assert.equal(f.api.submit(f.command).handled, false);
  assert.equal(f.calls.length, 0);
});

test('new conversation can acquire a server id without losing the accepted receipt', async () => {
  const f = fixture(); f.setServer(null); f.page.location.href = 'https://chatgpt.com/';
  f.props.isNewThread = true;
  const result = f.api.submit(f.command);
  f.setServer(id); f.page.location.href += 'c/' + id; f.settle(true);
  assert.equal((await result.completion).status, 'accepted');
});

for (const [name, change] of Object.entries({
  account: f => f.setIdentity('Bearer another-identity'),
  document: f => { f.page.__elonChatGptDocumentToken = 'doc_replaced'; },
  conversation: f => { f.props.conversation = { serverId$: () => id }; },
  route: f => { f.page.location.href = 'https://chatgpt.com/c/99999999-2222-3333-4444-555555555555'; },
  privacy_mode: f => { f.page.location.href += '?temporary-chat=true'; },
  detached: f => { f.node.isConnected = false; },
})) {
  test(name + ' change during submission does not clear the new context', async () => {
    const f = fixture(); f.setDraft(f.command.prompt); f.command.expectedDraft = f.command.prompt;
    const result = f.api.submit(f.command); change(f); f.settle(true);
    assert.deepEqual(await result.completion, { status: 'accepted', code: 'accepted', current: false });
    assert.equal(f.draft(), f.command.prompt);
  });
}

test('gate rejection has no second transport attempt', async () => {
  const f = fixture(); f.response(() => ({ accepted: false }));
  assert.equal((await f.api.submit(f.command).completion).status, 'rejected');
  assert.equal(f.calls.length, 1);
  assert.equal(f.api.state().pending, false);
});

for (const route of [
  'https://chatgpt.com/?temporary-chat=true',
  'https://chatgpt.com/g/g-p-' + 'a'.repeat(32) + '/project',
]) {
  test('official current context remains responsible for new temporary/project text: ' + route, async () => {
    const f = fixture(); f.setServer(null); f.page.location.href = route; f.props.isNewThread = true;
    const result = f.api.submit(f.command); f.settle(true);
    assert.equal((await result.completion).status, 'accepted');
    assert.deepEqual(f.calls[0][1], { kind: 'text_action', text: f.command.prompt });
  });
}

for (const [name, response] of Object.entries({
  throw: () => { throw Error('synthetic internal failure'); },
  missing: () => undefined,
  missing_completion: () => ({ accepted: true }),
})) {
  test(name + ' after invocation remains unknown and cannot start another write', async () => {
    const f = fixture(); f.response(response);
    assert.equal((await f.api.submit(f.command).completion).status, 'unknown');
    assert.equal((await f.api.submit({ ...f.command, requestId: 'mcp_again' }).completion).code, 'busy');
    assert.equal(f.calls.length, 1);
  });
}

test('observation timeout does not cancel ownership or initiate replay', async () => {
  const f = fixture(), result = f.api.submit(f.command);
  f.timeout();
  assert.equal((await result.completion).code, 'timeout');
  assert.equal((await f.api.submit({ ...f.command, requestId: 'mcp_again' }).completion).code, 'busy');
  assert.equal(f.calls.length, 1);
  f.settle(true); await flush();
  assert.equal(f.api.state().pending, false);
});

test('resolved false and rejected completions are not success', async () => {
  for (const completion of [Promise.resolve(false), Promise.resolve({ accepted: true })]) {
    const f = fixture(); f.response(() => ({ accepted: true, completion }));
    assert.equal((await f.api.submit(f.command).completion).status, 'unknown');
  }
  const f = fixture(); f.response(() => ({ accepted: true, completion: Promise.reject(Error('synthetic')) }));
  assert.equal((await f.api.submit(f.command).completion).code, 'completion_failed');
});

test('same-document reinjection preserves in-flight ownership', () => {
  const f = fixture(); f.api.submit(f.command);
  vm.runInNewContext(source, { window: f.page });
  assert.equal(f.page.__elonChatGptPrivateTextRuntimeSubmit, f.api);
  assert.equal(f.api.state().pending, true);
  f.settle(true);
});

test('pending runtime regeneration blocks native text before draft mutation or another write', async () => {
  const f = fixture();
  f.page.__elonChatGptPrivateRegenerateRuntime = { state: () => ({ pending: true }) };
  assert.equal((await f.api.submit(f.command).completion).code, 'busy');
  assert.equal(f.calls.length, 0);
});

test('production orchestrator prioritizes the runtime and never clicks after an uncertain write', async () => {
  for (const timeout of [false, true]) {
    const f = fixture(), events = [];
    let clicked = 0, relay = 0, prepared = 0;
    f.page.__elonChatGptPrivateStreamTransport = { prepareSend() { prepared++; } };
    f.page.__elonChatGptPrivateTextTransactionRelay = { dispatch() { relay++; return { dispatched: false }; } };
    vm.runInNewContext(orchestrator, { window: f.page });
    const api = f.page.__elonChatGptTextTransactionOrchestrator.create({
      findComposer: () => f.node, composerValue: f.command.readDraft,
      setComposerValue: () => { throw Error('unexpected DOM write'); }, comparableText: value => value,
      scheduleSnapshot() {}, findButton: () => ({ click() { clicked++; } }),
    });
    const respond = (action, ok, detail) => events.push({ action, ok, detail }); respond.requestId = 'mcp_test';
    api.sendPrompt(f.command.prompt, '', respond, true);
    if (timeout) f.timeout(); else f.settle(true);
    await flush();
    assert.equal(prepared, 1); assert.equal(clicked, 0); assert.equal(relay, 0);
    assert.equal(events.length, 1); assert.equal(events[0].ok, !timeout);
    assert.equal(events[0].detail, timeout ? 'official_runtime_v1:unknown:timeout' : 'official_runtime_v1:accepted');
    if (timeout) f.settle(false);
  }
});

test('production module registration precedes the actual send orchestrator', () => {
  const adapter = require('./chatgpt-web-adapter-assembly').readAdapterSource();
  assert.ok(adapter.indexOf('"chatgpt_web_private_text_runtime_submit.js"') < adapter.indexOf('"chatgpt_web_text_transaction_orchestrator.js"'));
  const names = [...adapter.split('private val ADAPTER_ASSETS = listOf(')[1].split(')')[0].matchAll(/"([a-z0-9_]+\.js)"/g)].map(match => match[1]);
  new vm.Script(names.map(name => fs.readFileSync(path.join(assets, name), 'utf8')).join('\n'));
  assert.doesNotMatch(source, /\bfetch\s*\(|\.click\s*\(|document\.cookie|localStorage/);
});

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

function fixture() {
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
  const node = { isConnected: true, __reactFiber$test: fiber };
  const page = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/' + id },
    document: { querySelectorAll: () => [] },
    __elonChatGptPrivateTextTransactionsEnabled: true, __elonChatGptDocumentToken: 'doc_runtime_submit',
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: credentials }) },
    performance: { getEntriesByName: url => loaded && url === runtimeUrl ? [{}] : [] },
    Event: class { constructor(type) { this.type = type; } preventDefault() {} },
    setTimeout(fn) { timers.set(++serial, fn); return serial; }, clearTimeout(key) { timers.delete(key); } };
  page.__elonChatGptPrivateTextRuntimeSubmit = moduleApi.create(page);
  const command = { requestId: 'mcp_test', prompt: 'synthetic prompt', expectedDraft: '', composer: node,
    readDraft: () => draft, clearDraft: () => { draft = ''; }, beforeSubmit() {} };
  return { calls, props, shared, fileStore, fiber, top, node, page, command, controller,
    api: page.__elonChatGptPrivateTextRuntimeSubmit, timers,
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
  assert.deepEqual(await result.completion, { status: 'accepted', code: 'accepted' });
  assert.equal(f.api.state().pending, false);
  assert.equal(f.timers.size, 0);
});

test('matching draft is cleared only after accepted official dispatch', async () => {
  const f = fixture(); f.setDraft(f.command.prompt); f.command.expectedDraft = f.command.prompt;
  const result = f.api.submit(f.command);
  assert.equal(f.draft(), f.command.prompt);
  f.settle(true); await result.completion;
  assert.equal(f.draft(), '');
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
    assert.equal((await result.completion).code, 'context_changed');
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
  const adapter = fs.readFileSync(path.join(__dirname,
    '../android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt'), 'utf8');
  assert.ok(adapter.indexOf('"chatgpt_web_private_text_runtime_submit.js"') < adapter.indexOf('"chatgpt_web_text_transaction_orchestrator.js"'));
  const names = [...adapter.split('private val ADAPTER_ASSETS = listOf(')[1].split(')')[0].matchAll(/"([a-z0-9_]+\.js)"/g)].map(match => match[1]);
  new vm.Script(names.map(name => fs.readFileSync(path.join(assets, name), 'utf8')).join('\n'));
  assert.doesNotMatch(source, /\bfetch\s*\(|\.click\s*\(|document\.cookie|localStorage/);
});

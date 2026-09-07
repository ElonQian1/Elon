'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const assets = path.join(__dirname, '../android/app/src/main/assets');
const composerModule = require(path.join(assets, 'chatgpt_web_private_attachment_composer.js'));
const senderModule = require(path.join(assets, 'chatgpt_web_private_attachment_send.js'));
const runtimeModule = require(path.join(assets, 'chatgpt_web_private_text_runtime_submit.js'));
const runtimeSource = fs.readFileSync(path.join(assets, 'chatgpt_web_private_text_runtime_submit.js'), 'utf8');
const orchestratorSource = fs.readFileSync(path.join(assets, 'chatgpt_web_text_transaction_orchestrator.js'), 'utf8');
const protocol = require(path.join(assets, 'chatgpt_web_private_attachment_protocol.js'));
const id = '11111111-2222-3333-4444-555555555555';
const flush = async () => { for (let i = 0; i < 16; i++) await Promise.resolve(); };

function fixture({ temporary = false, image = false, reused = false, count = 1 } = {}) {
  let values = [], serverId = null, identity = 'Bearer synthetic-only', draft = '', settle;
  let response = () => ({ accepted: true, completion: new Promise(resolve => { settle = resolve; }) });
  const calls = [], events = [], timers = new Map(), counts = { prepared: 0, click: 0, relay: 0, draftWrites: 0 };
  const files$ = () => values;
  files$.set = next => { values = next; };
  const store = { files$, readyFiles$: () => values.filter(item => item.status === 'ready'), hasUploadInProgress$: () => false };
  const conversation = { serverId$: () => serverId };
  const props = { conversation, composerController: { conversation }, currentLeafId: 'synthetic-leaf',
    isNewThread: true, isDisabled: false, isComposerSubmissionReady: true,
    isConsumerLockdownModeLoadingForConversation: false, shouldBlockConsumerLockdownModeActionsForConversation: false,
    currentModelId: 'synthetic-model', onCreateNewCompletion() {}, entrySurface: 'chat_composer', isLibraryEnabled: true,
    submitComposer(...args) { calls.push(args); return response(); } };
  const shared = { getSharedProps: () => props, subscribeToSharedProps() {} };
  const top = { stateNode: {} }; top.stateNode.current = top;
  const fiber = { return: top, memoizedProps: props, dependencies: { firstContext: {
    memoizedValue: { store: shared }, next: { memoizedValue: store }
  } } };
  const node = { isConnected: true, __reactFiber$fixture: fiber };
  const page = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/' + (temporary ? '?temporary-chat=true' : '') },
    document: { querySelector: () => node, querySelectorAll: () => [] },
    __elonChatGptDocumentToken: 'doc_attachment_submit', __elonChatGptPrivateTextTransactionsEnabled: true,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: identity }) },
    __elonChatGptPrivateAttachmentProtocol: protocol,
    __elonChatGptComposer: { currentModel: () => 'synthetic-model' },
    performance: { getEntriesByName: () => [{}] }, Event: class { constructor(type) { this.type = type; } },
    setTimeout(fn) { const key = Symbol(); timers.set(key, fn); return key; }, clearTimeout: key => timers.delete(key),
    __elonChatGptPrivateStreamTransport: { prepareSend() { counts.prepared++; } },
    __elonChatGptPrivateTextTransactionRelay: { dispatch() { counts.relay++; return { dispatched: false }; } } };
  const composer = composerModule.create(page), binding = composer.capture();
  const file = new File(['synthetic upload'], image ? 'fixture.png' : 'fixture.txt', { type: image ? 'image/png' : 'text/plain' });
  const result = { ok: true, associated: false, binding,
    stage: reused ? 'reused' : 'processed', fileId: 'file-synthetic', fileName: file.name, fileSize: file.size, mimeType: file.type,
    isTemporaryChat: temporary, ...(image ? { imageDimensions: { width: 20, height: 30 } } : {}),
    ...(reused ? { reusedFileName: 'original.txt', metadata: { libraryFileId: 'library-synthetic', libraryPersistenceResult: 'library' } } : {}),
  };
  if (count === 1) composer.associate(binding, file, result, 'synthetic');
  else composer.associateMany(binding, Array.from({ length: count }, (_, i) => {
    const selected = new File(['synthetic file ' + i], 'fixture-' + i + '.txt', { type: 'text/plain' });
    return { file: selected, leaseId: 'synthetic-' + i, result: { ...result, fileId: 'file-' + i,
      fileName: selected.name, fileSize: selected.size, mimeType: selected.type } };
  }));
  const attached = values[0];
  page.__elonChatGptPrivateAttachmentSend = senderModule.create(page, { composer });
  page.__elonChatGptPrivateTextRuntimeSubmit = runtimeModule.create(page);
  vm.runInNewContext(orchestratorSource, { window: page });
  const send = page.__elonChatGptTextTransactionOrchestrator.create({
    findComposer: () => node, composerValue: () => draft,
    setComposerValue(_, value) { counts.draftWrites++; draft = value; return true; },
    comparableText: text => text, scheduleSnapshot() {},
    findButton: () => { throw Error('DOM path is not expected'); },
  });
  const command = { composer: node, prompt: 'synthetic attachment question', expectedDraft: '', requestId: 'mcp_attachment',
    requireNativeAttachment: true, readDraft: () => draft, clearDraft: () => { draft = ''; } };
  const respond = (action, ok, detail) => events.push({ action, ok, detail }); respond.requestId = command.requestId;
  return { page, store, props, node, fiber, top, file, attached, composer, calls, events, counts, command,
    api: page.__elonChatGptPrivateTextRuntimeSubmit,
    send: (allow = false) => send.sendPrompt(command.prompt, command.expectedDraft, respond, allow),
    settle: value => settle(value), response: value => { response = value; },
    timeout: () => { for (const fn of [...timers.values()]) fn(); },
    setIdentity: value => { identity = value; }, setDraft: value => { draft = value; }, draft: () => draft,
    navigate() { serverId = id; page.location.href = 'https://chatgpt.com/c/' + id + (temporary ? '?temporary-chat=true' : ''); } };
}

for (const count of [2, 9]) {
  test('one official prepared_action dispatches an entire owned attachment batch: ' + count, async () => {
    const f = fixture({ count }), selected = f.store.files$().slice();
    f.send(false);
    assert.equal(f.calls.length, 1);
    assert.equal(f.calls[0][1].kind, 'prepared_action');
    assert.equal(f.calls[0][1].readyFiles.length, count);
    assert.deepEqual(f.calls[0][1].readyFiles.map(item => item.file), selected.map(item => item.file));
    assert.equal(f.store.files$().length, count);
    f.navigate(); f.settle(true); await flush();
    assert.equal(f.events[0].detail, 'official_runtime_v1:accepted');
    assert.equal(f.store.files$().length, 0);
    assert.equal(f.calls.length, 1);
    assert.deepEqual(f.counts, { prepared: 1, click: 0, relay: 0, draftWrites: 0 });
  });
}

for (const config of [{}, { temporary: true }, { image: true }, { reused: true }]) {
  test('native attachment send uses official prepared_action without permitting text replay: ' + JSON.stringify(config), async () => {
    const f = fixture(config); f.send(false);
    assert.equal(f.calls.length, 1);
    assert.equal(f.calls[0][0].type, 'submit');
    assert.deepEqual(f.calls[0][1], { kind: 'prepared_action', text: f.command.prompt, readyFiles: [{ ...f.attached }] });
    assert.deepEqual(f.calls[0][2], { requireDispatchAcceptance: true });
    assert.equal(f.store.files$().length, 1);
    f.settle(true); await flush();
    assert.equal(f.events.length, 1);
    assert.equal(f.events[0].detail, 'official_runtime_v1:accepted');
    assert.equal(f.store.files$().length, 0);
    assert.deepEqual(f.counts, { prepared: 1, click: 0, relay: 0, draftWrites: 0 });
  });
}

test('attachments also remain attached when the caller allows private text optimization', async () => {
  const f = fixture(); f.send(true);
  assert.equal(f.calls[0][1].kind, 'prepared_action');
  assert.equal(f.counts.relay, 0);
  f.settle(true); await flush();
});

test('new thread navigation and concurrent new draft/files survive accepted dispatch cleanup', async () => {
  const f = fixture(); f.command.expectedDraft = f.command.prompt; f.setDraft(f.command.prompt);
  f.send();
  f.navigate(); f.composer.merge([]);
  const later = { status: 'ready', fileId: 'user-later' };
  f.store.files$.set([f.attached, later]); f.setDraft('later draft');
  f.settle(true); await flush();
  assert.equal(f.events[0].ok, true);
  assert.deepEqual(f.store.files$(), [later]);
  assert.equal(f.draft(), 'later draft');
});

for (const [name, change] of Object.entries({
  identity: f => f.setIdentity('Bearer another-account'),
  document: f => { f.page.__elonChatGptDocumentToken = 'doc_other'; },
  conversation: f => { f.props.conversation = { serverId$: () => null }; },
  detached: f => { f.node.isConnected = false; },
})) {
  test(name + ' replacement cannot consume a former context attachment', async () => {
    const f = fixture(); f.send(); change(f); f.settle(true); await flush();
    assert.equal(f.events[0].ok, false);
    assert.equal(f.events[0].detail, 'official_runtime_v1:unknown:context_changed');
    assert.equal(f.store.files$()[0], f.attached);
    assert.equal(f.counts.relay, 0);
  });
}

for (const [name, change] of Object.entries({
  empty: f => f.store.files$.set([]),
  unowned: f => f.store.files$.set([{ ...f.attached }]),
  additional: f => f.store.files$.set([f.attached, { status: 'ready' }]),
  model: f => { f.props.currentModelId = 'another'; },
  branch: f => { f.command.beforeSubmit = () => { f.props.currentLeafId = 'another'; }; },
  metadata: f => { f.command.beforeSubmit = () => { f.attached.fileSpec.size++; }; },
  library: f => { f.command.beforeSubmit = () => { f.props.isLibraryEnabled = false; }; },
})) {
  test(name + ' cannot be submitted through the native attachment contract', () => {
    const f = fixture(); change(f);
    assert.equal(f.api.submit(f.command).handled, false);
    assert.equal(f.calls.length, 0);
    assert.equal(f.counts.relay, 0);
  });
}

test('native attachment timeout blocks a second dispatch and never drops the pending file', async () => {
  const f = fixture(); f.send(); f.timeout(); await flush();
  assert.equal(f.events[0].detail, 'official_runtime_v1:unknown:timeout');
  assert.equal(f.store.files$()[0], f.attached);
  f.send(); await flush();
  assert.equal(f.events[1].detail, 'official_runtime_v1:unknown:busy');
  assert.equal(f.calls.length, 1);
  assert.equal(f.counts.relay, 0);
  f.settle(true); await flush();
  assert.equal(f.store.files$().length, 0);
});

test('a user removing the submitted file during dispatch is not a cleanup error', async () => {
  const f = fixture(); f.send();
  f.composer.remove(f.composer.merge([])[0].id);
  f.settle(true); await flush();
  assert.equal(f.events[0].detail, 'official_runtime_v1:accepted');
  assert.equal(f.api.state().pending, false);
  assert.equal(f.store.files$().length, 0);
});

test('failed ready-store cleanup cannot resubmit an already accepted attachment', async () => {
  const f = fixture(); f.send();
  f.store.files$.set = () => { throw Error('synthetic store failure'); };
  f.settle(true); await flush();
  assert.equal(f.events[0].detail, 'official_runtime_v1:unknown:attachment_cleanup_unconfirmed');
  assert.equal(f.api.state().pending, true);
  assert.equal(f.store.files$()[0], f.attached);
  f.send(); await flush();
  assert.equal(f.events[1].detail, 'official_runtime_v1:unknown:busy');
  assert.equal(f.calls.length, 1);
  assert.equal(f.counts.relay, 0);
});

for (const [name, response, status] of [
  ['rejected', () => ({ accepted: false }), 'rejected:not_ready'],
  ['thrown', () => { throw Error('synthetic'); }, 'unknown:invocation_failed'],
  ['malformed', () => ({ accepted: true }), 'unknown:invalid_receipt'],
  ['unconfirmed', () => ({ accepted: true, completion: Promise.resolve(false) }), 'unknown:dispatch_not_confirmed'],
]) {
  test(name + ' runtime receipt cannot fall through to a second writer', async () => {
    const f = fixture(); f.response(response); f.send(); await flush();
    assert.equal(f.events[0].detail, 'official_runtime_v1:' + status);
    assert.equal(f.store.files$()[0], f.attached);
    assert.equal(f.calls.length, 1);
    assert.equal(f.counts.relay, 0);
    assert.equal(f.counts.draftWrites, 0);
  });
}

test('non-attachment command with text optimization disabled still follows the existing DOM route', () => {
  const f = fixture(); f.store.files$.set([]);
  f.node.closest = () => ({ querySelector: () => null });
  assert.throws(() => f.send(false), /DOM path is not expected/);
  assert.equal(f.calls.length, 0);
  assert.equal(f.counts.relay, 0);
});

test('reinjection upgrades idle runtime but preserves an older in-flight writer', () => {
  const f = fixture(), old = { version: 1, state: () => ({ pending: true }) };
  f.page.__elonChatGptPrivateTextRuntimeSubmit = old;
  vm.runInNewContext(runtimeSource, { window: f.page });
  assert.equal(f.page.__elonChatGptPrivateTextRuntimeSubmit, old);
  old.state = () => ({ pending: false });
  vm.runInNewContext(runtimeSource, { window: f.page });
  assert.equal(f.page.__elonChatGptPrivateTextRuntimeSubmit.version, 5);
});

test('an older retained runtime cannot ignore the native-attachment-only restriction', async () => {
  for (const pending of [false, true]) {
    const f = fixture(), events = []; let calls = 0;
    f.store.files$.set([]);
    f.page.__elonChatGptPrivateTextRuntimeSubmit = { version: 1, state: () => ({ pending }),
      submit() { calls++; return { handled: true, completion: Promise.resolve({ status: 'unknown', code: 'busy' }) }; } };
    const api = f.page.__elonChatGptTextTransactionOrchestrator.create({
      findComposer: () => f.node, composerValue: () => '', comparableText: text => text,
      setComposerValue: () => { throw Error('compatibility'); }, scheduleSnapshot() {},
    });
    if (pending) {
      api.sendPrompt('synthetic', '', (...args) => events.push(args), false);
      await flush();
      assert.equal(events[0][2], 'official_runtime_v1:unknown:busy');
    } else assert.throws(() => api.sendPrompt('synthetic', '', () => {}, false), /compatibility/);
    assert.equal(calls, pending ? 1 : 0);
  }
});

test('native attachment reservation still prohibits template replay', () => {
  const owner = fs.readFileSync(path.join(__dirname,
    '../android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebSendOwner.kt'), 'utf8');
  const begin = owner.slice(owner.indexOf('fun beginAttachments('), owner.indexOf('fun consumeQueuedUploadUris('));
  assert.match(begin, /privateTextTransactionAllowed = false/);
});

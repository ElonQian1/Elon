'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const runtime = require('../android/app/src/main/assets/chatgpt_web_runtime_generation_state.js');

function fixture() {
  const node = { isConnected: true }, conversation = { id: 'local-test' }, controller = { conversation };
  const tree = { requestId: 'request-test' };
  const props = { conversation, composerController: controller, currentLeafId: 'assistant-test', currentRequestId: tree.requestId };
  const stream = { id: props.currentLeafId, conversationId: 'conversation-test', state: 'completed' };
  const shared = {
    XM: value => { assert.equal(value, conversation.id); return tree; },
    HM: { getRequestId: value => value.requestId }, Fl: () => false, Fx: () => null,
    v7: { STREAMING: 3, UNREAD: 4, REALTIME: 5, REALTIME_BUSY: 6, REALTIME_BACKGROUND: 7 }
  };
  const binding = { node, conversation, controller, requestId: tree.requestId,
    conversationId: stream.conversationId, token: 'doc_test', href: 'https://chatgpt.com/',
    shared: { getSharedProps: () => props } };
  const page = {
    location: { origin: 'https://chatgpt.com', href: binding.href },
    __elonChatGptDocumentToken: binding.token,
    __elonChatGptPrivateTextTransactionsEnabled: true,
    __elonChatGptPrivateRuntimeBindings: { peek: () => shared },
    __elonChatGptPrivateTextRuntimeSubmit: {
      state: () => ({ pending: false }),
      captureConversation: (value, guest) => { assert.equal(value, node); assert.equal(guest, true); return binding; }
    }
  };
  return { page, props, tree, shared, stream, binding, node, read: () => runtime.create(page).read(node, stream) };
}

test('completed mounted turn reads official state without network or mutation', () => {
  const f = fixture();
  const before = JSON.stringify([f.props, f.tree, f.stream]);
  f.page.fetch = () => { throw Error('no network'); };
  assert.deepEqual(f.read(), { active: false, code: 'completed_current_turn' });
  assert.equal(JSON.stringify([f.props, f.tree, f.stream]), before);
  f.shared.Fx = () => ({ value: 4 });
  assert.equal(f.read().active, false);
});

test('a settled tree without a request ID is allowed only for the matching leaf', () => {
  const f = fixture();
  f.binding.requestId = f.props.currentRequestId = f.tree.requestId = null;
  assert.equal(f.read().active, false);
  f.props.currentLeafId = 'next-assistant';
  assert.equal(f.read().active, null);
});

test('ongoing text or voice always retains active state', () => {
  for (const mode of [3, 5, 6, 7]) {
    const f = fixture(); f.shared.Fx = () => ({ value: mode });
    assert.equal(f.read().active, true);
  }
  const f = fixture(); f.shared.Fl = () => true;
  assert.equal(f.read().active, true);
});

test('unknown identity, request, enum and module states are not idle', () => {
  const changes = [
    f => { f.page.__elonChatGptPrivateTextTransactionsEnabled = false; },
    f => { f.stream.state = 'streaming'; },
    f => { f.stream.id = ''; },
    f => { f.page.__elonChatGptPrivateTextRuntimeSubmit.captureConversation = () => null; },
    f => { f.page.__elonChatGptPrivateRuntimeBindings.peek = () => null; },
    f => { f.shared.v7.UNREAD = 99; },
    f => { f.shared.Fl = () => undefined; },
    f => { f.shared.Fx = () => ({ value: 99 }); },
    f => { f.shared.Fx = () => ({}); },
    f => { f.shared.XM = () => null; },
    f => { f.shared.XM = () => { throw Error('not ready'); }; },
    f => { f.binding.requestId = {}; },
    f => { f.tree.requestId = 'different-request'; }
  ];
  for (const change of changes) { const f = fixture(); change(f); assert.equal(f.read().active, null); }
});

test('stale completion cannot settle a different conversation, branch or composer', () => {
  const changes = [
    f => { f.stream.conversationId = 'different-conversation'; },
    f => { f.props.currentLeafId = 'different-leaf'; },
    f => { f.props.currentRequestId = 'different-request'; },
    f => { f.props.conversation = {}; },
    f => { f.props.composerController = {}; }
  ];
  for (const change of changes) { const f = fixture(); change(f); assert.equal(f.read().active, null); }
});

test('context changes during synchronous observation invalidate the sample', () => {
  const changes = [
    f => { f.page.__elonChatGptDocumentToken = 'doc_other'; },
    f => { f.page.location.href += '?temporary-chat=true'; },
    f => { f.props.currentLeafId = 'other-leaf'; },
    f => { f.props.currentRequestId = 'other-request'; },
    f => { f.tree.requestId = 'other-request'; },
    f => { f.node.isConnected = false; },
    f => { f.props.conversation = {}; },
    f => { f.props.composerController = {}; },
    f => { f.page.__elonChatGptPrivateTextRuntimeSubmit.state = () => ({ pending: true }); }
  ];
  for (const change of changes) {
    const f = fixture(); f.shared.Fx = () => { change(f); return null; };
    assert.equal(f.read().active, null);
  }
});

test('unsettled local writers cannot be classified as idle', () => {
  for (const owner of ['TextRuntimeSubmit', 'RegenerateRuntime', 'StopRuntime']) {
    const f = fixture();
    f.page['__elonChatGptPrivate' + owner] = { state: () => ({ pending: true }) };
    assert.equal(f.read().code, 'writer_pending');
  }
  const f = fixture();
  f.page.__elonChatGptPrivateTextTransactionRelay = { state: () => ({ active: true }) };
  assert.equal(f.read().code, 'writer_pending');
});

test('module warming reuses existing bindings and never runs inside read', () => {
  const f = fixture(); let loads = 0;
  f.page.__elonChatGptPrivateRuntimeBindings = {
    observed: () => true, peek: () => null,
    load: key => { assert.equal(key, 'shared'); loads++; return Promise.reject(Error('offline')); }
  };
  const state = runtime.create(f.page);
  assert.equal(loads, 1);
  state.read(f.node, f.stream); state.read(f.node, f.stream);
  assert.equal(loads, 1);
});

test('registered after the existing runtime resolver and before the page consumer', () => {
  const catalog = fs.readFileSync(path.join(__dirname,
    '../android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebAdapterAssets.kt'), 'utf8');
  const index = catalog.indexOf('"chatgpt_web_runtime_generation_state.js"');
  assert.ok(index > catalog.indexOf('"chatgpt_web_private_text_runtime_submit.js"'));
  assert.ok(index < catalog.indexOf('"chatgpt_web_adapter.js"'));
});

'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const source = fs.readFileSync(path.join(__dirname,
  '../android/app/src/main/assets/chatgpt_web_adapter.js'), 'utf8');
const currentPath = '/c/11111111-2222-3333-4444-555555555555';
const otherPath = '/c/aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee';

function fixture(suffix = '') {
  const events = [], calls = [];
  let snapshot, composerVisible = true;
  class NodeElement {}
  class InputElement extends NodeElement {
    value = '';
    getBoundingClientRect() { return { width: composerVisible ? 100 : 0, height: 40 }; }
  }
  const composer = new InputElement();
  const location = new URL('https://chatgpt.com' + currentPath + suffix);
  const document = {
    title: 'Fixture - ChatGPT', documentElement: new NodeElement(),
    querySelector: () => null,
    querySelectorAll: selector => selector.includes('prompt-textarea') ? [composer] : []
  };
  const window = {
    location, document, __elonChatGptAdapterVersion: 376,
    __elonChatGptDocumentToken: 'doc_same_route_test',
    elonChatGptNative: { postMessage(payload) {
      const envelope = JSON.parse(payload);
      const event = envelope.event || envelope;
      events.push(event);
      calls.push(event.type);
    } },
    __elonChatGptLayout: { pageKind: () => 'conversation' },
    __elonChatGptConversations: {
      capabilities: () => [],
      openConversation(value, result) {
        calls.push('open:' + value);
        result('open_conversation', true, '');
      }
    },
    __elonChatGptPrivateTransport: {
      conversationPrefetchEnabled: true,
      prefetchConversation(value) { calls.push('prefetch:' + value); }
    },
    __elonChatGptSnapshotScheduler: { create(options) {
      snapshot = options.snapshot;
      return { schedule() {}, dispose() {} };
    } },
    getComputedStyle: () => ({ display: 'block', visibility: 'visible' }),
    addEventListener() {}, removeEventListener() {},
    setTimeout() { throw Error('Unexpected timer'); }
  };
  vm.runInNewContext(source, {
    window, document, location, Node: NodeElement,
    HTMLInputElement: InputElement, HTMLTextAreaElement: class extends InputElement {},
    MutationObserver: class { observe() {} disconnect() {} }
  });
  const snapshots = () => events.filter(event => event.type === 'message_snapshot');
  return { window, composer, location, events, calls, snapshots,
    snapshot: force => snapshot(force),
    visible: value => { composerVisible = value; },
    open(value = currentPath, documentToken = 'doc_same_route_test') {
      window.__elonChatGptBridge.command(JSON.stringify({ action: 'open_conversation',
        value, documentToken, requestId: 'mcp_fixtureopen' }));
    }
  };
}

test('opening the current route resends full ready state before prefetch and receipt', () => {
  const f = fixture();
  assert.equal(f.snapshots().length, 1);
  assert.equal(f.snapshots()[0].composerReady, true);
  f.calls.length = 0;
  f.open();
  assert.equal(f.snapshots().length, 2);
  assert.deepEqual(f.snapshots()[1], f.snapshots()[0]);
  assert.equal(f.snapshots()[1].contentOnly, undefined);
  assert.deepEqual(f.calls, ['message_snapshot', 'prefetch:' + currentPath,
    'open:' + currentPath, 'command_result']);
  f.snapshot();
  assert.equal(f.snapshots().length, 2, 'idle sampling still deduplicates');
});

test('each explicit reopen can settle a fresh native loading state without a DOM mutation', () => {
  const f = fixture();
  f.open(); f.open();
  assert.equal(f.snapshots().length, 3);
  assert.equal(f.calls.filter(value => value.startsWith('open:')).length, 2);
});

test('ordinary scheduling and non-boolean arguments do not bypass deduplication', () => {
  const f = fixture();
  for (const argument of [undefined, false, {}, [], 1, 'true']) f.snapshot(argument);
  assert.equal(f.snapshots().length, 1);
  f.composer.value = 'changed locally';
  f.snapshot();
  assert.equal(f.snapshots().length, 2);
  assert.equal(f.snapshots()[1].draft, 'changed locally');
});

test('missing composer is reported honestly then becomes ready on a real DOM change', () => {
  const f = fixture();
  f.visible(false); f.snapshot(); f.open();
  assert.equal(f.snapshots().length, 3);
  assert.equal(f.snapshots().at(-1).composerReady, false);
  f.visible(true); f.snapshot();
  assert.equal(f.snapshots().at(-1).composerReady, true);
});

for (const suffix of ['', '?temporary-chat=true', '#fragment']) {
  test('navigation does not force the old route snapshot: ' + (suffix || 'different conversation'), () => {
    const f = fixture(suffix);
    const target = suffix ? currentPath : otherPath;
    f.calls.length = 0; f.open(target);
    assert.equal(f.snapshots().length, 1);
    assert.equal(f.calls.includes('open:' + target), true);
  });
}

test('unsent drafts reject navigation without prefetch or forced snapshots', () => {
  const f = fixture();
  f.composer.value = 'do not discard';
  f.calls.length = 0; f.open();
  assert.equal(f.snapshots().length, 1);
  assert.equal(f.composer.value, 'do not discard');
  assert.equal(f.events.at(-1).ok, false);
  assert.deepEqual(f.calls, ['command_result']);
});

test('stale document commands and disposed bridges cannot emit ready state', () => {
  const f = fixture();
  f.open(currentPath, 'doc_stale_test');
  assert.equal(f.snapshots().length, 1);
  assert.equal(f.events.at(-1).ok, false);
  f.window.__elonChatGptBridge.dispose();
  const count = f.events.length;
  f.open(); f.snapshot(true);
  assert.equal(f.events.length, count);
});

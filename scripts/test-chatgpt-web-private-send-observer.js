'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const root = path.join(__dirname, '..');
const assetPath = path.join(
  root, 'android', 'app', 'src', 'main', 'assets', 'chatgpt_web_private_send_observer.js'
);
const observerSource = fs.readFileSync(assetPath, 'utf8');
const adapterSource = fs.readFileSync(path.join(
  root, 'android', 'app', 'src', 'main', 'assets', 'chatgpt_web_adapter.js'
), 'utf8');
const orchestratorSource = fs.readFileSync(path.join(
  root, 'android', 'app', 'src', 'main', 'assets',
  'chatgpt_web_text_transaction_orchestrator.js'
), 'utf8');
const pageAdapterSource = fs.readFileSync(path.join(
  root, 'android', 'app', 'src', 'main', 'kotlin', 'com', 'elon', 'app',
  'chatgptweb', 'ChatGptWebPageAdapter.kt'
), 'utf8');
const streamingPolicySource = fs.readFileSync(path.join(
  root, 'android', 'app', 'src', 'main', 'assets', 'chatgpt_web_adapter_streaming_policy.js'
), 'utf8');

function createContext(options = {}) {
  let calls = 0;
  const location = {
    origin: options.origin || 'https://chatgpt.com',
    href: (options.origin || 'https://chatgpt.com') + '/c/conversation-one',
    pathname: '/c/conversation-one'
  };
  const delegateFetch = options.delegateFetch || (() => {
    calls += 1;
    return Promise.resolve({ ok: true, status: 200 });
  });
  const window = {
    __elonChatGptPrivateStreamObserverEnabled: options.enabled !== false,
    fetch: function () {
      calls += 1;
      return delegateFetch.apply(this, arguments);
    }
  };
  window.window = window;
  vm.runInNewContext(observerSource, {
    window,
    location,
    URL,
    Date,
    Object,
    String,
    Number,
    RegExp
  }, { filename: 'chatgpt_web_private_send_observer.js' });
  return { window, location, calls: () => calls };
}

function createAdapterContext() {
  const events = [];
  class NodeElement {}
  class InputElement extends NodeElement {
    constructor(value = '') {
      super();
      this._value = value;
      this.disabled = false;
    }
    focus() {}
    closest() { return form; }
    getAttribute() { return null; }
    getBoundingClientRect() { return { width: 240, height: 48 }; }
    dispatchEvent(event) {
      if (event && event.type === 'input') sendButton.disabled = false;
    }
  }
  const composer = new InputElement('');
  const location = {
    origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/conversation-one',
    pathname: '/c/conversation-one'
  };
  let fetchCalls = 0;
  const sendButton = {
    disabled: true,
    getAttribute(name) { return name === 'aria-disabled' ? String(this.disabled) : null; },
    getBoundingClientRect() { return { width: 48, height: 48 }; },
    click() {
      window.fetch('/backend-api/conversation', { method: 'POST' });
    }
  };
  const form = {
    querySelector(selector) { return selector.includes('send-button') ? sendButton : null; },
    querySelectorAll() { return [sendButton]; }
  };
  const document = {
    title: 'ChatGPT',
    documentElement: new NodeElement(),
    querySelector() { return null; },
    querySelectorAll(selector) {
      if (selector.includes('prompt-textarea')) return [composer];
      if (selector === 'button') return [sendButton];
      return [];
    }
  };
  const window = {
    document,
    location,
    __elonChatGptPrivateStreamObserverEnabled: true,
    __elonChatGptAdapterVersion: 178,
    __elonChatGptDocumentToken: 'doc_private_send',
    elonChatGptNative: { postMessage: (payload) => events.push(JSON.parse(payload)) },
    __elonChatGptMessages: {
      capabilities() { return []; },
      lastAssistantObservation() {
        return { key: 'old', fingerprint: 'old', pending: false, completionVisible: true };
      },
      lastAssistantPending() { return false; },
      readMessageWindow() { return { observedCount: 0, startIndex: 0, messages: [] }; }
    },
    __elonChatGptSnapshotScheduler: {
      create() { return { schedule() {}, dispose() {} }; }
    },
    fetch() {
      fetchCalls += 1;
      return Promise.resolve({ ok: true, status: 200 });
    },
    getComputedStyle: () => ({ display: 'block', visibility: 'visible' }),
    setTimeout,
    clearTimeout,
    addEventListener() {},
    removeEventListener() {},
    HTMLInputElement: InputElement,
    HTMLTextAreaElement: class extends InputElement {},
    InputEvent: class { constructor(type) { this.type = type; } },
    Event: class { constructor(type) { this.type = type; } },
    MutationObserver: class { observe() {} disconnect() {} }
  };
  window.Node = NodeElement;
  window.window = window;
  Object.defineProperty(InputElement.prototype, 'value', {
    configurable: true,
    get() { return this._value || ''; },
    set(value) { this._value = String(value); }
  });
  const sandbox = {
    window,
    document,
    location,
    URL,
    Date,
    JSON,
    Object,
    String,
    Number,
    RegExp,
    HTMLInputElement: window.HTMLInputElement,
    HTMLTextAreaElement: window.HTMLTextAreaElement,
    InputEvent: window.InputEvent,
    Event: window.Event,
    MutationObserver: window.MutationObserver,
    Node: window.Node,
    setTimeout,
    clearTimeout
  };
  vm.runInNewContext(observerSource, sandbox, { filename: 'chatgpt_web_private_send_observer.js' });
  vm.runInNewContext(streamingPolicySource, sandbox, {
    filename: 'chatgpt_web_adapter_streaming_policy.js'
  });
  vm.runInNewContext(orchestratorSource, sandbox, {
    filename: 'chatgpt_web_text_transaction_orchestrator.js'
  });
  vm.runInNewContext(adapterSource, sandbox, { filename: 'chatgpt_web_adapter.js' });
  return { window, composer, events, fetchCalls: () => fetchCalls };
}

assert.ok(
  pageAdapterSource.indexOf('chatgpt_web_private_stream_transport.js') <
  pageAdapterSource.indexOf('chatgpt_web_private_send_observer.js')
);
assert.ok(
  pageAdapterSource.indexOf('chatgpt_web_private_send_observer.js') <
  pageAdapterSource.indexOf('chatgpt_web_text_transaction_orchestrator.js')
);
assert.match(orchestratorSource, /privateSendObserver\.marker\(\)/);
assert.match(orchestratorSource, /privateSendObserver\.dispatchedAfter\(sendMarker\)/);
assert.match(orchestratorSource, /official_request_dispatched/);
assert.match(orchestratorSource, /official_page_accepted/);
assert.match(
  orchestratorSource,
  /privateStreamTransport\.prepareSend\(\);[\s\S]*?button\.click\(\);/,
  'the adapter must clear stale private completion state before the official send click'
);

(async () => {
  const enabled = createContext();
  const observer = enabled.window.__elonChatGptPrivateSendObserver;
  assert.equal(observer.version, 2);

  const first = observer.marker();
  await enabled.window.fetch('/backend-api/conversations/conversation-one', { method: 'GET' });
  assert.equal(observer.dispatchedAfter(first), false, 'conversation reads are not sends');

  const prepareMarker = observer.marker();
  await enabled.window.fetch('/backend-api/f/conversation/prepare', { method: 'POST' });
  assert.equal(observer.dispatchedAfter(prepareMarker), false, 'prepare requests are not sends');

  const privateMarker = observer.marker();
  await enabled.window.fetch('/backend-api/conversation', {
    method: 'POST',
    __elonPrivateTransport: 'test-only',
    headers: { Authorization: 'must-not-be-read' },
    body: 'private prompt must not be read'
  });
  assert.equal(observer.dispatchedAfter(privateMarker), false, 'private transport cannot self-confirm');

  const crossOrigin = observer.marker();
  await enabled.window.fetch('https://example.com/backend-api/conversation', { method: 'POST' });
  assert.equal(observer.dispatchedAfter(crossOrigin), false, 'cross-origin posts are ignored');

  const official = observer.marker();
  const init = { method: 'POST' };
  Object.defineProperty(init, 'headers', {
    get: () => { throw new Error('headers must never be read'); }
  });
  Object.defineProperty(init, 'body', {
    get: () => { throw new Error('body must never be read'); }
  });
  await enabled.window.fetch('/backend-api/f/conversation', init);
  assert.equal(observer.dispatchedAfter(official), true, 'official same-origin sends are observed');
  assert.equal(
    JSON.stringify(observer).includes('prompt'),
    false,
    'observer API exposes no request content'
  );

  const priorConversation = observer.marker();
  enabled.location.pathname = '/c/conversation-two';
  await enabled.window.fetch('/backend-anon/conversation', { method: 'POST' });
  assert.equal(
    observer.dispatchedAfter(priorConversation),
    false,
    'a send from another visible conversation cannot confirm the prior command'
  );

  const disabled = createContext({ enabled: false });
  assert.equal(disabled.window.__elonChatGptPrivateSendObserver, undefined);

  let failedCalls = 0;
  const failed = createContext({
    delegateFetch: () => {
      failedCalls += 1;
      throw new Error('synchronous failure');
    }
  });
  const failedMarker = failed.window.__elonChatGptPrivateSendObserver.marker();
  assert.throws(
    () => failed.window.fetch('/backend-api/conversation', { method: 'POST' }),
    /synchronous failure/
  );
  assert.equal(failedCalls, 1);
  assert.equal(
    failed.window.__elonChatGptPrivateSendObserver.dispatchedAfter(failedMarker),
    false,
    'a synchronous fetch failure is not reported as dispatched'
  );

  assert.ok(enabled.calls() >= 4, 'the observer never suppresses official fetch calls');

  const adapter = createAdapterContext();
  adapter.window.__elonChatGptBridge.command(JSON.stringify({
    action: 'send_prompt',
    documentToken: 'doc_private_send',
    requestId: 'mcp_private_send',
    value: 'synthetic prompt',
    expectedDraft: ''
  }));
  await new Promise((resolve) => setTimeout(resolve, 420));
  const receipt = adapter.events.find((event) => event.action === 'send_prompt');
  assert.equal(adapter.fetchCalls(), 1, 'the official button remains the only request owner');
  assert.equal(receipt && receipt.ok, true);
  assert.equal(receipt && receipt.detail, '官网发送请求已提交。');
  assert.equal(
    adapter.composer.value,
    'synthetic prompt',
    'private dispatch evidence confirms send even before the official DOM clears'
  );
  console.log('CHATGPT_WEB_PRIVATE_SEND_OBSERVER_TESTS=passed');
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

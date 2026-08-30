'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const source = fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'assets', 'chatgpt_web_adapter.js'
), 'utf8');
const orchestratorSource = fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'assets',
  'chatgpt_web_text_transaction_orchestrator.js'
), 'utf8');
const streamingPolicySource = fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'assets', 'chatgpt_web_adapter_streaming_policy.js'
), 'utf8');
const events = [];

class NodeElement {}

class InputElement extends NodeElement {
  constructor(value = '') {
    super();
    this._value = String(value);
    this.disabled = false;
  }
  focus() {}
  closest() { return form; }
  getAttribute() { return null; }
  getBoundingClientRect() { return { width: 240, height: 48 }; }
  dispatchEvent(event) {
    if (event && event.type === 'input') {
      setTimeout(() => { sendButton.disabled = false; }, 20);
    }
  }
}

const composer = new InputElement('');
let clickedAt = 0;
let assistantObservation = {
  key: 'assistant-old', fingerprint: 'old answer', pending: false, completionVisible: true
};
const sendButton = {
  disabled: true,
  getAttribute(name) { return name === 'aria-disabled' ? String(this.disabled) : null; },
  getBoundingClientRect() { return { width: 48, height: 48 }; },
  click() {
    clickedAt = Date.now();
    composer.value = '';
    assistantObservation = {
      key: 'assistant-new', fingerprint: 'first streamed token', pending: false, completionVisible: false
    };
  }
};
const form = {
  querySelector(selector) {
    return selector.includes('send-button') ? sendButton : null;
  },
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
  location: { origin: 'https://chatgpt.com', pathname: '/' },
  elonChatGptNative: { postMessage: (payload) => events.push(JSON.parse(payload)) },
  __elonChatGptAdapterVersion: 118,
  __elonChatGptDocumentToken: 'doc_send_settle',
  __elonChatGptMessages: {
    capabilities() { return []; },
    lastAssistantObservation() { return assistantObservation; },
    lastAssistantPending() { return assistantObservation.pending; },
    readMessageWindow(streaming, assistantKey) {
      return {
        observedCount: 1,
        startIndex: 0,
        messages: [{
          id: assistantObservation.key,
          role: 'assistant',
          state: streaming && assistantKey === assistantObservation.key ? 'streaming' : 'completed',
          content: [{ type: 'markdown', text: assistantObservation.fingerprint }]
        }]
      };
    }
  },
  __elonChatGptSnapshotScheduler: {
    create() { return { schedule() {}, dispose() {} }; }
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
Object.defineProperty(InputElement.prototype, 'value', {
  configurable: true,
  get() { return this._value || ''; },
  set(value) { this._value = String(value); }
});
window.window = window;

const sandbox = {
  window,
  document,
  location: window.location,
  HTMLInputElement: window.HTMLInputElement,
  HTMLTextAreaElement: window.HTMLTextAreaElement,
  InputEvent: window.InputEvent,
  Event: window.Event,
  MutationObserver: window.MutationObserver,
  Node: window.Node,
  setTimeout,
  clearTimeout
};
vm.runInNewContext(streamingPolicySource, sandbox, {
  filename: 'chatgpt_web_adapter_streaming_policy.js'
});
vm.runInNewContext(orchestratorSource, sandbox, {
  filename: 'chatgpt_web_text_transaction_orchestrator.js'
});
vm.runInNewContext(source, sandbox, { filename: 'chatgpt_web_adapter.js' });

const startedAt = Date.now();
window.__elonChatGptBridge.command(JSON.stringify({
  action: 'send_prompt',
  documentToken: 'doc_send_settle',
  requestId: 'mcp_send',
  value: 'production timing probe',
  expectedDraft: ''
}));

setTimeout(() => {
  const result = events.find((event) => event.action === 'send_prompt');
  assert.ok(result, 'send command must produce a receipt');
  assert.equal(result.ok, true);
  assert.equal(result.requestId, 'mcp_send');
  assert.equal(composer.value, '');
  assert.ok(clickedAt - startedAt >= 180, 'send must wait for a stable enabled button');
  window.__elonChatGptBridge.command(JSON.stringify({
    action: 'snapshot',
    documentToken: 'doc_send_settle'
  }));
  const streamingSnapshot = events.filter((event) => event.event?.type === 'message_snapshot').at(-1);
  assert.equal(streamingSnapshot.event.streaming, true);
  assert.equal(streamingSnapshot.event.messages[0].state, 'streaming');
  assert.equal(streamingSnapshot.event.messages[0].content[0].text, 'first streamed token');
  console.log('CHATGPT_SEND_SETTLE_POLICY=passed');
}, 650);

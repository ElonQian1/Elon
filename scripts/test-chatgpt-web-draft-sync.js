'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const source = fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'assets', 'chatgpt_web_adapter.js'
), 'utf8');
const events = [];

class InputElement {
  constructor(value = '') {
    this._value = String(value);
    this.disabled = false;
  }
  focus() {}
  closest() { return null; }
  getAttribute() { return null; }
  getBoundingClientRect() { return { width: 100, height: 40 }; }
  dispatchEvent() {}
}
const composer = new InputElement('old draft');
const document = {
  title: 'ChatGPT',
  documentElement: {},
  querySelector: () => null,
  querySelectorAll(selector) {
    return selector.includes('prompt-textarea') ? [composer] : [];
  }
};
const window = {
  document,
  location: { origin: 'https://chatgpt.com', pathname: '/c/test' },
  elonChatGptNative: { postMessage: (payload) => events.push(JSON.parse(payload)) },
  __elonChatGptAdapterVersion: 70,
  __elonChatGptDocumentToken: 'doc_test_1',
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
  InputEvent: class {},
  Event: class {},
  MutationObserver: class { observe() {} disconnect() {} }
};
Object.defineProperty(InputElement.prototype, 'value', {
  configurable: true,
  get() { return this._value || ''; },
  set(value) { this._value = String(value); }
});
window.window = window;

vm.runInNewContext(source, {
  window,
  document,
  location: window.location,
  HTMLInputElement: window.HTMLInputElement,
  HTMLTextAreaElement: window.HTMLTextAreaElement,
  InputEvent: window.InputEvent,
  Event: window.Event,
  MutationObserver: window.MutationObserver,
  setTimeout,
  clearTimeout
}, { filename: 'chatgpt_web_adapter.js' });

window.__elonChatGptBridge.command(JSON.stringify({
  action: 'set_draft',
  documentToken: 'doc_test_1',
  requestId: 'mcp_a',
  value: 'next draft',
  expectedDraft: 'old draft'
}));
assert.equal(composer.value, 'next draft');
assert.equal(events.at(-1).action, 'set_draft');
assert.equal(events.at(-1).ok, true);
assert.equal(events.at(-1).requestId, 'mcp_a');

window.__elonChatGptBridge.command(JSON.stringify({
  action: 'set_draft',
  documentToken: 'doc_test_1',
  requestId: 'mcp_b',
  value: 'must not overwrite',
  expectedDraft: 'stale draft'
}));
assert.equal(composer.value, 'next draft');
assert.equal(events.at(-1).action, 'set_draft');
assert.equal(events.at(-1).ok, false);
assert.equal(events.at(-1).requestId, 'mcp_b');

console.log('CHATGPT_DRAFT_SYNC_POLICY=passed');

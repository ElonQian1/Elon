'use strict';

const { test } = require('node:test');
const assert = require('node:assert/strict');
const vm = require('node:vm');
const fs = require('node:fs');
const path = require('node:path');
const { selectors, pageOperation, endpointUrl, socketUrl } = require('./chatgpt-composer-dom-lease.cjs');
const nonce = 'a'.repeat(32);

function fixture() {
  const styles = [], timers = new Map();
  let nextTimer = 1;
  const editor = { getBoundingClientRect: () => ({ width: 200, height: 50 }) };
  const document = {
    head: { appendChild: style => { style.isConnected = true; styles.push(style); } },
    createElement: name => {
      assert.equal(name, 'style');
      return { isConnected: false, textContent: '', remove() { this.isConnected = false; } };
    },
    querySelectorAll: query => { assert.equal(query, selectors.join(',')); return [editor]; }
  };
  const window = { __elonChatGptPrivateResearchEnabled: true, __elonChatGptBridge: { version: 410 },
    __elonChatGptDocumentToken: 'synthetic-document',
    getComputedStyle: () => ({ display: styles.some(style => style.isConnected) ? 'none' : 'block', visibility: 'visible' }) };
  const timer = (callback, ms) => { const id = nextTimer++; timers.set(id, { callback, ms }); return id; };
  const context = vm.createContext({ window, document, location: new URL('https://chatgpt.com/'),
    setTimeout: timer, setInterval: timer, clearTimeout: id => timers.delete(id), clearInterval: id => timers.delete(id) });
  const call = (action, owner = nonce) => vm.runInContext(`(${pageOperation.toString()})` +
    `(${JSON.stringify(action)},${JSON.stringify(owner)},410,${JSON.stringify(selectors)})`, context);
  return { context, window, styles, timers, editor, call };
}

test('CSS-only lease prevents usable DOM, survives SPA route and restores without changing editor', () => {
  const f = fixture();
  assert.equal(f.call('hide').visible_now, 0);
  assert.equal(f.styles.length, 1);
  f.context.location = new URL('https://chatgpt.com/c/00000000-0000-4000-8000-000000000000');
  const state = f.call('state');
  assert.equal(state.active, true);
  assert.equal(state.invalid_samples, 0);
  assert.equal(state.visible_samples, 0);
  assert.equal(f.call('restore').restored, true);
  assert.equal(f.styles[0].isConnected, false);
  assert.equal(f.timers.size, 0);
  assert.equal(f.window.getComputedStyle(f.editor).display, 'block');
  assert.equal(f.call('restore').existed, false);
});

test('lease expiry restores even if runner disconnects', () => {
  const f = fixture(); f.call('hide');
  [...f.timers.values()].find(timer => timer.ms === 240000).callback();
  assert.equal(f.styles[0].isConnected, false);
  assert.equal(f.timers.size, 0);
  assert.throws(() => f.call('state'), /composer_lease_missing/);
});

test('foreign owner cannot replace or restore a lease', () => {
  const f = fixture(); f.call('hide');
  assert.throws(() => f.call('hide'), /already_present/);
  assert.throws(() => f.call('restore', 'b'.repeat(32)), /wrong_owner/);
  assert.equal(f.styles[0].isConnected, true);
});

for (const [name, change] of [
  ['normal build', f => { f.window.__elonChatGptPrivateResearchEnabled = false; }],
  ['wrong adapter', f => { f.window.__elonChatGptBridge.version = 409; }],
  ['missing document', f => { delete f.window.__elonChatGptDocumentToken; }],
  ['wrong origin', f => { f.context.location = new URL('https://example.com/'); }],
  ['existing chat', f => { f.context.location = new URL('https://chatgpt.com/c/fixture'); }],
  ['temporary', f => { f.context.location = new URL('https://chatgpt.com/?temporary-chat=true'); }]
]) test(`hide refuses ${name}`, () => {
  const f = fixture(); change(f);
  assert.throws(() => f.call('hide'), /composer_lease_/);
  assert.equal(f.styles.length, 0);
});

test('document replacement invalidates evidence but owned CSS remains recoverable', () => {
  const f = fixture(); f.call('hide'); f.window.__elonChatGptDocumentToken = 'replacement';
  const state = f.call('state');
  assert.equal(state.active, false); assert.equal(state.invalid_samples, 1);
  assert.equal(f.call('restore').restored, true);
});

test('style loss cannot count as a successful DOM-unavailable sample', () => {
  const f = fixture(); f.call('hide'); f.styles[0].remove();
  const state = f.call('state');
  assert.equal(state.active, false); assert.equal(state.visible_samples, 1);
});

test('network target can only be the exact local forwarded page socket', () => {
  const endpoint = endpointUrl('http://127.0.0.1:9222');
  assert.equal(socketUrl('ws://localhost:9222/devtools/page/fixture-1', endpoint),
    'ws://127.0.0.1:9222/devtools/page/fixture-1');
  for (const invalid of ['http://example.com:9222', 'http://u:p@127.0.0.1:9222',
    'http://127.0.0.1:9222/path', 'http://127.0.0.1:9222?query']) assert.throws(() => endpointUrl(invalid));
  for (const invalid of ['ws://example.com:9222/devtools/page/1', 'ws://localhost:9223/devtools/page/1',
    'ws://localhost:9222/devtools/browser/1', 'ws://u:p@localhost:9222/devtools/page/1']) {
    assert.throws(() => socketUrl(invalid, endpoint));
  }
});

test('selectors cover actual adapter locator, not a synthetic replacement', () => {
  const source = fs.readFileSync(path.join(__dirname, '../android/app/src/main/assets/chatgpt_web_adapter.js'), 'utf8');
  const block = source.slice(source.indexOf('function findComposer()'), source.indexOf('function findComposer()') + 700);
  for (const selector of selectors) assert.ok(block.includes(`'${selector}'`), selector);
});

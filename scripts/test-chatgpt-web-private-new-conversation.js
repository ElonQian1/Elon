'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const navigation = require('../android/app/src/main/assets/chatgpt_web_private_new_conversation');
const { attach } = require('./fixtures/chatgpt-runtime-bindings');
const assets = path.join(__dirname, '../android/app/src/main/assets');
const SAVED = 'https://chatgpt.com/c/12345678-1234-1234-1234-123456789012';

function fixture() {
  let time = 0, messageCount = 2, ready = true, modal = false;
  const tasks = new Map(), calls = [], results = [];
  let serial = 0, fallbacks = 0;
  const page = {
    location: new URL(SAVED), __elonChatGptDocumentToken: 'doc_navigation',
    document: { querySelector: selector => modal && selector.includes('modal-no-auth-new-chat') ? {} : null },
    KeyboardEvent: class { constructor(type, properties) { this.type = type; Object.assign(this, properties); } },
    setTimeout: (fn, delay) => { const id = ++serial; tasks.set(id, { fn, at: time + delay }); return id; },
    clearTimeout: id => tasks.delete(id)
  };
  const config = { id: 'newChat', isAvailable: true, disabled: false, scope: 'global',
    label: { id: 'keyboardActions.newChat' } };
  const shared = { Ur: () => config, zr: (key, event) => { calls.push({ key, event }); return true; } };
  page.__elonChatGptPrivateRuntimeBindings = { observed: () => true, peek: () => shared,
    load: async () => shared };
  const api = navigation.create(page, { now: () => time });
  const inspect = () => ({ messageCount, composerReady: ready });
  const result = (...args) => results.push(args);
  const fallback = () => { fallbacks++; };
  return { page, shared, config, calls, results, api, inspect, result, fallback,
    fallbackCount: () => fallbacks, start: () => api.start(inspect, result, fallback),
    messages: value => { messageCount = value; }, ready: value => { ready = value; },
    modal: value => { modal = value; },
    tick() {
      const next = [...tasks].sort((a, b) => a[1].at - b[1].at)[0];
      if (!next) return;
      tasks.delete(next[0]); time = next[1].at; next[1].fn();
    },
    drain() { let limit = 200; while (tasks.size && limit-- > 0) this.tick(); assert.ok(limit > 0); }
  };
}

test('registered newChat runs once without a visible button and waits for stable blank content', () => {
  const f = fixture();
  assert.equal(f.start(), true);
  assert.equal(f.calls.length, 1); assert.equal(f.calls[0].key, 'newChat');
  assert.equal(f.calls[0].event.type, 'keydown');
  assert.deepEqual(f.results, []);
  f.page.location = new URL('https://chatgpt.com/');
  f.messages(0); f.tick(); assert.deepEqual(f.results, []); f.drain();
  assert.equal(f.results.length, 1); assert.equal(f.results[0][1], true);
  assert.match(f.results[0][2], /runtime_new_chat:ready/);
  assert.equal(f.fallbackCount(), 0); assert.equal(f.api.state().pending, false);
});

test('guest root URL is not mistaken for an already empty conversation', () => {
  const f = fixture(); f.page.location = new URL('https://chatgpt.com/');
  f.start(); f.tick(); assert.deepEqual(f.results, []);
  f.messages(0); f.drain(); assert.equal(f.results[0][1], true);
});

test('guest discard confirmation is preserved and never auto-clicked', () => {
  const f = fixture(); f.page.location = new URL('https://chatgpt.com/');
  f.shared.zr = () => { f.modal(true); return true; };
  f.start(); assert.match(f.results[0][2], /confirmation_required/);
  assert.equal(f.results[0][1], false); assert.equal(f.fallbackCount(), 0);
});

for (const change of ['missing', 'disabled', 'wrong_label', 'wrong_scope', 'unknown_export']) {
  test('pre-invocation ' + change + ' uses the existing route, not a fake capability result', () => {
    const f = fixture();
    if (change === 'missing') f.config.isAvailable = false;
    if (change === 'disabled') f.config.disabled = true;
    if (change === 'wrong_label') f.config.label.id = 'other';
    if (change === 'wrong_scope') f.config.scope = 'composer';
    if (change === 'unknown_export') delete f.shared.zr;
    f.start(); assert.equal(f.calls.length, 0); assert.equal(f.fallbackCount(), 1);
    assert.deepEqual(f.results, []); assert.equal(f.api.state().pending, false);
  });
}

for (const outcome of ['throw', 'false', 'undefined', 'timeout']) {
  test('post-invocation ' + outcome + ' is never replayed through DOM', () => {
    const f = fixture();
    f.shared.zr = key => { f.calls.push(key); if (outcome === 'throw') throw Error('synthetic');
      return outcome === 'false' ? false : outcome === 'undefined' ? undefined : true; };
    f.start(); f.drain(); assert.equal(f.calls.length, 1); assert.equal(f.results[0][1], false);
    assert.equal(f.fallbackCount(), 0); assert.equal(f.api.state().pending, false);
  });
}

test('duplicate requests do not dispatch a second registered action', () => {
  const f = fixture(); f.start(); f.start();
  assert.equal(f.calls.length, 1); assert.match(f.results[0][2], /:busy/);
  f.page.location = new URL('https://chatgpt.com/');
  f.messages(0); f.drain(); assert.equal(f.results[1][1], true);
});

test('a second request cannot fall through while the first hides the composer', () => {
  const f = fixture(); f.start(); f.ready(false);
  assert.equal(f.start(), true); assert.equal(f.calls.length, 1);
  assert.match(f.results[0][2], /:busy/); assert.equal(f.fallbackCount(), 0);
  f.drain(); assert.equal(f.api.state().pending, false);
});

test('empty content at the old saved route is not a completed new chat', () => {
  const f = fixture(); f.start(); f.messages(0); f.drain();
  assert.equal(f.results[0][1], false); assert.match(f.results[0][2], /:timeout/);
});

test('a foreign conversation mode during settlement stops observation', () => {
  const f = fixture(); f.start();
  f.page.location = new URL('https://chatgpt.com/?temporary-chat=true'); f.tick();
  assert.match(f.results[0]?.[2] || '', /:context_changed/);
  assert.equal(f.api.state().pending, false); assert.equal(f.fallbackCount(), 0);
});

test('an unexpected synchronous runtime probe failure leaves the existing route usable', () => {
  const f = fixture(); f.page.__elonChatGptPrivateRuntimeBindings.peek = () => { throw Error('synthetic'); };
  assert.equal(f.start(), false); assert.equal(f.api.state().pending, false);
  assert.equal(f.calls.length, 0);
});

for (const change of ['document', 'token', 'route', 'messages']) {
  test('pending import cannot act after a changed ' + change, async () => {
    const f = fixture(); let release;
    f.page.__elonChatGptPrivateRuntimeBindings.peek = () => null;
    f.page.__elonChatGptPrivateRuntimeBindings.load = () => new Promise(resolve => { release = resolve; });
    f.start(); await Promise.resolve();
    if (change === 'document') f.page.document = { querySelector: () => null };
    if (change === 'token') f.page.__elonChatGptDocumentToken = 'doc_other';
    if (change === 'route') f.page.location = new URL('https://chatgpt.com/');
    if (change === 'messages') f.messages(3);
    release(f.shared); await Promise.resolve(); await Promise.resolve(); await Promise.resolve();
    assert.equal(f.calls.length, 0); assert.equal(f.fallbackCount(), 0);
    assert.match(f.results[0][2], /context_changed/);
  });
}

test('failed bounded import falls back once before invocation', async () => {
  const f = fixture(); f.page.__elonChatGptPrivateRuntimeBindings.peek = () => null;
  f.page.__elonChatGptPrivateRuntimeBindings.load = async () => { throw Error('load_timeout'); };
  f.start(); await Promise.resolve(); await Promise.resolve(); await Promise.resolve();
  await Promise.resolve(); assert.equal(f.fallbackCount(), 1); assert.equal(f.calls.length, 0);
});

test('current versioned namespace drives the real navigation consumer', async () => {
  const f = fixture(); attach(f.page, { shared: f.shared });
  f.start(); await new Promise(resolve => setImmediate(resolve));
  f.page.location = new URL('https://chatgpt.com/');
  assert.equal(f.calls.length, 1); f.messages(0); f.drain(); assert.equal(f.results[0][1], true);
});

for (const url of ['https://example.invalid/', SAVED + '?temporary-chat=true',
  'https://chatgpt.com/g/g-p-project/project', SAVED + '#anchor']) {
  test('unaccepted route remains on the existing provider path: ' + url, () => {
    const f = fixture(); f.page.location = new URL(url);
    assert.equal(f.start(), false); assert.equal(f.calls.length, 0); assert.equal(f.fallbackCount(), 0);
  });
}

test('unready composer and unknown runtime do not claim ownership', () => {
  const f = fixture(); f.ready(false); assert.equal(f.start(), false);
  f.ready(true); f.page.__elonChatGptPrivateRuntimeBindings.observed = () => false;
  assert.equal(f.start(), false); assert.equal(f.calls.length, 0);
});

test('production conversation entry delegates once before scanning for a DOM button', () => {
  let invoked = 0, scanned = 0;
  const window = { __elonChatGptPrivateNewConversation: { start: () => { invoked++; return true; } } };
  vm.runInNewContext(fs.readFileSync(path.join(assets, 'chatgpt_web_adapter_conversations.js'), 'utf8'), {
    window, location: { origin: 'https://chatgpt.com' }, document: {
      querySelector: () => { scanned++; return null; }, querySelectorAll: () => { scanned++; return []; }
    }
  });
  window.__elonChatGptConversations.newConversation(() => ({ messageCount: 2, composerReady: true }), () => {});
  assert.equal(invoked, 1); assert.equal(scanned, 0);
});

test('production asset catalog loads navigation before its conversation consumer', () => {
  const catalog = fs.readFileSync(path.join(assets, '../kotlin/com/elon/app/chatgptweb/ChatGptWebAdapterAssets.kt'), 'utf8');
  const navigationAt = catalog.indexOf('"chatgpt_web_private_new_conversation.js"');
  assert.ok(navigationAt > catalog.indexOf('"chatgpt_web_private_runtime_bindings.js"'));
  assert.ok(navigationAt < catalog.indexOf('"chatgpt_web_adapter_conversations.js"'));
});

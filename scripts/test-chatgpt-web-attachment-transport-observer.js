'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const root = path.join(__dirname, '..');
const assetPath = path.join(
  root, 'android', 'app', 'src', 'main', 'assets',
  'chatgpt_web_attachment_transport_observer.js'
);
const observerSource = fs.readFileSync(assetPath, 'utf8');
const adapterSource = fs.readFileSync(path.join(
  root, 'android', 'app', 'src', 'main', 'assets', 'chatgpt_web_adapter.js'
), 'utf8');
const pageAdapterSource = fs.readFileSync(path.join(
  root, 'android', 'app', 'src', 'main', 'kotlin', 'com', 'elon', 'app',
  'chatgptweb', 'ChatGptWebPageAdapter.kt'
), 'utf8');

function createContext(options = {}) {
  const events = [];
  const timers = [];
  let nextStatus = 200;
  const location = {
    origin: 'https://chatgpt.com',
    href: 'https://chatgpt.com/c/synthetic',
    pathname: '/c/synthetic'
  };

  class FakeXhr {
    constructor() {
      this.listeners = new Map();
      this.status = 0;
    }
    open() {}
    send() {}
    addEventListener(type, listener) { this.listeners.set(type, listener); }
    finish(status) {
      this.status = status;
      const listener = this.listeners.get('loadend');
      if (listener) listener.call(this);
    }
  }

  const window = {
    fetch: () => Promise.resolve({ status: nextStatus }),
    XMLHttpRequest: FakeXhr,
    setTimeout: (callback) => {
      timers.push(callback);
      return timers.length;
    }
  };
  if (options.early !== true) {
    window.__elonChatGptAdapterTargetVersion = 207;
    window.__elonChatGptDocumentToken = 'doc_attachment_test';
    window.elonChatGptNative = { postMessage: (payload) => events.push(JSON.parse(payload)) };
  }
  window.window = window;
  if (options.existingObserver) {
    window.__elonChatGptAttachmentTransportObserver = options.existingObserver;
  }
  const globals = {
    window,
    location,
    URL,
    Date,
    JSON,
    Math,
    Number,
    Object,
    Promise,
    RegExp,
    Set,
    String,
    WeakMap
  };
  const inject = () => vm.runInNewContext(observerSource, globals, {
    filename: 'chatgpt_web_attachment_transport_observer.js'
  });
  inject();
  return {
    window,
    events,
    inject,
    timerCount: () => timers.length,
    setStatus: (status) => { nextStatus = status; },
    flushTimers: () => {
      while (timers.length) timers.shift()();
    },
    newXhr: () => new FakeXhr()
  };
}

function transportEvents(context) {
  return context.events.map((item) => item.event).filter((item) =>
    item && item.type === 'attachment_transport'
  );
}

assert.ok(
  pageAdapterSource.indexOf('chatgpt_web_attachment_transport_observer.js') <
  pageAdapterSource.indexOf('chatgpt_web_adapter.js'),
  'the observer must be installed before the command adapter'
);
assert.match(adapterSource, /attachmentTransportObserver\.arm\(\)/);

(async () => {
  const context = createContext();
  const observer = context.window.__elonChatGptAttachmentTransportObserver;

  const early = createContext({ early: true });
  const earlyObserver = early.window.__elonChatGptAttachmentTransportObserver;
  early.window.__elonChatGptAdapterTargetVersion = 207;
  early.window.__elonChatGptDocumentToken = 'doc_attachment_early';
  early.window.elonChatGptNative = {
    postMessage: (payload) => early.events.push(JSON.parse(payload))
  };
  earlyObserver.arm();
  assert.equal(transportEvents(early).at(-1).state, 'armed');
  await early.window.fetch('/backend-api/files/early-file', { method: 'POST' });
  early.flushTimers();
  assert.equal(transportEvents(early).at(-1).state, 'started');
  assert.equal(transportEvents(early).at(-1).completedCount, 0);

  await context.window.fetch('/backend-api/files/unarmed', { method: 'POST' });
  context.flushTimers();
  assert.equal(transportEvents(context).length, 0, 'normal page traffic is ignored until armed');

  observer.arm();
  assert.equal(transportEvents(context).at(-1).state, 'armed');
  await context.window.fetch('https://example.com/backend-api/files/cross-origin', { method: 'POST' });
  await context.window.fetch('/backend-api/files/read-only', { method: 'GET' });
  assert.equal(transportEvents(context).length, 1, 'cross-origin and non-POST traffic is ignored');

  await context.window.fetch('/backend-api/sentinel/synthetic/prepare', { method: 'POST' });
  assert.deepEqual(
    transportEvents(context).map((item) => item.state),
    ['armed', 'started']
  );

  await context.window.fetch('/backend-api/files/file-one', {
    method: 'POST',
    headers: { Authorization: 'must-not-be-observed' },
    body: 'synthetic body must not be observed'
  });
  context.flushTimers();
  let events = transportEvents(context);
  assert.equal(events.at(-1).state, 'started');
  assert.equal(events.at(-1).completedCount, 0,
    'HTTP success for a file reservation cannot prove uploaded or attached bytes');
  assert.doesNotMatch(JSON.stringify(events), /Authorization|synthetic body|file-one/);

  await context.window.fetch('/backend-api/files/file-one', { method: 'POST' });
  context.flushTimers();
  assert.equal(transportEvents(context).length, events.length, 'repeated progress is deduplicated');

  await context.window.fetch('/backend-api/files/file-two', { method: 'POST' });
  context.flushTimers();
  events = transportEvents(context);
  assert.equal(events.at(-1).completedCount, 0, 'several reservations still prove no completed file');

  observer.arm();
  context.setStatus(503);
  await context.window.fetch('/backend-api/files/failure', { method: 'POST' });
  assert.equal(transportEvents(context).at(-1).state, 'failed');
  assert.equal(transportEvents(context).at(-1).completedCount, 0);

  observer.arm();
  context.setStatus(200);
  const xhr = context.newXhr();
  xhr.open('POST', '/backend-api/files/xhr-file');
  xhr.send('body is intentionally opaque');
  xhr.finish(200);
  context.flushTimers();
  assert.equal(transportEvents(context).at(-1).state, 'started');
  assert.equal(transportEvents(context).at(-1).completedCount, 0);

  assert.equal(observer.version, 2);
  assert.equal(earlyObserver.version, 2, 'the observer installs before the native bridge exists');
  assert.equal(context.timerCount(), 0, 'HTTP success must not schedule guessed completion');
  const installedFetch = context.window.fetch;
  context.inject();
  assert.equal(context.window.fetch, installedFetch, 'reinjection cannot accumulate fetch wrappers');
  assert.equal(context.window.__elonChatGptAttachmentTransportObserver, observer);

  let legacyCancelled = false;
  const upgraded = createContext({
    existingObserver: { version: 1, cancel: () => { legacyCancelled = true; } }
  });
  assert.equal(legacyCancelled, true, 'upgrading cancels pending legacy completion callbacks');
  upgraded.window.__elonChatGptAttachmentTransportObserver.arm();
  await upgraded.window.fetch('/backend-api/files/reservation', { method: 'POST' });
  assert.deepEqual(transportEvents(upgraded).map((item) => item.state), ['armed', 'started']);
  assert.equal(upgraded.timerCount(), 0);

  observer.cancel();
  await context.window.fetch('/backend-api/files/after-cancel', { method: 'POST' });
  context.flushTimers();
  assert.equal(transportEvents(context).at(-1).completedCount, 0);

  console.log('ChatGPT attachment transport observer contract passed.');
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

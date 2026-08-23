'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const source = fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'assets',
  'chatgpt_web_private_transport.js'
), 'utf8');
const policySource = fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'assets',
  'chatgpt_web_private_transport_policy.js'
), 'utf8');

function jsonResponse(value) {
  return { ok: true, status: 200, json: async () => value };
}

class MemoryStorage {
  constructor() { this.values = new Map(); }
  getItem(key) { return this.values.has(key) ? this.values.get(key) : null; }
  setItem(key, value) { this.values.set(key, String(value)); }
}

function createContext(
  fetchImpl,
  researchEnabled = true,
  prefetchEnabled = true,
  storage = new MemoryStorage()
) {
  const timers = new Set();
  const outcomes = [];
  const shapes = [];
  const location = {
    origin: 'https://chatgpt.com',
    pathname: '/',
    href: 'https://chatgpt.com/'
  };
  const window = {
    __elonChatGptPrivateResearchEnabled: researchEnabled,
    __elonChatGptPrivateConversationPrefetchEnabled: prefetchEnabled,
    __elonChatGptPrivateResearchProbe: {
      recordPrivateOutcome: (outcome, messageCount, elapsedMs) => {
        outcomes.push({ outcome, messageCount, elapsedMs });
      },
      recordPrivatePayloadShape: (payload) => { shapes.push(payload); }
    },
    fetch: fetchImpl,
    sessionStorage: storage,
    setTimeout: (callback) => {
      const id = setTimeout(callback, 10000);
      timers.add(id);
      return id;
    },
    clearTimeout: (id) => { clearTimeout(id); timers.delete(id); }
  };
  window.window = window;
  window.location = location;
  const context = {
    window,
    location,
    URL,
    AbortController,
    Date,
    Number,
    String,
    Array,
    Object,
    Map,
    Set,
    Promise,
    Math,
    JSON,
    encodeURIComponent
  };
  vm.runInNewContext(policySource, context, {
    filename: 'chatgpt_web_private_transport_policy.js'
  });
  vm.runInNewContext(source, context, { filename: 'chatgpt_web_private_transport.js' });
  return { window, timers, storage, outcomes, shapes };
}

async function flush() {
  await new Promise((resolve) => setTimeout(resolve, 20));
}

const detailPayload = {
  title: 'Visible title',
  current_node: 'assistant-node',
  mapping: {
    'user-node': {
      parent: '',
      message: {
        id: 'user-message',
        author: { role: 'user' },
        content: { parts: ['hello'] },
        status: 'finished_successfully'
      }
    },
    'assistant-node': {
      parent: 'user-node',
      message: {
        id: 'assistant-message',
        author: { role: 'assistant' },
        content: { parts: ['hi'] },
        status: 'finished_successfully'
      }
    }
  }
};

(async () => {
  const disabled = createContext(async () => jsonResponse(detailPayload), false);
  assert.equal(disabled.window.__elonChatGptPrivateTransport, undefined);

  const gated = createContext(async () => jsonResponse(detailPayload), true, false);
  assert.equal(gated.window.__elonChatGptPrivateTransport.version, 9);
  assert.equal(gated.window.__elonChatGptPrivateTransport.conversationPrefetchEnabled, false);
  assert.equal(gated.window.__elonChatGptPrivateTransport.conversationPrefetchReady(), false);

  const requests = [];
  const snapshots = [];
  let navigated = 0;
  const detail = createContext(async (url, options) => {
    requests.push({ url, options });
    return jsonResponse(detailPayload);
  });
  const transport = detail.window.__elonChatGptPrivateTransport;
  assert.equal(transport.version, 9);
  assert.equal(transport.conversationPrefetchEnabled, true);
  assert.equal(transport.experimentalConversationPrefetchAvailable, true);
  assert.equal(transport.conversationPrefetchReady(), false);
  assert.equal(transport.prefetchConversation(
    '/c/cold-chat',
    () => assert.fail('cold prefetch must not emit'),
    () => assert.fail('cold prefetch leaves navigation to the adapter')
  ), false);

  await detail.window.fetch('/backend-api/conversations/current-chat-id-12345', {
    headers: { Authorization: 'page-scoped-value' }
  });
  assert.equal(transport.conversationPrefetchReady(), true);
  assert.equal(transport.prefetchConversation(
    '/c/plain-chat',
    (event) => snapshots.push(event),
    () => { navigated += 1; }
  ), true);
  await flush();
  assert.equal(navigated, 1);
  assert.equal(snapshots.length, 1);
  assert.equal(requests.length, 2);
  assert.equal(requests[1].url, '/backend-api/conversations/plain-chat');
  assert.equal(requests[1].options.headers.Authorization, 'page-scoped-value');
  assert.equal(requests[1].options.__elonPrivateResearch, 'conversation_prefetch');
  assert.equal(snapshots[0].composerReady, false);
  assert.equal(transport.health().successes, 1);
  assert.equal(transport.health().lastOutcome, 'success');
  assert.equal(detail.outcomes.length, 1);
  assert.equal(detail.outcomes[0].outcome, 'success');
  assert.equal(detail.outcomes[0].messageCount, 2);
  assert.equal(detail.shapes[0], detailPayload);
  assert.deepEqual(
    Array.from(snapshots[0].messages, (value) => [value.role, value.content]),
    [['user', 'hello'], ['assistant', 'hi']]
  );

  let failedNavigation = 0;
  let failedCalls = 0;
  const failed = createContext(async () => {
    failedCalls += 1;
    if (failedCalls === 1) return jsonResponse(detailPayload);
    throw new Error('offline');
  });
  await failed.window.fetch('/backend-api/conversations/current-chat-id-12345', {
    headers: { Authorization: 'page-scoped-value' }
  });
  assert.equal(failed.window.__elonChatGptPrivateTransport.prefetchConversation(
    '/c/plain-chat',
    () => assert.fail('failed prefetch must not emit a snapshot'),
    () => { failedNavigation += 1; }
  ), true);
  await flush();
  assert.equal(failedNavigation, 1);
  assert.equal(failed.window.__elonChatGptPrivateTransport.health().failures, 1);
  assert.equal(failed.outcomes[0].outcome, 'network');

  const wrappedSnapshots = [];
  const wrapped = createContext(async () => jsonResponse({ data: { conversation: detailPayload } }));
  await wrapped.window.fetch('/backend-api/conversations/current-chat-id-12345', {
    headers: { Authorization: 'page-scoped-value' }
  });
  assert.equal(wrapped.window.__elonChatGptPrivateTransport.prefetchConversation(
    '/c/wrapped-chat',
    (event) => wrappedSnapshots.push(event),
    () => {}
  ), true);
  await flush();
  assert.equal(wrappedSnapshots.length, 1);
  assert.equal(wrappedSnapshots[0].messages.length, 2);

  const linearPayload = {
    title: 'Linear title',
    linear_conversation: [
      detailPayload.mapping['user-node'].message,
      detailPayload.mapping['assistant-node'].message
    ]
  };
  const linearSnapshots = [];
  const linear = createContext(async () => jsonResponse(linearPayload));
  await linear.window.fetch('/backend-api/conversations/current-chat-id-12345', {
    headers: { Authorization: 'page-scoped-value' }
  });
  assert.equal(linear.window.__elonChatGptPrivateTransport.prefetchConversation(
    '/c/linear-chat',
    (event) => linearSnapshots.push(event),
    () => {}
  ), true);
  await flush();
  assert.equal(linearSnapshots.length, 1);
  assert.equal(linearSnapshots[0].messages.length, 2);
  assert.equal(failed.window.__elonChatGptPrivateTransport.conversationPrefetchReady(), false);

  console.log('CHATGPT_WEB_PRIVATE_TRANSPORT_TESTS=passed');
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

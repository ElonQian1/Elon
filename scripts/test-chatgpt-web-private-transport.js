'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const source = fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'assets',
  'chatgpt_web_private_transport.js'
), 'utf8');

function jsonResponse(value) {
  return { ok: true, status: 200, json: async () => value };
}

function createContext(fetchImpl, enabled = true) {
  const timers = new Set();
  const location = {
    origin: 'https://chatgpt.com',
    pathname: '/',
    href: 'https://chatgpt.com/'
  };
  const window = {
    __elonChatGptPrivateResearchEnabled: enabled,
    fetch: fetchImpl,
    setTimeout: (callback) => {
      const id = setTimeout(callback, 10000);
      timers.add(id);
      return id;
    },
    clearTimeout: (id) => { clearTimeout(id); timers.delete(id); }
  };
  window.window = window;
  window.location = location;
  vm.runInNewContext(source, {
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
  }, { filename: 'chatgpt_web_private_transport.js' });
  return { window, timers };
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

  const requests = [];
  const snapshots = [];
  let navigated = 0;
  const detail = createContext(async (url, options) => {
    requests.push({ url, options });
    return jsonResponse(detailPayload);
  });
  const transport = detail.window.__elonChatGptPrivateTransport;
  assert.equal(transport.version, 6);
  assert.equal(transport.conversationPrefetchEnabled, false);
  assert.equal(transport.experimentalConversationPrefetchAvailable, true);
  assert.equal(transport.conversationPrefetchReady(), false);
  assert.equal(transport.prefetchConversation(
    '/c/cold-chat',
    () => assert.fail('cold prefetch must not emit'),
    () => assert.fail('cold prefetch leaves navigation to the adapter')
  ), false);

  await detail.window.fetch('/backend-api/conversations/current-chat', {
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
  assert.equal(requests[1].options.headers.Authorization, 'page-scoped-value');
  assert.equal(requests[1].options.__elonPrivateResearch, 'conversation_prefetch');
  assert.equal(snapshots[0].composerReady, false);
  assert.deepEqual(
    Array.from(snapshots[0].messages, (value) => [value.role, value.content]),
    [['user', 'hello'], ['assistant', 'hi']]
  );

  const brokerRequests = [];
  const broker = createContext(async (url, options) => {
    brokerRequests.push({ url, options });
    return jsonResponse(detailPayload);
  });
  broker.window.__elonChatGptPrivateResearchProbe = {
    copyRequestContext: () => ({ Authorization: 'broker-value' })
  };
  assert.equal(broker.window.__elonChatGptPrivateTransport.conversationPrefetchReady(), true);
  broker.window.__elonChatGptPrivateTransport.prefetchConversation(
    '/c/broker-chat',
    () => undefined,
    () => undefined
  );
  await flush();
  assert.equal(brokerRequests[0].options.headers.Authorization, 'broker-value');

  let failedNavigation = 0;
  const failed = createContext(async () => { throw new Error('offline'); });
  await failed.window.fetch('/backend-api/conversations/current-chat', {
    headers: { Authorization: 'page-scoped-value' }
  }).catch(() => undefined);
  assert.equal(failed.window.__elonChatGptPrivateTransport.prefetchConversation(
    '/c/plain-chat',
    () => assert.fail('failed prefetch must not emit a snapshot'),
    () => { failedNavigation += 1; }
  ), true);
  await flush();
  assert.equal(failedNavigation, 1);

  console.log('CHATGPT_WEB_PRIVATE_TRANSPORT_TESTS=passed');
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

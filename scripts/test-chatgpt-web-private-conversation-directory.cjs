const assert = require('assert');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

const root = path.resolve(__dirname, '..');
const assetPath = path.join(
  root,
  'android/app/src/main/assets/chatgpt_web_private_conversation_directory.js'
);
const adapterPath = path.join(root, 'android/app/src/main/assets/chatgpt_web_adapter.js');
const pageAdapterPath = path.join(
  root,
  'android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt'
);
const source = fs.readFileSync(assetPath, 'utf8');
const adapterSource = fs.readFileSync(adapterPath, 'utf8');
const pageAdapterSource = fs.readFileSync(pageAdapterPath, 'utf8');

assert(!/document\.cookie|\.headers\b|\.body\b/i.test(source));
assert(source.includes("url.origin !== location.origin"));
assert(source.includes("method !== 'GET'"));
assert(adapterSource.includes('privateConversationDirectory.setListener(emitPrivateDirectorySnapshot)'));
assert(adapterSource.includes("source: 'official_private'"));
assert.match(pageAdapterSource, /internal const val ADAPTER_VERSION = \d+/);
assert(pageAdapterSource.includes('addDocumentStartJavaScript'));
assert(
  pageAdapterSource.indexOf('chatgpt_web_private_conversation_directory.js') <
    pageAdapterSource.indexOf('chatgpt_web_adapter.js')
);

const responses = new Map([
  ['/backend-api/conversations', JSON.stringify({
    items: [
      { id: 'global-chat-12345', title: '普通聊天', is_pinned: true },
      { id: 'bad id', title: '忽略无效 ID' }
    ]
  })],
  ['/backend-api/gizmos/snorlax/sidebar', JSON.stringify({
    items: [{ gizmo: { id: 'g-p-health123', display: { name: '家庭健康' } } }]
  })],
  ['/backend-api/gizmos/g-p-health123/conversations', JSON.stringify({
    data: { conversations: [{ conversation_id: 'project-chat-12345', title: '健康记录' }] }
  })]
]);
const fetchCalls = [];
let cloneCount = 0;

class FakeResponse {
  constructor(status, text) {
    this.status = status;
    this.value = text;
  }
  clone() {
    cloneCount += 1;
    return new FakeResponse(this.status, this.value);
  }
  text() { return Promise.resolve(this.value); }
}

function responseFor(rawUrl) {
  const url = new URL(String(rawUrl && rawUrl.url || rawUrl), 'https://chatgpt.com/');
  return new FakeResponse(200, responses.get(url.pathname) || '{}');
}

function originalFetch(input, init) {
  fetchCalls.push({ input, init });
  return Promise.resolve(responseFor(input));
}

class FakeXhr {
  constructor() {
    this.status = 0;
    this.responseText = '';
    this.listeners = new Map();
    this.sendCount = 0;
  }
  open(method, rawUrl) {
    this.method = method;
    this.rawUrl = rawUrl;
  }
  addEventListener(name, listener) { this.listeners.set(name, listener); }
  send() {
    this.sendCount += 1;
    const response = responseFor(this.rawUrl);
    this.status = response.status;
    this.responseText = response.value;
    const listener = this.listeners.get('load');
    if (listener) listener();
  }
}

const location = {
  origin: 'https://chatgpt.com',
  href: 'https://chatgpt.com/g/g-p-health123/c/project-chat-12345',
  pathname: '/g/g-p-health123/c/project-chat-12345'
};
const window = { fetch: originalFetch, XMLHttpRequest: FakeXhr };
vm.runInNewContext(source, {
  window,
  location,
  URL,
  Promise,
  JSON,
  Map,
  WeakMap,
  Set,
  Object,
  Array,
  String,
  Number,
  RegExp
}, { filename: 'chatgpt_web_private_conversation_directory.js' });

async function flush() {
  await Promise.resolve();
  await Promise.resolve();
  await new Promise((resolve) => setImmediate(resolve));
}

(async () => {
  const directory = window.__elonChatGptPrivateConversationDirectory;
  assert(directory);
  assert.strictEqual(directory.version, 1);
  let notifications = 0;
  directory.setListener(() => { notifications += 1; });

  await window.fetch('https://chatgpt.com/backend-api/conversations?offset=0&limit=28');
  await window.fetch('https://chatgpt.com/backend-api/gizmos/snorlax/sidebar');
  await window.fetch('https://chatgpt.com/backend-api/gizmos/g-p-health123/conversations');
  await flush();

  const snapshot = directory.snapshot();
  assert.strictEqual(snapshot.complete, false);
  assert.strictEqual(snapshot.projects.length, 1);
  assert.strictEqual(snapshot.projects[0].title, '家庭健康');
  assert.strictEqual(snapshot.projects[0].active, true);
  assert.strictEqual(snapshot.conversations.length, 2);
  assert(snapshot.conversations.some((row) => row.path === '/c/global-chat-12345'));
  const project = snapshot.conversations.find((row) => row.id === 'project-chat-12345');
  assert(project);
  assert.strictEqual(project.path, '/g/g-p-health123/c/project-chat-12345');
  assert.strictEqual(project.projectTitle, '家庭健康');
  assert.strictEqual(project.active, true);
  assert(notifications >= 3);
  assert.strictEqual(fetchCalls.length, 3);
  assert.strictEqual(cloneCount, 3);

  const beforeIgnored = directory.snapshot().revision;
  await window.fetch('https://chatgpt.com/backend-api/conversations', { method: 'POST' });
  await window.fetch('https://example.com/backend-api/conversations');
  await flush();
  assert.strictEqual(directory.snapshot().revision, beforeIgnored);
  assert.strictEqual(fetchCalls.length, 5);

  const xhr = new window.XMLHttpRequest();
  xhr.open('GET', '/backend-api/conversations?offset=28&limit=28');
  xhr.send();
  assert.strictEqual(xhr.sendCount, 1);
  assert(directory.snapshot().conversations.some((row) => row.id === 'global-chat-12345'));

  console.log('chatgpt private conversation directory contract: ok');
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

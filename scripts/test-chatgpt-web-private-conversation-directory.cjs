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
const directoryRequestsPath = path.join(
  root,
  'android/app/src/main/assets/chatgpt_web_adapter_conversation_directory_requests.js'
);
const pageAdapterPath = path.join(
  root,
  'android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt'
);
const source = fs.readFileSync(assetPath, 'utf8');
const adapterSource = fs.readFileSync(adapterPath, 'utf8');
const directoryRequestsSource = fs.readFileSync(directoryRequestsPath, 'utf8');
const pageAdapterSource = fs.readFileSync(pageAdapterPath, 'utf8');

assert(!/document\.cookie|\.headers\b|\.body\b/i.test(source));
assert(source.includes("url.origin !== location.origin"));
assert(source.includes("method !== 'GET'"));
assert(source.includes('const PROJECT_REFRESH_TIMEOUT_MS = 4000'));
assert(source.includes('acceptConversationMembership'));
assert(source.includes('acceptPinnedState'));
assert(source.includes('const PIN_OVERRIDE_TTL_MS = 120000'));
assert(source.includes('window.setTimeout(() =>'));
assert(directoryRequestsSource.includes('privateDirectory.setListener(() => emitSnapshot(null))'));
assert(directoryRequestsSource.includes('privateDirectory.refreshProject(projectId)'));
assert(directoryRequestsSource.includes('conversationAdapter.requestList(command, emitEvent, respond)'));
assert(adapterSource.includes("action === 'probe_conversation_project'"));
assert(directoryRequestsSource.includes("source: 'official_private'"));
assert(directoryRequestsSource.includes("scopeProjectId: projectId || null"));
assert(directoryRequestsSource.includes('emitSnapshot(projectId, true)'));
assert(directoryRequestsSource.includes('const complete = Boolean(projectId && scopedComplete === true)'));
assert(directoryRequestsSource.includes("value.conversations.filter((item) => item && item.projectId === projectId)"));
assert(source.includes('replaceProjectConversations(projectId, text)'));
assert.match(pageAdapterSource, /internal const val ADAPTER_VERSION = \d+/);
assert(pageAdapterSource.includes('addDocumentStartJavaScript'));
assert(
  pageAdapterSource.indexOf('chatgpt_web_private_conversation_directory.js') <
    pageAdapterSource.indexOf('chatgpt_web_adapter.js')
);

const responses = new Map([
  ['/backend-api/conversations', JSON.stringify({
    items: [
      { id: 'global-chat-12345', title: '普通聊天', is_starred: true },
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
const hangingPaths = new Set();
const pendingTimeouts = new Map();
let nextTimeoutId = 1;
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
  const url = new URL(String(input && input.url || input), 'https://chatgpt.com/');
  if (hangingPaths.has(url.pathname)) return new Promise(() => {});
  return Promise.resolve(responseFor(input));
}

function scheduledTimeout(callback, delay) {
  if (delay !== 4000) return setTimeout(callback, delay);
  const id = nextTimeoutId++;
  pendingTimeouts.set(id, () => {
    pendingTimeouts.delete(id);
    callback();
  });
  return id;
}

function cancelTimeout(id) {
  if (!pendingTimeouts.delete(id)) clearTimeout(id);
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
const window = {
  fetch: originalFetch,
  XMLHttpRequest: FakeXhr,
  setTimeout: scheduledTimeout,
  clearTimeout: cancelTimeout
};
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
  assert.strictEqual(directory.version, 6);
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
  const global = snapshot.conversations.find((row) => row.id === 'global-chat-12345');
  assert.strictEqual(global.pinned, true);
  const project = snapshot.conversations.find((row) => row.id === 'project-chat-12345');
  assert(project);
  assert.strictEqual(project.path, '/g/g-p-health123/c/project-chat-12345');
  assert.strictEqual(project.projectTitle, '家庭健康');
  assert.strictEqual(project.active, true);
  assert.strictEqual(project.pinned, null);
  assert(notifications >= 3);
  assert.strictEqual(fetchCalls.length, 3);
  assert.strictEqual(cloneCount, 3);

  const notificationsBeforeMembership = notifications;
  assert.strictEqual(directory.acceptConversationMembership(
    'membership-chat-12345',
    '移动后立即可见',
    'g-p-destination123'
  ), true);
  const membership = directory.snapshot().conversations.find(
    (row) => row.id === 'membership-chat-12345'
  );
  assert(membership);
  assert.strictEqual(membership.projectId, 'g-p-destination123');
  assert.strictEqual(
    membership.path,
    '/g/g-p-destination123/c/membership-chat-12345'
  );
  assert.strictEqual(notifications, notificationsBeforeMembership + 1);
  assert.strictEqual(directory.acceptConversationMembership(
    'membership-chat-12345',
    '移动后立即可见',
    'not-a-project'
  ), false);

  const notificationsBeforePin = notifications;
  assert.strictEqual(directory.acceptPinnedState('global-chat-12345', false), true);
  assert.strictEqual(
    directory.snapshot().conversations.find((row) => row.id === 'global-chat-12345').pinned,
    false
  );
  assert.strictEqual(notifications, notificationsBeforePin + 1);
  assert.strictEqual(directory.acceptPinnedState('missing-chat', true), false);

  await window.fetch('https://chatgpt.com/backend-api/conversations?offset=0&limit=28');
  await flush();
  assert.strictEqual(
    directory.snapshot().conversations.find((row) => row.id === 'global-chat-12345').pinned,
    false,
    'a stale directory response must not overwrite a recently reconciled pin state'
  );
  responses.set('/backend-api/conversations', JSON.stringify({
    items: [{ id: 'global-chat-12345', title: '普通聊天', is_starred: false }]
  }));
  await window.fetch('https://chatgpt.com/backend-api/conversations?offset=0&limit=28');
  await flush();
  assert.strictEqual(
    directory.snapshot().conversations.find((row) => row.id === 'global-chat-12345').pinned,
    false
  );

  responses.set('/backend-api/gizmos/g-p-health123/conversations', JSON.stringify({
    data: { conversations: [
      { conversation_id: 'project-chat-12345', title: '健康记录' },
      { conversation_id: 'moved-chat-12345', title: '移动后的会话' }
    ] }
  }));
  const beforeProjectRefresh = fetchCalls.length;
  const notificationsBeforeProjectRefresh = notifications;
  const refreshResults = await Promise.all([
    directory.refreshProject('g-p-health123'),
    directory.refreshProject('g-p-health123')
  ]);
  await flush();
  assert.deepStrictEqual(Array.from(refreshResults), [true, true]);
  assert.strictEqual(fetchCalls.length, beforeProjectRefresh + 1);
  const targetedRequest = fetchCalls[fetchCalls.length - 1];
  assert.strictEqual(targetedRequest.input, '/backend-api/gizmos/g-p-health123/conversations');
  assert.strictEqual(targetedRequest.init.method, 'GET');
  assert.strictEqual(targetedRequest.init.credentials, 'same-origin');
  assert.strictEqual(targetedRequest.init.cache, 'no-store');
  assert.strictEqual(targetedRequest.init.headers, undefined);
  assert.strictEqual(targetedRequest.init.body, undefined);
  assert.strictEqual(notifications, notificationsBeforeProjectRefresh);
  assert(directory.snapshot().conversations.some((row) => row.id === 'moved-chat-12345'));
  responses.set('/backend-api/gizmos/g-p-health123/conversations', JSON.stringify({
    data: { conversations: [{ conversation_id: 'replacement-chat-12345', title: '替换后的会话' }] }
  }));
  assert.strictEqual(await directory.refreshProject('g-p-health123'), true);
  const replacedSnapshot = directory.snapshot();
  assert(!replacedSnapshot.conversations.some((row) => row.id === 'project-chat-12345'));
  assert(!replacedSnapshot.conversations.some((row) => row.id === 'moved-chat-12345'));
  assert(replacedSnapshot.conversations.some((row) => row.id === 'replacement-chat-12345'));
  const beforeInvalidRefresh = fetchCalls.length;
  assert.strictEqual(await directory.refreshProject('not-a-project'), false);
  assert.strictEqual(fetchCalls.length, beforeInvalidRefresh);

  responses.set('/backend-api/gizmos/g-p-empty123/conversations', '{}');
  assert.strictEqual(await directory.refreshProject('g-p-empty123'), false);

  hangingPaths.add('/backend-api/gizmos/g-p-hanging123/conversations');
  const hangingRefresh = directory.refreshProject('g-p-hanging123');
  await flush();
  assert.strictEqual(pendingTimeouts.size, 1);
  pendingTimeouts.values().next().value();
  assert.strictEqual(await hangingRefresh, false);
  assert.strictEqual(pendingTimeouts.size, 0);

  const beforeIgnored = directory.snapshot().revision;
  const beforeIgnoredCalls = fetchCalls.length;
  await window.fetch('https://chatgpt.com/backend-api/conversations', { method: 'POST' });
  await window.fetch('https://example.com/backend-api/conversations');
  await flush();
  assert.strictEqual(directory.snapshot().revision, beforeIgnored);
  assert.strictEqual(fetchCalls.length, beforeIgnoredCalls + 2);

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

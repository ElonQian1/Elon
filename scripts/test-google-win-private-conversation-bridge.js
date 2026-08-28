const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const root = path.resolve(__dirname, '..');
const source = fs.readFileSync(path.join(
  root,
  'desktop-shell/src-tauri/src/local_ai_browser/google_win_private_conversation_bridge.js'
), 'utf8');

const baseCommands = [];
const events = [];
const navigations = [];
let privateReplyResets = 0;
let baseDisposes = 0;
const location = {
  origin: 'https://www.google.com',
  href: 'https://www.google.com/search?udm=50&q=current&csuir=active_thread_1234',
  assign(value) { navigations.push(String(value)); },
};
const baseBridge = {
  version: 37,
  documentToken: 'doc_test',
  command(raw) { baseCommands.push(String(raw)); },
  dispose() { baseDisposes += 1; },
};
const window = {
  __elonGoogleWebBridge: baseBridge,
  __elonGoogleWebPrivateThreadDirectory: {
    snapshot() {
      return [{
        id: 'thread_1234567890',
        title: 'BTC 走势',
        path: '/c/thread_1234567890',
        providerUrl: 'https://www.google.com/search?udm=50&q=BTC&csuir=thread_1234567890',
      }, {
        id: 'prompt_1234567890',
        title: '瞬时提问',
        path: '/c/prompt_1234567890',
        providerUrl: 'https://www.google.com/search?udm=50&q=prompt-only',
      }];
    },
  },
  elonGoogleWebNative: {
    postMessage(raw) { events.push(JSON.parse(String(raw))); },
  },
  __elonWinGooglePrivateReplyState: {
    reset() { privateReplyResets += 1; },
  },
};

vm.runInNewContext(source, {
  window,
  location,
  URL,
  Set,
  JSON,
  Object,
  String,
  Array,
  Number,
});

assert.equal(window.__elonWinGooglePrivateConversationBridgeVersion, 4);
assert.notEqual(window.__elonGoogleWebBridge, baseBridge);
const installedBridge = window.__elonGoogleWebBridge;

window.__elonGoogleWebBridge.command(JSON.stringify({
  action: 'list_conversations',
  requestId: 'mcp_list1',
}));
assert.deepEqual(events.pop(), {
  type: 'command_result',
  action: 'list_conversations',
  ok: true,
  detail: '已同步 2 个 Google AI 官网会话。',
  requestId: 'mcp_list1',
});

window.__elonGoogleWebBridge.command(JSON.stringify({
  action: 'open_conversation',
  value: '/c/thread_1234567890',
  requestId: 'mcp_open1',
}));
assert.equal(events.pop().ok, true);
assert.equal(
  navigations.pop(),
  'https://www.google.com/search?udm=50&q=BTC&csuir=thread_1234567890',
);
assert.equal(privateReplyResets, 1);

window.__elonGoogleWebBridge.command(JSON.stringify({
  action: 'open_conversation',
  value: '/c/prompt_1234567890',
  requestId: 'mcp_open_prompt',
}));
assert.equal(events.pop().ok, false);
assert.equal(navigations.length, 0);
assert.equal(privateReplyResets, 1);

window.__elonGoogleWebBridge.command(JSON.stringify({
  action: 'open_conversation',
  value: '/c/missing_thread_1234',
  requestId: 'mcp_open2',
}));
assert.equal(events.pop().ok, false);
assert.equal(navigations.length, 0);
assert.equal(privateReplyResets, 1);

const newConversation = JSON.stringify({
  action: 'new_conversation',
  requestId: 'mcp_new1',
});
window.__elonGoogleWebBridge.command(newConversation);
assert.equal(privateReplyResets, 2);
assert.equal(baseCommands.pop(), newConversation);

const passthrough = JSON.stringify({ action: 'snapshot' });
window.__elonGoogleWebBridge.command(passthrough);
assert.deepEqual(baseCommands, [passthrough]);

const reboundCommands = [];
const reboundEvents = [];
const reboundBaseBridge = {
  version: 38,
  documentToken: 'doc_rebound',
  command(raw) { reboundCommands.push(String(raw)); },
  dispose() {},
};
window.__elonGoogleWebBridge = reboundBaseBridge;
window.__elonGoogleWebPrivateThreadDirectory = {
  snapshot() {
    return [
      {
        id: 'thread_2222222222',
        title: 'ETH 走势',
        path: '/c/thread_2222222222',
        providerUrl: 'https://www.google.com/search?udm=50&q=ETH&csuir=thread_2222222222',
      },
      {
        id: 'thread_3333333333',
        title: '黄金走势',
        path: '/c/thread_3333333333',
        providerUrl: 'https://www.google.com/search?udm=50&q=gold&csuir=thread_3333333333',
      },
    ];
  },
};
window.elonGoogleWebNative = {
  postMessage(raw) { reboundEvents.push(JSON.parse(String(raw))); },
};
vm.runInNewContext(source, {
  window,
  location,
  URL,
  Set,
  JSON,
  Object,
  String,
  Array,
  Number,
});

assert.equal(window.__elonGoogleWebBridge, installedBridge);
assert.equal(baseDisposes, 1, 'stale Google adapter bridge must be disposed during rebind');
assert.equal(window.__elonGoogleWebBridge.version, 38);
assert.equal(window.__elonGoogleWebBridge.documentToken, 'doc_rebound');
assert.equal(
  window.__elonWinGooglePrivateConversationBridge.diagnostics(),
  'v4|bindings=2',
);
window.__elonGoogleWebBridge.command(JSON.stringify({
  action: 'list_conversations',
  requestId: 'mcp_list2',
}));
assert.equal(reboundEvents.pop().detail, '已同步 2 个 Google AI 官网会话。');
window.__elonGoogleWebBridge.command(passthrough);
assert.deepEqual(reboundCommands, [passthrough]);
assert.equal(events.length, 0, 'rebound results must not leak to the stale native bridge');
assert.doesNotMatch(source, /document\.cookie|authorization|access[_-]?token|fetch\s*\(/i);

process.stdout.write('PASS Google Win private conversation bridge\n');

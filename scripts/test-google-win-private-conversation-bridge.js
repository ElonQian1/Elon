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
const location = {
  origin: 'https://www.google.com',
  href: 'https://www.google.com/search?udm=50&q=current&csuir=active_thread_1234',
  assign(value) { navigations.push(String(value)); },
};
const baseBridge = {
  version: 37,
  documentToken: 'doc_test',
  command(raw) { baseCommands.push(String(raw)); },
  dispose() {},
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
      }];
    },
  },
  elonGoogleWebNative: {
    postMessage(raw) { events.push(JSON.parse(String(raw))); },
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

assert.equal(window.__elonWinGooglePrivateConversationBridgeVersion, 1);
assert.notEqual(window.__elonGoogleWebBridge, baseBridge);

window.__elonGoogleWebBridge.command(JSON.stringify({
  action: 'list_conversations',
  requestId: 'mcp_list1',
}));
assert.deepEqual(events.pop(), {
  type: 'command_result',
  action: 'list_conversations',
  ok: true,
  detail: '已同步 1 个 Google AI 官网会话。',
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

window.__elonGoogleWebBridge.command(JSON.stringify({
  action: 'open_conversation',
  value: '/c/missing_thread_1234',
  requestId: 'mcp_open2',
}));
assert.equal(events.pop().ok, false);
assert.equal(navigations.length, 0);

const passthrough = JSON.stringify({ action: 'snapshot' });
window.__elonGoogleWebBridge.command(passthrough);
assert.deepEqual(baseCommands, [passthrough]);
assert.doesNotMatch(source, /document\.cookie|authorization|access[_-]?token|fetch\s*\(/i);

process.stdout.write('PASS Google Win private conversation bridge\n');

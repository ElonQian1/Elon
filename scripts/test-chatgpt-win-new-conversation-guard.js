'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const source = fs.readFileSync(path.join(
  __dirname, '..', 'desktop-shell', 'src-tauri', 'src', 'local_ai_browser',
  'chatgpt_win_new_conversation_guard.js'
), 'utf8');

function createContext(initialPath) {
  let current = { messageCount: 2, composerReady: true };
  const results = [];
  const location = { origin: 'https://chatgpt.com', pathname: initialPath };
  const original = Object.freeze({
    marker: 'preserved',
    newConversation: (_inspect, result) => {
      current = { messageCount: 0, composerReady: true };
      result('new_conversation', true, '');
    }
  });
  const window = {
    __elonChatGptConversations: original,
    setTimeout
  };
  window.window = window;
  vm.runInNewContext(source, {
    window,
    location,
    Date,
    Number,
    Object,
    RegExp
  }, { filename: 'chatgpt_win_new_conversation_guard.js' });
  return {
    adapter: window.__elonChatGptConversations,
    guard: window.__elonWinChatGptNewConversationGuard,
    inspect: () => current,
    location,
    results
  };
}

async function runNewConversation(context) {
  context.adapter.newConversation(context.inspect, (...value) => context.results.push(value));
  await new Promise((resolve) => setTimeout(resolve, context.guard.confirmTimeoutMs + 120));
}

(async () => {
  const stale = createContext('/c/old-conversation');
  await runNewConversation(stale);
  assert.equal(stale.adapter.marker, 'preserved');
  assert.deepEqual(stale.results, [[
    'new_conversation', false, '官网未离开上一会话，已转入安全恢复。'
  ]]);

  const routed = createContext('/c/old-conversation');
  setTimeout(() => { routed.location.pathname = '/'; }, 120);
  await runNewConversation(routed);
  assert.deepEqual(routed.results, [['new_conversation', true, '']]);

  const guest = createContext('/');
  await runNewConversation(guest);
  assert.deepEqual(guest.results, [['new_conversation', true, '']]);

  process.stdout.write('PASS Win ChatGPT new-conversation route guard\n');
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

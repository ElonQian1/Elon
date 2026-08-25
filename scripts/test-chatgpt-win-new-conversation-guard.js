'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const source = fs.readFileSync(path.join(
  __dirname, '..', 'desktop-shell', 'src-tauri', 'src', 'local_ai_browser',
  'chatgpt_win_new_conversation_guard.js'
), 'utf8');

function createContext(initialPath, options = {}) {
  let current = {
    messageCount: options.initialMessageCount === undefined ? 2 : options.initialMessageCount,
    composerReady: true
  };
  let now = 0;
  let nextTimerId = 1;
  const timers = [];
  const results = [];
  const location = { origin: 'https://chatgpt.com', pathname: initialPath };
  const oldTurn = {
    isConnected: options.initialTurn !== false && current.messageCount > 0,
    textContent: 'previous user and assistant turn'
  };
  const document = {
    documentElement: {
      contains: (node) => node.isConnected !== false
    },
    querySelectorAll: () => oldTurn.isConnected ? [oldTurn] : []
  };
  const FakeDate = class extends Date {
    static now() { return now; }
  };
  const setTimeoutFake = (callback, delay = 0) => {
    const id = nextTimerId++;
    timers.push({ id, at: now + Number(delay || 0), callback });
    return id;
  };
  const original = Object.freeze({
    marker: 'preserved',
    newConversation: (_inspect, result) => {
      current = { messageCount: 0, composerReady: true };
      if (options.detachTurnOnClick) oldTurn.isConnected = false;
      if (options.routeOnClick) location.pathname = options.routeOnClick;
      result('new_conversation', true, '');
    }
  });
  const window = {
    __elonChatGptConversations: original,
    setTimeout: setTimeoutFake
  };
  window.window = window;
  vm.runInNewContext(source, {
    window,
    document,
    location,
    Date: FakeDate,
    Number,
    Object,
    RegExp,
    Set,
    String,
    Array
  }, { filename: 'chatgpt_win_new_conversation_guard.js' });

  function advance(milliseconds) {
    const target = now + milliseconds;
    while (true) {
      timers.sort((left, right) => left.at - right.at || left.id - right.id);
      const timer = timers[0];
      if (!timer || timer.at > target) break;
      timers.shift();
      now = timer.at;
      timer.callback();
    }
    now = target;
  }

  return {
    adapter: window.__elonChatGptConversations,
    guard: window.__elonWinChatGptNewConversationGuard,
    inspect: () => current,
    location,
    results,
    advance
  };
}

function runNewConversation(context) {
  context.adapter.newConversation(context.inspect, (...value) => context.results.push(value));
  context.advance(context.guard.confirmTimeoutMs + context.guard.confirmStableMs + 500);
}

const staleRoute = createContext('/c/old-conversation');
runNewConversation(staleRoute);
assert.equal(staleRoute.adapter.marker, 'preserved');
assert.deepEqual(staleRoute.results, [[
  'new_conversation', false, '官网未离开上一会话，已转入安全恢复。'
]]);

const routed = createContext('/c/old-conversation', {
  detachTurnOnClick: true,
  routeOnClick: '/'
});
runNewConversation(routed);
assert.deepEqual(routed.results, [['new_conversation', true, '']]);

const blankGuest = createContext('/', { initialMessageCount: 0, initialTurn: false });
runNewConversation(blankGuest);
assert.deepEqual(blankGuest.results, [['new_conversation', true, '']]);

const staleGuest = createContext('/');
runNewConversation(staleGuest);
assert.deepEqual(staleGuest.results, [[
  'new_conversation', false, '官网未离开上一会话，已转入安全恢复。'
]]);

const freshGuest = createContext('/', { detachTurnOnClick: true });
runNewConversation(freshGuest);
assert.deepEqual(freshGuest.results, [['new_conversation', true, '']]);

process.stdout.write('PASS Win ChatGPT new-conversation route and DOM guard\n');

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
  const sandbox = {
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
  };
  vm.runInNewContext(source, sandbox, { filename: 'chatgpt_win_new_conversation_guard.js' });

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
    advance,
    reinstall(conversations) {
      window.__elonChatGptConversations = conversations;
      vm.runInNewContext(source, sandbox, { filename: 'chatgpt_win_new_conversation_guard.js' });
      this.adapter = window.__elonChatGptConversations;
      this.guard = window.__elonWinChatGptNewConversationGuard;
    }
  };
}

function runNewConversation(context) {
  context.adapter.newConversation(context.inspect, (...value) => context.results.push(value));
  context.advance(context.guard.confirmTimeoutMs + context.guard.confirmStableMs + 500);
}

const staleRoute = createContext('/c/old-conversation');
runNewConversation(staleRoute);
assert.equal(staleRoute.adapter.baseConversations.marker, 'preserved');
assert.equal(staleRoute.guard.version, 4);
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

const reconnect = createContext('/c/old-conversation');
const firstGuardedConversations = reconnect.adapter;
let replacementCalls = 0;
const replacement = Object.freeze({
  marker: 'replacement',
  capabilities: () => ['replacement'],
  newConversation: (_inspect, result) => {
    replacementCalls += 1;
    result('new_conversation', false, 'replacement-delegate');
  }
});
reconnect.reinstall(replacement);
assert.equal(reconnect.adapter, firstGuardedConversations);
assert.equal(reconnect.adapter.baseConversations, replacement);
assert.equal(reconnect.adapter.__elonWinNewConversationGuardWrapped, true);
assert.equal(reconnect.guard.conversations, reconnect.adapter);
assert.deepEqual(reconnect.adapter.capabilities(), ['replacement']);
reconnect.results.length = 0;
runNewConversation(reconnect);
assert.equal(replacementCalls, 1);
assert.deepEqual(reconnect.results, [[
  'new_conversation', false, 'replacement-delegate'
]]);
assert.equal(reconnect.guard.diagnostics(), 'v4|bindings=2');

reconnect.reinstall(reconnect.adapter);
assert.equal(reconnect.adapter, firstGuardedConversations);
assert.equal(reconnect.guard.diagnostics(), 'v4|bindings=2');

process.stdout.write('PASS Win ChatGPT new-conversation route, DOM guard, and hot rebind\n');

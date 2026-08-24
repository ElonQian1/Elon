'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const source = fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'assets',
  'chatgpt_web_adapter_conversations.js'
), 'utf8');

function createVisibleNode(onClick) {
  return {
    click: onClick,
    textContent: 'New chat',
    getAttribute: (name) => name === 'aria-label' ? 'New chat' : '',
    getBoundingClientRect: () => ({
      width: 40, height: 40, top: 0, left: 0, right: 40, bottom: 40
    })
  };
}

function createContext(initialMessageCount) {
  let messageCount = initialMessageCount;
  let clicks = 0;
  const newChat = createVisibleNode(() => {
    clicks += 1;
    messageCount = 0;
  });
  const document = {
    body: {},
    documentElement: {},
    querySelector: (selector) => selector.includes('create-new-chat-button') ? newChat : null,
    querySelectorAll: () => []
  };
  const window = {
    innerHeight: 900,
    innerWidth: 1200,
    getComputedStyle: () => ({ display: 'block', visibility: 'visible' }),
    setTimeout
  };
  window.window = window;
  const sandbox = {
    window,
    document,
    location: { origin: 'https://chatgpt.com', pathname: '/' },
    URL,
    Date,
    Number,
    String,
    Math,
    Array,
    Set,
    Object,
    RegExp
  };
  vm.runInNewContext(source, sandbox, { filename: 'chatgpt_web_adapter_conversations.js' });
  return {
    adapter: window.__elonChatGptConversations,
    clicks: () => clicks,
    inspect: () => ({ messageCount, composerReady: true })
  };
}

(async () => {
  const active = createContext(2);
  const results = [];
  active.adapter.newConversation(active.inspect, (...value) => results.push(value));
  assert.equal(active.clicks(), 1, 'root-path visitor chats must still activate New chat');
  assert.equal(results.length, 0, 'success is not acknowledged before the old answer disappears');
  await new Promise((resolve) => setTimeout(resolve, 260));
  assert.deepEqual(results, [['new_conversation', true, '']]);

  const blank = createContext(0);
  const blankResults = [];
  blank.adapter.newConversation(blank.inspect, (...value) => blankResults.push(value));
  assert.equal(blank.clicks(), 1, 'the official control remains the source of truth even on root');
  await new Promise((resolve) => setTimeout(resolve, 260));
  assert.deepEqual(blankResults, [['new_conversation', true, '']]);

  process.stdout.write('PASS ChatGPT new-conversation boundary\n');
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

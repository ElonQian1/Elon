'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const source = fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'assets',
  'chatgpt_web_adapter_conversations.js'
), 'utf8');

function visibleNode(attributes, onClick) {
  return {
    click: onClick,
    textContent: attributes.text || '',
    getAttribute: (name) => attributes[name] || '',
    getBoundingClientRect: () => ({
      width: 40, height: 40, top: 0, left: 0, right: 40, bottom: 40
    })
  };
}

(async () => {
  const targetPath = '/c/conversation-target';
  let sidebarOpen = true;
  let closeClicks = 0;
  const location = {
    origin: 'https://chatgpt.com',
    pathname: '/c/conversation-source',
    assign: (href) => { location.pathname = new URL(href).pathname; }
  };
  const target = visibleNode({ href: targetPath }, () => {
    location.pathname = targetPath;
  });
  const close = visibleNode({ 'aria-label': 'Close sidebar' }, () => {
    closeClicks += 1;
    sidebarOpen = false;
  });
  const document = {
    body: {},
    documentElement: {},
    querySelector: (selector) => {
      if (sidebarOpen && selector.includes('close-sidebar')) return close;
      return null;
    },
    querySelectorAll: (selector) => {
      if (selector === 'a[href*="/c/"]') return [target];
      if (selector === 'button') return sidebarOpen ? [close] : [];
      return [];
    }
  };
  const window = {
    innerHeight: 900,
    innerWidth: 1200,
    getComputedStyle: () => ({ display: 'block', visibility: 'visible' }),
    setTimeout
  };
  window.window = window;
  vm.runInNewContext(source, {
    window,
    document,
    location,
    URL,
    Date,
    Number,
    String,
    Math,
    Array,
    Set,
    Object,
    RegExp
  }, { filename: 'chatgpt_web_adapter_conversations.js' });

  const results = [];
  window.__elonChatGptConversations.openConversation(
    targetPath,
    (...value) => results.push(value)
  );
  assert.deepEqual(results, [['open_conversation', true, '']]);
  assert.equal(location.pathname, targetPath);
  await new Promise((resolve) => setTimeout(resolve, 180));
  assert.equal(closeClicks, 1, 'route navigation must dismiss the official mobile sidebar');
  assert.equal(sidebarOpen, false);

  process.stdout.write('PASS ChatGPT open-conversation sidebar boundary\n');
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

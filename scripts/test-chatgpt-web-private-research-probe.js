'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const source = fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'assets',
  'chatgpt_web_private_research_probe.js'
), 'utf8');
const buildGradle = fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'build.gradle'
), 'utf8');
const pageAdapter = fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'kotlin',
  'com', 'elon', 'app', 'chatgptweb', 'ChatGptWebPageAdapter.kt'
), 'utf8');

assert.match(
  buildGradle,
  /findProperty\("ELON_CHATGPT_PRIVATE_RESEARCH"\)[\s\S]*?\?\.toBoolean\(\) \?: false/
);
assert.match(
  buildGradle,
  /buildConfigField "boolean", "CHATGPT_PRIVATE_RESEARCH_ENABLED"/
);
assert.match(
  buildGradle,
  /private research version overrides require ELON_CHATGPT_PRIVATE_RESEARCH=true/
);
assert.match(
  buildGradle,
  /versionCode chatGptPrivateResearchVersionCode\.toInteger\(\)/
);
assert.match(
  pageAdapter,
  /window\.__elonChatGptPrivateResearchEnabled =[\s\S]*?BuildConfig\.CHATGPT_PRIVATE_RESEARCH_ENABLED/
);

function response(status, contentType) {
  return {
    status,
    headers: { get: (name) => name === 'content-type' ? contentType : null }
  };
}

async function run(enabled) {
  const events = [];
  const requests = [];
  class FakeXhr {
    constructor() { this.listeners = {}; this.status = 0; this.contentType = ''; }
    open(method, url) { this.method = method; this.url = url; }
    send() {}
    addEventListener(name, callback) { this.listeners[name] = callback; }
    getResponseHeader(name) { return name === 'content-type' ? this.contentType : null; }
  }
  const window = {
    __elonChatGptPrivateResearchEnabled: enabled,
    __elonChatGptAdapterTargetVersion: 142,
    __elonChatGptDocumentToken: 'doc_private_research',
    elonChatGptNative: { postMessage: (value) => events.push(JSON.parse(value)) },
    performance: { now: () => Date.now() },
    XMLHttpRequest: FakeXhr,
    fetch: async (input, init = {}) => {
      requests.push({ input, init });
      return response(200, 'application/json');
    }
  };
  window.window = window;
  const context = {
    window,
    location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/' },
    URL,
    WeakMap,
    Promise,
    Date,
    Number,
    Math,
    JSON
  };
  vm.runInNewContext(source, context, { filename: 'chatgpt_web_private_research_probe.js' });

  await window.fetch('https://chatgpt.com/backend-api/conversations?offset=0&limit=1');
  await window.fetch('https://chatgpt.com/backend-api/conversations/private-chat-id', {
    headers: { Authorization: 'must-not-be-observed' },
    __elonPrivateResearch: 'conversation_prefetch'
  });
  await window.fetch('https://chatgpt.com/backend-api/conversation/123e4567-e89b-12d3-a456-426614174000', {
    method: 'POST',
    body: 'must-not-be-observed'
  });
  await window.fetch('https://chatgpt.com/backend-api/f/conversation', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'OpenAI-Sentinel-Proof-Token': 'must-not-be-observed'
    },
    body: JSON.stringify({
      action: 'next',
      model: 'must-not-be-observed',
      messages: [{
        id: 'must-not-be-observed',
        author: { role: 'user' },
        content: { content_type: 'text', parts: ['must-not-be-observed'] }
      }]
    })
  });
  await window.fetch('https://chatgpt.com/_next/static/chunk.js');
  await window.fetch('https://example.com/backend-api/conversations');
  return { events, requests, window };
}

(async () => {
  const disabled = await run(false);
  assert.equal(disabled.events.length, 0);
  assert.equal(disabled.window.__elonChatGptPrivateResearchProbe, undefined);

  const enabled = await run(true);
  assert.equal(enabled.requests.length, 6);
  assert.equal(enabled.events.length, 10);
  assert.equal(enabled.window.__elonChatGptPrivateResearchProbe.version, 3);
  assert.equal(
    enabled.window.__elonChatGptPrivateResearchProbe
      .copyRequestContext('conversation_detail').Authorization,
    'must-not-be-observed'
  );
  assert.match(enabled.events[0].detail, /^v1\|fetch\|GET\|\/backend-api\/conversations\|200\|json\|\d+$/);
  assert.match(enabled.events[1].detail, /^v1\|headers\|\/backend-api\/conversations\/private-chat-id\|authorization$/);
  assert.match(enabled.events[2].detail, /^v1\|fetch\|GET\|\/backend-api\/conversations\/private-chat-id\|200\|json\|\d+$/);
  assert.match(enabled.events[3].detail, /^v1\|private_prefetch\|GET\|\/backend-api\/conversations\/private-chat-id\|200\|json\|\d+$/);
  assert.match(enabled.events[4].detail, /^v1\|fetch\|POST\|\/backend-api\/conversation\/\{id\}\|200\|json\|\d+$/);
  assert.match(enabled.events[5].detail, /^v1\|headers\|\/backend-api\/f\/conversation\|content-type\.openai-sentinel-proof-token$/);
  assert.match(enabled.events[6].detail, /^v1\|body\|\/backend-api\/f\/conversation\|action\.messages\.model$/);
  assert.match(enabled.events[7].detail, /^v1\|message\|\/backend-api\/f\/conversation\|author\.content\.id$/);
  assert.match(enabled.events[8].detail, /^v1\|content\|\/backend-api\/f\/conversation\|content_type\.parts$/);
  assert.match(enabled.events[9].detail, /^v1\|fetch\|POST\|\/backend-api\/f\/conversation\|200\|json\|\d+$/);
  const emitted = JSON.stringify(enabled.events);
  assert.doesNotMatch(emitted, /offset|limit|must-not-be-observed|123e4567|\bnext\b/);
  assert.ok(enabled.events.every((event) => event.action === 'research_network_observation'));
  console.log('CHATGPT_WEB_PRIVATE_RESEARCH_PROBE_TESTS=passed');
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

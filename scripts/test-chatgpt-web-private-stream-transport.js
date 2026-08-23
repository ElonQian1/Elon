'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const asset = (name) => fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'assets', name
), 'utf8');
const policySource = asset('chatgpt_web_private_stream_policy.js');
const transportSource = asset('chatgpt_web_private_stream_transport.js');
const buildGradle = fs.readFileSync(path.join(__dirname, '..', 'android', 'app', 'build.gradle'), 'utf8');
const pageAdapter = fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'kotlin',
  'com', 'elon', 'app', 'chatgptweb', 'ChatGptWebPageAdapter.kt'
), 'utf8');

assert.match(
  buildGradle,
  /findProperty\("ELON_CHATGPT_PRIVATE_STREAM_OBSERVER"\)[\s\S]*?\?\.toBoolean\(\) \?: false/
);
assert.match(buildGradle, /buildConfigField "boolean", "CHATGPT_PRIVATE_STREAM_OBSERVER_ENABLED"/);
assert.match(pageAdapter, /BuildConfig\.CHATGPT_PRIVATE_STREAM_OBSERVER_ENABLED/);
assert.ok(
  pageAdapter.indexOf('chatgpt_web_private_stream_policy.js') <
  pageAdapter.indexOf('chatgpt_web_private_stream_transport.js')
);
assert.ok(
  pageAdapter.indexOf('chatgpt_web_private_stream_transport.js') <
  pageAdapter.indexOf('chatgpt_web_adapter.js')
);

const tick = () => new Promise((resolve) => setImmediate(resolve));

function createResponse(chunks) {
  const encoded = chunks.map((value) => new TextEncoder().encode(value));
  return {
    ok: true,
    status: 200,
    headers: { get: (name) => name === 'content-type' ? 'text/event-stream; charset=utf-8' : null },
    clone: () => ({
      body: {
        getReader: () => {
          let index = 0;
          return {
            read: async () => index < encoded.length
              ? { done: false, value: encoded[index++] }
              : { done: true },
            releaseLock: () => {}
          };
        }
      }
    })
  };
}

function context(enabled, response) {
  let calls = 0;
  const outcomes = [];
  const shapes = [];
  const originalFetch = async () => {
    calls += 1;
    return response;
  };
  const window = {
    __elonChatGptPrivateStreamObserverEnabled: enabled,
    __elonChatGptPrivateResearchProbe: {
      recordPrivateStreamOutcome: (outcome, frames, elapsedMs) =>
        outcomes.push({ outcome, frames, elapsedMs }),
      recordPrivateStreamShape: (shape) => shapes.push(shape)
    },
    fetch: originalFetch
  };
  window.window = window;
  const sandbox = {
    window,
    location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/conversation-one' },
    URL,
    Promise,
    Date,
    JSON,
    TextDecoder,
    Set,
    Object,
    String,
    Number,
    Array,
    RegExp
  };
  vm.runInNewContext(policySource, sandbox, { filename: 'chatgpt_web_private_stream_policy.js' });
  vm.runInNewContext(transportSource, sandbox, { filename: 'chatgpt_web_private_stream_transport.js' });
  return { window, originalFetch, calls: () => calls, outcomes, shapes };
}

(async () => {
  const response = createResponse([
    'data: {"conversation_id":"conversation-one","message":{"id":"assistant-one",',
    '"author":{"role":"assistant"},"status":"in_progress","content":{"parts":["hello"]}}}\n\n',
    'data: {"conversation_id":"conversation-one","message":{"id":"assistant-one",',
    '"author":{"role":"assistant"},"status":"finished_successfully","content":{"parts":["hello world"]}}}\n\n',
    'data: [DONE]\n\n'
  ]);
  const enabled = context(true, response);
  assert.equal(enabled.window.__elonChatGptPrivateStreamTransport.version, 2);
  let notifications = 0;
  enabled.window.__elonChatGptPrivateStreamTransport.subscribe(() => { notifications += 1; });

  const request = { method: 'POST', url: 'https://chatgpt.com/backend-api/f/conversation' };
  const init = { method: 'POST' };
  Object.defineProperty(init, 'headers', { get: () => { throw new Error('headers must not be read'); } });
  Object.defineProperty(init, 'body', { get: () => { throw new Error('body must not be read'); } });
  const returned = await enabled.window.fetch(request, init);
  await tick();
  await tick();
  assert.equal(returned, response);
  assert.equal(enabled.calls(), 1);
  assert.ok(notifications >= 2);
  assert.deepEqual(enabled.outcomes.map((item) => [item.outcome, item.frames]), [
    ['first', 1],
    ['success', 2]
  ]);
  assert.deepEqual(enabled.shapes, [
    't:none/k:conversation_id.message/dt:none/dk:none/mk:author.content.id.status/ck:parts',
    't:none/k:conversation_id.message/dt:none/dk:none/mk:author.content.id.status/ck:parts'
  ]);
  assert.equal(
    enabled.window.__elonChatGptPrivateStreamTransport.current('/c/conversation-one').text,
    'hello world'
  );
  const merged = enabled.window.__elonChatGptPrivateStreamTransport.mergeMessages([], '/c/conversation-one');
  assert.equal(merged.length, 1);
  assert.equal(merged[0].state, 'completed');
  assert.equal(merged[0].content[0].text, 'hello world');

  await enabled.window.fetch({
    method: 'POST',
    url: 'https://chatgpt.com/backend-api/f/conversation/stream'
  }, { method: 'POST' });
  await tick();
  assert.equal(enabled.calls(), 2, 'versioned stream paths use the same single official request');

  await enabled.window.fetch({
    method: 'POST',
    url: 'https://chatgpt.com/backend-anon/conversation'
  }, { method: 'POST' });
  await tick();
  assert.equal(enabled.calls(), 3, 'guest conversation streams are observed without request replay');

  enabled.window.__elonChatGptPrivateStreamTransport.dispose();
  assert.equal(enabled.window.fetch, enabled.originalFetch);

  const disabled = context(false, response);
  assert.equal(disabled.window.__elonChatGptPrivateStreamTransport, undefined);
  assert.equal(disabled.window.fetch, disabled.originalFetch);

  console.log('CHATGPT_WEB_PRIVATE_STREAM_TRANSPORT_TESTS=passed');
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

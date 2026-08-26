'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const asset = (name) => fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'assets', name
), 'utf8');
const fetchTapSource = asset('chatgpt_web_private_fetch_tap.js');
const policySource = asset('chatgpt_web_private_stream_policy.js');
const transportSource = asset('chatgpt_web_private_stream_transport.js');
const pageAdapter = fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'kotlin',
  'com', 'elon', 'app', 'chatgptweb', 'ChatGptWebPageAdapter.kt'
), 'utf8');

assert.match(pageAdapter, /PRIVATE_FETCH_TAP_ASSET/);
assert.match(pageAdapter, /chatgpt_web_private_fetch_tap\.js/);
assert.ok(pageAdapter.indexOf('PRIVATE_FETCH_TAP_ASSET') < pageAdapter.indexOf('PRIVATE_SOCKET_TAP_ASSET'));

const tick = () => new Promise((resolve) => setImmediate(resolve));
function responseFor(chunks) {
  return {
    ok: true,
    status: 200,
    headers: { get: (name) => name === 'content-type' ? 'text/event-stream' : null },
    clone: () => responseFor(chunks),
    body: {
      getReader: () => {
        const encoded = chunks.map((value) => new TextEncoder().encode(value));
        let index = 0;
        return {
          read: async () => index < encoded.length
            ? { done: false, value: encoded[index++] }
            : { done: true },
          releaseLock: () => {}
        };
      }
    }
  };
}

(async () => {
  let calls = 0;
  const response = responseFor([
    'data: {"conversation_id":"early-fetch","message":{"id":"assistant-one",',
    '"author":{"role":"assistant"},"status":"in_progress","content":{"parts":["hello"]}}}\n\n',
    'data: {"conversation_id":"early-fetch","message":{"id":"assistant-one",',
    '"author":{"role":"assistant"},"status":"finished_successfully",',
    '"content":{"parts":["hello from early fetch"]}}}\n\ndata: [DONE]\n\n'
  ]);
  const originalFetch = async () => { calls += 1; return response; };
  const window = {
    __elonChatGptPrivateStreamObserverEnabled: true,
    fetch: originalFetch
  };
  window.window = window;
  const sandbox = {
    window,
    location: {
      origin: 'https://chatgpt.com',
      href: 'https://chatgpt.com/c/early-fetch',
      pathname: '/c/early-fetch'
    },
    URL, Promise, Date, JSON, TextDecoder, Set, Object, String, Number, Array, RegExp
  };

  vm.runInNewContext(fetchTapSource, sandbox, { filename: 'chatgpt_web_private_fetch_tap.js' });
  const reactCapturedFetch = window.fetch;
  vm.runInNewContext(policySource, sandbox, { filename: 'chatgpt_web_private_stream_policy.js' });
  vm.runInNewContext(transportSource, sandbox, { filename: 'chatgpt_web_private_stream_transport.js' });
  assert.equal(window.__elonChatGptPrivateFetchTap.version, 1);
  assert.equal(window.__elonChatGptPrivateStreamTransport.version, 11);

  const request = { method: 'POST', url: 'https://chatgpt.com/backend-api/f/conversation' };
  const init = { method: 'POST' };
  Object.defineProperty(init, 'headers', { get: () => { throw new Error('headers must not be read'); } });
  Object.defineProperty(init, 'body', { get: () => { throw new Error('body must not be read'); } });
  const returned = await reactCapturedFetch(request, init);
  for (let index = 0; index < 8; index += 1) await tick();
  assert.equal(returned, response, 'the tap never replaces the official response');
  assert.equal(calls, 1, 'the tap never replays the official request');
  assert.equal(
    window.__elonChatGptPrivateStreamTransport.current('/c/early-fetch').text,
    'hello from early fetch',
    'a fetch reference captured before adapter injection remains observable'
  );

  await reactCapturedFetch('https://chatgpt.com/backend-api/accounts/check', { method: 'GET' });
  assert.equal(calls, 2, 'unrelated requests still pass through exactly once');
  window.__elonChatGptPrivateStreamTransport.dispose();
  window.__elonChatGptPrivateFetchTap.dispose();
  assert.equal(window.fetch, originalFetch);
  console.log('CHATGPT_WEB_PRIVATE_FETCH_TAP_TESTS=passed');
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

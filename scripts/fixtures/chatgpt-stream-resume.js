'use strict';
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const tick = async () => { for (let i = 0; i < 8; i++) await new Promise(setImmediate); };
const cid = '11111111-1111-4111-8111-111111111111';
const other = '22222222-2222-4222-8222-222222222222';
const token = 'doc_test_resume';
const message = (text, conversationId = cid) => ({ conversation_id: conversationId,
  message: { id: 'assistant-resume', author: { role: 'assistant' }, status: 'in_progress',
    content: { content_type: 'text', parts: [text] } } });
const event = (value, name = '') => (name ? `event: ${name}\n` : '') +
  'data: ' + JSON.stringify(value) + '\n\n';
const marker = event('v1', 'delta_encoding');
const baseline = marker + event({ v: message('hello') }, 'delta');
const append = event({ p: '/message/content/parts/0', o: 'append', v: ' world' }, 'delta');

function response(chunks, failure = false, status = 200) {
  let reads = 0;
  const value = { ok: status === 200, status,
    headers: { get: key => key === 'content-type' ? 'text/event-stream' : null },
    clone: () => response(chunks, failure, status),
    body: { getReader: () => {
      let index = 0;
      return { read: async () => {
        reads++;
        if (index < chunks.length) return { done: false, value: new TextEncoder().encode(chunks[index++]) };
        if (failure) throw Error('synthetic_disconnect');
        return { done: true };
      }, cancel: async () => {}, releaseLock() {} };
    } }, reads: () => reads };
  return value;
}

function fixture() {
  const calls = [], outcomes = [];
  let next = response([]), delay = null;
  const window = { __elonChatGptPrivateStreamObserverEnabled: true,
    __elonChatGptDocumentToken: token,
    __elonChatGptPrivateResearchProbe: { recordPrivateStreamOutcome: value => outcomes.push(value),
      recordPrivateStreamShape() {} },
    fetch: (input, init) => {
      calls.push({ input, init });
      const pending = delay; delay = null;
      return pending || Promise.resolve(next);
    } };
  window.window = window;
  const location = { origin: 'https://chatgpt.com', pathname: '/c/' + cid,
    href: 'https://chatgpt.com/c/' + cid };
  const sandbox = { window, location, URL, Promise, Date, JSON, TextDecoder,
    Set, Object, String, Number, Array, RegExp };
  const root = path.join(__dirname, '../../android/app/src/main/assets');
  const run = name => vm.runInNewContext(fs.readFileSync(path.join(root, name), 'utf8'), sandbox, { filename: name });
  run('chatgpt_web_private_fetch_tap.js');
  const officialFetch = window.fetch;
  run('chatgpt_web_private_delta_document.js');
  run('chatgpt_web_private_stream_policy.js');
  run('chatgpt_web_private_stream_transport.js');
  return { window, location, calls, outcomes, api: window.__elonChatGptPrivateStreamTransport,
    async send(chunks, { resume = false, offset = 2, conversationId = cid, failure = false,
      body, origin = location.origin, suffix, status = 200 } = {}) {
      next = response(chunks, failure, status);
      const init = { method: 'POST' };
      Object.defineProperty(init, 'headers', { get: () => { throw Error('headers_must_not_be_read'); } });
      if (resume) init.body = body === undefined ? JSON.stringify({ conversation_id: conversationId, offset }) : body;
      else Object.defineProperty(init, 'body', { get: () => { throw Error('send_body_must_not_be_read'); } });
      const result = await officialFetch(origin + '/backend-api/f/conversation' + (suffix ?? (resume ? '/resume' : '')), init);
      await tick(); return result;
    },
    defer() { let resolve; delay = new Promise(done => { resolve = done; });
      return chunks => { const value = response(chunks); delay = null; resolve(value); }; },
    current() { return this.api.current(location.pathname); }
  };
}
module.exports = { fixture, tick, cid, other, token, message, event, marker, baseline, append };

'use strict';

const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const { randomUUID } = require('node:crypto');
const vm = require('node:vm');
const { test } = require('node:test');

const assets = join(__dirname, '..', 'android/app/src/main/assets');
const sources = ['policy', 'relay'].map(name => readFileSync(
  join(assets, `chatgpt_web_private_text_transaction_${name}.js`), 'utf8'
));
const body = {
  action: 'next', parent_message_id: 'parent-message-one', conversation_id: 'conversation-one',
  messages: [{ id: 'user-message-one', author: { role: 'user' },
    content: { content_type: 'text', parts: ['synthetic prompt'] } }]
};
const completed = { id: 'assistant-message-one', conversationId: 'conversation-one', state: 'completed' };
const response = () => new Response('', { headers: { 'content-type': 'text/event-stream' } });
const deferred = () => {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
};
async function until(predicate) {
  for (let i = 0; i < 50; i++) {
    if (predicate()) return;
    await new Promise(resolve => setTimeout(resolve, 5));
  }
  assert.ok(predicate(), 'bounded fixture did not settle');
}

function fixture(t, options = {}) {
  const timers = new Map();
  const calls = [];
  let timerId = 0;
  class CapturedRequest extends Request {
    clone() { return new CapturedRequest(super.clone()); }
    async text() {
      const text = await super.text();
      return options.read ? options.read(text) : text;
    }
  }
  const location = { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/conversation-one',
    pathname: '/c/conversation-one' };
  const window = {
    __elonChatGptPrivateTextTransactionsEnabled: true,
    crypto: { randomUUID },
    setTimeout(callback, delay) { timers.set(++timerId, { callback, delay }); return timerId; },
    clearTimeout(id) { timers.delete(id); },
    fetch(input, init) {
      calls.push({ input, init });
      return init.__elonPrivateTransport ? options.fetch?.(input, init) ?? Promise.resolve(response())
        : Promise.resolve(response());
    }
  };
  const context = { window, location, URL, Request: CapturedRequest, AbortController, Uint8Array };
  sources.forEach(source => vm.runInNewContext(source, context));
  const relay = window.__elonChatGptPrivateTextTransactionRelay;
  t.after(() => relay.dispose());
  const post = (value = body, headers = {}) => window.fetch('/backend-api/f/conversation', {
    method: 'POST', headers, body: typeof value === 'string' ? value : JSON.stringify(value)
  });
  return { relay, location, calls, timers, post,
    async seed() {
      await post();
      await until(() => relay.state().state === 'stream_not_confirmed');
      assert.equal(relay.observeStream(completed), true);
      assert.equal(relay.state().state, 'ready');
    },
    send() { return relay.dispatch({ prompt: 'synthetic followup', requestId: 'mcp_test1' }); },
    fireTimer() {
      const [id, timer] = timers.entries().next().value;
      timers.delete(id);
      timer.callback();
    }
  };
}

test('current prepare proof and future sentinel headers are never reusable', async t => {
  for (const name of ['openai-sentinel-chat-requirements-prepare-token', 'OpenAI-Sentinel-Future-Proof']) {
    const f = fixture(t);
    await f.seed();
    await f.post(body, { [name]: 'synthetic-one-use-value' });
    assert.equal(f.relay.state().state, 'capture_dynamic_proof');
    assert.equal(f.send().dispatched, false);
    assert.equal(f.relay.state().regenerateReady, false);
  }
});

test('a delayed clone cannot restore a template after context invalidation', async t => {
  const read = deferred();
  const f = fixture(t, { read: () => read.promise });
  await f.post();
  f.relay.invalidateContext();
  read.resolve(JSON.stringify(body));
  await new Promise(resolve => setTimeout(resolve, 25));
  assert.equal(f.relay.state().state, 'template_unavailable');
  assert.equal(f.send().dispatched, false);
});

test('a delayed older clone cannot replace a newer protected request', async t => {
  const read = deferred();
  const f = fixture(t, { read: () => read.promise });
  await f.post();
  await f.post(body, { 'openai-sentinel-proof-token': 'synthetic-proof' });
  read.resolve(JSON.stringify(body));
  await new Promise(resolve => setTimeout(resolve, 25));
  assert.equal(f.relay.state().state, 'capture_dynamic_proof');
  assert.equal(f.send().dispatched, false);
});

test('a new official request revokes the previous ready template before parsing', async t => {
  let held = false;
  const read = deferred();
  const f = fixture(t, { read: text => held ? read.promise : text });
  await f.seed();
  held = true;
  await f.post('invalid json');
  assert.equal(f.relay.state().state, 'capture_pending');
  assert.equal(f.send().dispatched, false);
  read.resolve('invalid json');
  await until(() => !['capture_pending', 'template_unavailable'].includes(f.relay.state().state));
  assert.equal(f.relay.state().state, 'capture_invalid_body');
});

test('a completed receipt before capture cannot authorize a later request', async t => {
  const f = fixture(t);
  assert.equal(f.relay.observeStream(completed), false);
  await f.post();
  await until(() => !['capture_pending', 'template_unavailable'].includes(f.relay.state().state));
  assert.equal(f.send().dispatched, false);
});

test('a receipt cannot retarget an existing template to another conversation', async t => {
  const f = fixture(t);
  await f.seed();
  f.location.pathname = '/c/conversation-two';
  assert.equal(f.relay.observeStream({ ...completed, conversationId: 'conversation-two' }), false);
  assert.equal(f.send().dispatched, false);
});

test('a stale previous assistant cannot settle the newly dispatched turn', async t => {
  const f = fixture(t);
  await f.seed();
  await f.send().completion;
  assert.equal(f.relay.observeStream(completed), false);
  assert.equal(f.relay.state().state, 'busy');
  assert.equal(f.relay.observeStream({ ...completed, id: 'assistant-message-two' }), true);
  assert.equal(f.relay.state().state, 'ready');
  assert.equal(f.timers.size, 0);
});

test('response headers replace the short deadline with a bounded stream deadline', async t => {
  const result = deferred();
  const f = fixture(t, { fetch: () => result.promise });
  await f.seed();
  const tx = f.send();
  assert.equal([...f.timers.values()][0].delay, 15000);
  result.resolve(response());
  assert.equal((await tx.completion).status, 'accepted');
  assert.equal(f.timers.size, 1);
  assert.equal([...f.timers.values()][0].delay, 600000);
  assert.equal(f.calls.at(-1).init.signal.aborted, false);
  f.fireTimer();
  assert.equal(f.calls.at(-1).init.signal.aborted, true);
  assert.equal(f.relay.state().active, false);
  assert.equal(f.send().dispatched, false);
});

test('late response headers cannot turn a timed out write into accepted', async t => {
  const result = deferred();
  const f = fixture(t, { fetch: () => result.promise });
  await f.seed();
  const tx = f.send();
  f.fireTimer();
  result.resolve(response());
  const done = await tx.completion;
  assert.equal(done.status, 'unknown');
  assert.equal(done.code, 'timeout');
  assert.equal(f.relay.state().failures, 1);
  assert.equal(f.timers.size, 0);
});

test('a fetch wrapper throwing after invocation cannot authorize official replay', async t => {
  const f = fixture(t, { fetch: () => { throw new Error('synthetic wrapper failure'); } });
  await f.seed();
  const tx = f.send();
  assert.equal(tx.dispatched, true);
  const done = await tx.completion;
  assert.equal(done.status, 'unknown');
  assert.equal(done.code, 'synchronous_failure');
  assert.equal(f.calls.length, 2);
  assert.equal(f.calls.at(-1).init.signal.aborted, true);
  assert.equal(f.send().dispatched, false);
  assert.equal(f.timers.size, 0);
});

test('late rejection from a stopped transaction does not penalize a newer turn', async t => {
  const result = deferred();
  const f = fixture(t, { fetch: () => result.promise });
  await f.seed();
  const tx = f.send();
  f.relay.stop('mcp_test1');
  await f.seed();
  result.reject(new Error('late old failure'));
  assert.equal((await tx.completion).status, 'unknown');
  assert.equal(f.relay.state().state, 'ready');
  assert.equal(f.relay.state().failures, 0);
});

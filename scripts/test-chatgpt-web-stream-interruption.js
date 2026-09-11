'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const tick = () => new Promise(resolve => setImmediate(resolve));

function fixture(fetchResponse) {
  const reads = [], outcomes = [];
  let released = 0, calls = 0, cancelled = 0;
  const page = {
    __elonChatGptPrivateStreamObserverEnabled: true,
    __elonChatGptPrivateResearchProbe: {
      recordPrivateStreamOutcome: outcome => outcomes.push(outcome)
    },
    fetch: async () => {
      calls++;
      if (fetchResponse) return fetchResponse();
      return { ok: true, status: 200, headers: { get: () => 'text/event-stream' },
        clone: () => ({ body: { getReader: () => ({
          read: () => new Promise((resolve, reject) => reads.push({ resolve, reject })),
          cancel: () => { cancelled++; return Promise.resolve(); },
          releaseLock: () => { released++; }
        }) } }) };
    }
  };
  const location = { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/thread-one',
    pathname: '/c/thread-one' };
  const sandbox = { window: page, location, URL, TextDecoder };
  for (const name of ['delta_document', 'stream_policy', 'stream_transport']) vm.runInNewContext(fs.readFileSync(path.join(
    __dirname, '../android/app/src/main/assets/chatgpt_web_private_' + name + '.js'
  ), 'utf8'), sandbox);
  const api = page.__elonChatGptPrivateStreamTransport;
  return { api, page, location, reads, outcomes, calls: () => calls, released: () => released,
    cancelled: () => cancelled,
    async start() { return page.fetch('https://chatgpt.com/backend-api/f/conversation', { method: 'POST' }); },
    async frame(text, messageId = 'reply-one', conversationId = 'thread-one', index = 0) {
      const payload = { conversation_id: conversationId, message: { id: messageId,
        author: { role: 'assistant' }, status: 'in_progress', content: { parts: [text] } } };
      reads.splice(index, 1)[0].resolve({ done: false, value: new TextEncoder().encode('data: ' + JSON.stringify(payload) + '\n\n') });
      await tick();
    },
    async fail(name = 'AbortError', index = 0) {
      const error = new Error('synthetic stream interruption'); error.name = name;
      reads.splice(index, 1)[0].reject(error); await tick();
    }
  };
}

for (const error of ['AbortError', 'TypeError']) test(error + ' retains the received reply without reporting stream success', async () => {
  const f = fixture(); await f.start(); await f.frame('Partial answer');
  assert.equal(f.api.current(f.location.pathname).state, 'streaming');
  await f.fail(error);
  const current = f.api.current(f.location.pathname);
  assert.ok(current, 'an interrupted reader must not erase the visible reply');
  assert.equal(current.text, 'Partial answer');
  assert.equal(current.state, 'streaming', 'a disconnected reader is not evidence of a completed answer');
  const merged = f.api.mergeMessages([{ id: 'user-one', role: 'user', content: [] }], f.location.pathname);
  assert.equal(merged.length, 2);
  assert.equal(merged[1].content[0].text, 'Partial answer');
  assert.deepEqual(f.outcomes, ['first', 'error']);
  assert.equal(f.calls(), 1, 'an interrupted write is never replayed');
  assert.equal(f.released(), 1);
});

test('failure before the first reply leaves no phantom message or streaming placeholder', async () => {
  const f = fixture(); await f.start(); await f.fail();
  assert.equal(f.api.current(f.location.pathname), null);
  assert.equal(f.api.mergeMessages([], f.location.pathname).length, 0);
  assert.deepEqual(f.outcomes, ['error']);
});

test('a stale reader failure cannot finalize the next request', async () => {
  const f = fixture(); await f.start(); await f.frame('Old partial');
  f.api.prepareSend(); await f.start();
  await f.fail('AbortError', 0);
  await f.frame('New partial', 'reply-two');
  assert.equal(f.api.current(f.location.pathname).text, 'New partial');
  assert.equal(f.api.current(f.location.pathname).state, 'streaming');
  await f.fail();
});

test('resetting the conversation drops old text before the old reader fails', async () => {
  const f = fixture(); await f.start(); await f.frame('Old partial');
  f.api.reset(); f.location.pathname = '/c/thread-two';
  await f.fail();
  assert.equal(f.api.current(f.location.pathname), null);
  assert.equal(f.api.mergeMessages([], f.location.pathname).length, 0);
});

test('disposing the observer cannot resurrect an interrupted reply', async () => {
  const f = fixture(); await f.start(); await f.frame('Old partial');
  f.api.dispose(); await f.fail();
  assert.equal(f.api.current(f.location.pathname), null);
  assert.equal(f.released(), 1);
});

test('official final text still replaces the retained partial instead of duplicating it', async () => {
  const f = fixture(); await f.start(); await f.frame('Partial'); await f.fail();
  const merged = f.api.mergeMessages([{ id: 'reply-one', role: 'assistant',
    content: [{ type: 'markdown', text: 'Partial answer from the official page' }] }], f.location.pathname);
  assert.equal(merged.length, 1);
  assert.equal(merged[0].content[0].text, 'Partial answer from the official page');
});

for (const lateEvent of ['error', 'end', 'chunk']) test('superseded same-conversation reader ignores late ' + lateEvent, async () => {
  const f = fixture(); await f.start(); await f.frame('Old partial');
  await f.start(); await f.frame('New partial', 'reply-two', 'thread-one', 1);
  if (lateEvent === 'error') await f.fail();
  else if (lateEvent === 'chunk') await f.frame('Old continuation');
  else { f.reads.shift().resolve({ done: true }); await tick(); }
  assert.equal(f.api.current(f.location.pathname).text, 'New partial');
  assert.equal(f.api.current(f.location.pathname).state, 'streaming');
  assert.deepEqual(f.outcomes, ['first', 'first'], 'retired readers cannot publish a current outcome');
  assert.equal(f.cancelled(), 1, 'only the old observer clone is cancelled');
  assert.equal(f.calls(), 2, 'the observer never starts another fetch');
  await f.fail();
});

for (const boundary of ['prepareSend', 'reset', 'dispose']) test(boundary + ' retires pending clone before a late chunk arrives', async () => {
  const f = fixture(); await f.start(); await f.frame('Old partial');
  f.api[boundary]();
  await f.frame('Late continuation');
  assert.equal(f.api.current(f.location.pathname), null);
  assert.equal(f.cancelled(), 1);
  assert.equal(f.released(), 1);
  assert.deepEqual(f.outcomes, ['first']);
});

test('disposing while official fetch is pending never acquires a late observer reader', async () => {
  const f = fixture(); const pending = f.start();
  f.api.dispose(); await pending; await tick();
  assert.equal(f.released(), 0, 'a disposed observer must not acquire another reader');
  assert.equal(f.calls(), 1);
  assert.equal(f.api.current(f.location.pathname), null);
});

test('retiring a real cloned response does not cancel or consume the official response', async () => {
  let controller, upstreamCancelled = 0;
  const body = new ReadableStream({
    start(value) { controller = value; },
    cancel() { upstreamCancelled++; }
  });
  const response = new Response(body, { headers: { 'content-type': 'text/event-stream' } });
  const f = fixture(() => response);
  const original = await f.start();
  const nativeReader = original.body.getReader();
  const first = new TextEncoder().encode('data: {"conversation_id":"thread-one","message":{"id":"reply-one","author":{"role":"assistant"},"content":{"parts":["Before stop"]}}}\n\n');
  controller.enqueue(first);
  assert.deepEqual((await nativeReader.read()).value, first);
  for (let attempt = 0; attempt < 20 && !f.api.current(f.location.pathname); attempt++) await tick();
  assert.equal(f.api.current(f.location.pathname).text, 'Before stop');
  f.api.dispose();
  const after = new TextEncoder().encode('data: [DONE]\n\n');
  controller.enqueue(after); controller.close();
  assert.deepEqual((await nativeReader.read()).value, after);
  assert.equal((await nativeReader.read()).done, true);
  await tick();
  assert.equal(upstreamCancelled, 0);
  assert.equal(original, response);
  assert.equal(f.api.current(f.location.pathname), null);
  nativeReader.releaseLock();
});

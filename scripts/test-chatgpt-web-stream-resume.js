'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const { fixture, cid, other, message, event, baseline, append } = require('./fixtures/chatgpt-stream-resume');

for (const failure of [false, true]) test(`interrupted stream is not a completed answer (throw=${failure})`, async () => {
  const f = fixture(); await f.send([baseline], { failure });
  assert.equal(f.current().text, 'hello');
  assert.equal(f.current().state, 'streaming');
  assert.equal(f.outcomes.includes('success'), false);
});

test('official resume retains delta state and completes exactly one existing answer', async () => {
  const f = fixture(); await f.send([baseline], { failure: true });
  const resumed = await f.send([append, 'data: [DONE]\n\n'], { resume: true });
  assert.equal(resumed.status, 200);
  assert.equal(f.current().text, 'hello world');
  assert.equal(f.current().state, 'completed');
  assert.equal(f.calls.length, 2, 'only the two simulated official requests ran');
  assert.equal(f.outcomes.filter(value => value === 'success').length, 1);
});

test('resume offsets count encoding and token events but exclude heartbeats', async () => {
  const f = fixture();
  await f.send([event({ type: 'resume_conversation_token', conversation_id: cid, token: 'synthetic-only' }),
    'event: ping\ndata: alive\n\n', baseline]);
  await f.send([append, 'data: [DONE]\n\n'], { resume: true, offset: 3 });
  assert.equal(f.current().text, 'hello world');
});

test('partial interrupted SSE bytes are dropped while complete delta state is retained', async () => {
  const f = fixture(); await f.send([baseline, 'event: delta\ndata: {"p":'], { failure: true });
  await f.send([append, 'data: [DONE]\n\n'], { resume: true });
  assert.equal(f.current().text, 'hello world');
});

for (const args of [
  { offset: 0 }, { offset: 1 }, { offset: 3 }, { offset: -1 },
  { offset: 2.5 }, { offset: '2' }, { conversationId: other },
  { body: '{' }, { body: JSON.stringify({ conversation_id: cid, offset: 2, extra: true }) },
  { body: ' '.repeat(1025) }, { body: new Uint8Array([1, 2]) },
  { origin: 'https://untrusted.invalid' }, { suffix: '/prepare' }, { status: 403 }
]) test(`unmatched resume cannot replace visible content ${JSON.stringify(args).slice(0,100)}`, async () => {
  const f = fixture(); await f.send([baseline]);
  await f.send([event(message('wrong')), 'data: [DONE]\n\n'], { resume: true, ...args });
  assert.equal(f.current().text, 'hello');
  assert.equal(f.current().state, 'streaming');
  assert.equal(f.calls.length, 2);
});

test('a fresh navigation cannot consume an old resume response', async () => {
  const f = fixture(); await f.send([baseline]);
  const resolve = f.defer(); const pending = f.send([], { resume: true });
  f.window.__elonChatGptDocumentToken = 'doc_other_page';
  resolve([append, 'data: [DONE]\n\n']); await pending;
  assert.equal(f.current().text, 'hello');
});

test('a new turn invalidates an outstanding resume response', async () => {
  const f = fixture(); await f.send([baseline]);
  const resolve = f.defer(); const pending = f.send([], { resume: true });
  f.api.prepareSend();
  resolve([event(message('wrong')), 'data: [DONE]\n\n']); await pending;
  assert.equal(f.current(), null);
});

test('an old resume cannot attach to a new send with the same conversation and offset', async () => {
  const f = fixture(); await f.send([baseline]);
  const resolve = f.defer(); const pending = f.send([], { resume: true });
  f.api.prepareSend();
  await f.send([baseline.replace('hello', 'new turn')]);
  resolve([append, 'data: [DONE]\n\n']); await pending;
  assert.equal(f.current().text, 'new turn');
  assert.equal(f.current().state, 'streaming');
});

test('multiple official resumptions append without duplicating prior text', async () => {
  const f = fixture(); await f.send([baseline]);
  await f.send([append], { resume: true });
  await f.send([append.replace(' world', '!'), 'data: [DONE]\n\n'], { resume: true, offset: 3 });
  assert.equal(f.current().text, 'hello world!');
  assert.equal(f.outcomes.filter(value => value === 'success').length, 1);
  assert.equal(f.calls.length, 3);
});

test('resume after terminal DONE cannot append twice', async () => {
  const f = fixture(); await f.send([baseline, 'data: [DONE]\n\n']);
  await f.send([append, 'data: [DONE]\n\n'], { resume: true });
  assert.equal(f.current().text, 'hello');
  assert.equal(f.current().state, 'completed');
});

test('a mismatched conversation inside a resume payload is rejected', async () => {
  const f = fixture(); await f.send([baseline]);
  await f.send([event(message('wrong', other)), 'data: [DONE]\n\n'], { resume: true });
  assert.equal(f.current().text, 'hello');
  assert.equal(f.current().state, 'streaming');
});

test('incomplete delta evidence cannot be revived by a later resume', async () => {
  const f = fixture(); await f.send([baseline, 'event: delta\ndata: {"o":"unknown","v":1}\n\n']);
  await f.send([event(message('wrong')), 'data: [DONE]\n\n'], { resume: true, offset: 3 });
  assert.equal(f.current().text, 'hello');
});

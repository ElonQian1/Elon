'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const stream = require('../android/app/src/main/assets/chatgpt_web_fresh_text_stream');
const owned = require('../android/app/src/main/assets/chatgpt_web_private_owned_stream');
const policy = require('../android/app/src/main/assets/chatgpt_web_private_stream_policy');
const CID = 'synthetic-conversation';
const frame = (data, event) => (event ? 'event: ' + event + '\n' : '') + 'data: ' + JSON.stringify(data) + '\n\n';
const handoff = { data: { type: 'stream_handoff', options: [{ type: 'subscribe_ws_topic', topic_id: 'synthetic-topic' }] } };
const pause = () => new Promise(resolve => setImmediate(resolve));

function fixture(options = {}) {
  const controller = new AbortController(), handlers = new Map(), calls = [];
  let current = true;
  const on = key => callback => { handlers.set(key, callback); return () => handlers.delete(key); };
  const emit = payload => handlers.get('message')?.({ type: 'conversation-turn-stream', payload });
  const item = (id, parent, data, event) => emit({ type: 'stream-item', stream_item_id: id,
    parent_stream_item_id: parent, encoded_item: frame(data, event) });
  const topic = {
    state: 2, hasEverSubscribed: false,
    onMessage: on('message'), onPotentialMissedMessages: on('missed'),
    subscribe(value) { calls.push(['subscribe', value]); return options.subscribe?.({ emit, item }); },
    unsubscribe() { calls.push(['unsubscribe']); }
  };
  const api = stream.create({ signal: controller.signal, current: () => current,
    getTopic: id => { assert.equal(id, 'synthetic-topic'); return topic; }, idleTimeoutMs: options.idleTimeoutMs || 1000 });
  let returned = false;
  async function* source() {
    try {
      yield { response: { ok: true } };
      yield { event: 'delta_encoding', data: 'v1' };
      yield handoff;
    } finally { returned = true; }
  }
  return { api, topic, calls, emit, item, handlers, controller,
    stale: () => { current = false; }, iterator: () => api.follow(source()), returned: () => returned };
}
async function atTopic(f) {
  const iterator = f.iterator();
  await iterator.next(); await iterator.next(); await iterator.next();
  const pending = iterator.next(); await pause();
  return { iterator, pending };
}

test('in-band topic retains delta encoding and synchronous history catchups until authoritative done', async () => {
  const f = fixture({ subscribe({ item, emit }) {
    item('a', null, { message: { id: 'synthetic-answer' } });
    item('b', 'a', { type: 'message_stream_complete' }); emit({ type: 'done' });
  } });
  const output = [];
  for await (const value of f.iterator()) output.push(value);
  assert.equal(output.length, 5);
  assert.equal(output[1].event, 'delta_encoding');
  assert.equal(output[3].data.message.id, 'synthetic-answer');
  assert.deepEqual(f.calls, [['subscribe', { includeAllHistory: true }], ['unsubscribe']]);
  assert.equal(f.handlers.size, 0); assert.equal(f.returned(), true);
});

test('root EOF and encoded DONE do not finish a live handed-off topic; duplicate IDs are ignored', async () => {
  const f = fixture(), { iterator, pending } = await atTopic(f);
  let finished = false; pending.then(() => { finished = true; });
  f.emit({ type: 'stream-item', stream_item_id: 'a', encoded_item: 'data: [DONE]\n\n' });
  await pause(); assert.equal(finished, false);
  f.item('b', 'a', { value: 'once' }); f.item('b', 'a', { value: 'duplicate' });
  assert.equal((await pending).value.data.value, 'once');
  f.emit({ type: 'done' }); assert.equal((await iterator.next()).done, true);
  assert.equal(f.returned(), true);
});

for (const reason of ['gap', 'missed', 'abort', 'owner', 'invalid', 'server']) {
  test('handoff rejects ' + reason + ', cleans listeners and never replays a POST', async () => {
    const f = fixture(), { pending } = await atTopic(f);
    const rejected = assert.rejects(pending, /stream_|cancelled|context_changed/);
    if (reason === 'gap') f.item('a', 'missing', {});
    if (reason === 'missed') f.handlers.get('missed')();
    if (reason === 'abort') f.controller.abort();
    if (reason === 'owner') { f.stale(); f.item('a', null, {}); }
    if (reason === 'invalid') f.emit({ type: 'stream-item', stream_item_id: 'a', encoded_item: 'data: invalid' });
    if (reason === 'server') f.item('a', null, { error: 'synthetic server error, never exported' });
    await rejected;
    assert.equal(f.calls.length, 2); assert.equal(f.calls[1][0], 'unsubscribe');
    assert.equal(f.handlers.size, 0); assert.equal(f.returned(), true);
  });
}

test('topic timeout is bounded and unsubscribes only our topic', async () => {
  const f = fixture({ idleTimeoutMs: 20 }), iterator = f.iterator();
  await assert.rejects(async () => { for await (const _ of iterator) {} }, /stream_topic_timeout/);
  assert.equal(f.calls.filter(x => x[0] === 'unsubscribe').length, 1);
});

test('an existing provider topic is neither subscribed nor unsubscribed', async () => {
  const f = fixture(); f.topic.state = 0; f.topic.hasEverSubscribed = true;
  await assert.rejects(async () => { for await (const _ of f.iterator()) {} }, /stream_topic_owned/);
  assert.equal(f.calls.length, 0); assert.equal(f.handlers.size, 0);
});

test('unsupported handoff never guesses a resume endpoint', async () => {
  const f = fixture();
  async function* source() { yield { data: { type: 'stream_handoff', options: [{ type: 'unknown' }] } }; }
  await assert.rejects(async () => { for await (const _ of f.api.follow(source())) {} }, /stream_handoff_unavailable/);
  assert.equal(f.calls.length, 0);
});

function sinkFixture() {
  const session = policy.createSession({ now: Date.now }); let changed = 0, current = true;
  const api = owned.create({ policy, session, notify: () => changed++, conversationId: p => p.conversation_id || '' });
  const sink = api.begin({ conversationId: CID, current: () => current });
  return { api, sink, session, changed: () => changed, stale: () => { current = false; } };
}
const message = text => ({ conversation_id: CID, message: { id: 'synthetic-answer', author: { role: 'assistant' },
  status: 'in_progress', content: { content_type: 'text', parts: [text] } } });

test('owned delivery decodes v1 deltas into the existing native session without a second store', () => {
  const f = sinkFixture();
  f.sink.push({ event: 'delta_encoding', data: 'v1' });
  f.sink.push({ event: 'delta', data: { p: '', o: 'add', v: message('Hello') } });
  f.sink.push({ event: 'delta', data: { p: '/message/content/parts/0', o: 'append', v: ' world' } });
  const snapshot = f.session.current('/c/' + CID);
  assert.equal(snapshot.text, 'Hello world');
  assert.ok(f.changed() >= 2);
  f.sink.finish(); assert.equal(f.session.current('/c/' + CID).state, 'completed');
  assert.equal(f.api.active(), true); f.api.reset(); assert.equal(f.api.active(), false);
});

test('owned delivery rejects foreign conversation, bad encoding and late document events', () => {
  const f = sinkFixture();
  assert.throws(() => f.sink.push({ data: { ...message('foreign'), conversation_id: 'other' } }), /stream_owner_changed/);
  assert.equal(f.changed(), 0);
  const g = sinkFixture(); g.sink.push({ event: 'delta_encoding', data: 'v1' }); g.stale();
  assert.throws(() => g.sink.push({ data: message('late') }), /context_changed/);
  const h = sinkFixture(); assert.throws(() => h.sink.push({ event: 'delta_encoding', data: 'unsupported' }), /stream_decode_failed/);
});

test('native generating state does not wait for DOM, composer or the first assistant text', () => {
  const streaming = require('../android/app/src/main/assets/chatgpt_web_adapter_streaming_policy');
  const document = { querySelector() { throw Error('DOM must not be queried'); } };
  assert.deepEqual(streaming.readState(null, null, document, null, null,
    { privateWriterActive: true, privateStreamState: 'idle' }), { active: true, assistantKey: '' });
});

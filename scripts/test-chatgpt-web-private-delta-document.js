'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');
const path = require('node:path');
const assets = path.join(__dirname, '../android/app/src/main/assets');
const codec = require(path.join(assets, 'chatgpt_web_private_delta_document.js'));
const policy = require(path.join(assets, 'chatgpt_web_private_stream_policy.js'));
const frame = (text = '') => ({ conversation_id: 'delta-fixture', message: {
  id: 'answer', author: { role: 'assistant' }, status: 'in_progress',
  content: { content_type: 'text', parts: [text] }, metadata: {}
} });
const event = (type, data) => `event: ${type}\ndata: ${JSON.stringify(data)}\n\n`;
function apply(decoder, delta) {
  const result = decoder.apply(delta);
  assert.equal(result.ok, true);
  return result.value;
}

test('delta v1 inherits channel, path and operation, never the old value', () => {
  const decoder = codec.create();
  apply(decoder, { c: 13, v: frame('a') });
  apply(decoder, { p: '/message/content/parts/0', o: 'append', v: 'b' });
  assert.equal(apply(decoder, { v: 'c' }).message.content.parts[0], 'abc');
  assert.equal(decoder.apply({}).ok, false);
});

test('interleaved channels keep separate documents and patches never replace the root', () => {
  const decoder = codec.create();
  apply(decoder, { c: 1, v: { label: 'one' } });
  apply(decoder, { c: 13, v: frame('a') });
  apply(decoder, { c: 1, o: 'append', p: '/label', v: '!' });
  const result = apply(decoder, { c: 13, p: '/message/metadata', v: { complete: true } });
  assert.equal(result.message.content.parts[0], 'a');
  assert.equal(result.message.metadata.complete, true);
  assert.equal(apply(decoder, { c: 1, p: '/label', v: '?' }).label, 'one!?');
});

test('patches are relative to their path; the outer operation remains inherited', () => {
  const decoder = codec.create();
  apply(decoder, { v: { message: { parts: ['hello'], obsolete: true } } });
  apply(decoder, { p: '/message', o: 'patch', v: [
    { p: '/parts/0', o: 'append', v: ' world' },
    { p: '/obsolete', o: 'remove' }
  ] });
  const result = apply(decoder, { v: [{ p: '/parts/0', o: 'truncate', v: 5 }] });
  assert.deepEqual(result, { message: { parts: ['hello'] } });
});

test('array add inserts, remove splices, append accepts a scalar and truncate shortens', () => {
  const decoder = codec.create();
  apply(decoder, { v: { items: [1, 3] } });
  apply(decoder, { p: '/items/1', o: 'add', v: 2 });
  apply(decoder, { p: '/items', o: 'append', v: 4 });
  apply(decoder, { p: '/items/0', o: 'remove' });
  assert.deepEqual(apply(decoder, { p: '/items', o: 'truncate', v: 2 }), { items: [2, 3] });
});

test('missing parents, escaped pointers and root replacement match the observed codec', () => {
  const decoder = codec.create();
  apply(decoder, { v: {} });
  assert.deepEqual(apply(decoder, { p: '/a~1b/0/~0key', o: 'add', v: 'x' }),
    { 'a/b': [{ '~key': 'x' }] });
  assert.deepEqual(apply(decoder, { p: '', o: 'replace', v: { type: 'status', status: 'done' } }),
    { type: 'status', status: 'done' });
});

test('source updates and source masks are distinct channels, not message-only paths', () => {
  const decoder = codec.create();
  apply(decoder, { c: 0, v: { type: 'conversation_context_source', item: {
    citation_uuid: 'source-a', type: 'file', deleted: false
  } } });
  apply(decoder, { c: 1, v: { type: 'pca_source_filter_mask', mask: { urls: [] } } });
  const source = apply(decoder, { c: 0, p: '/item/deleted', o: 'replace', v: true });
  assert.equal(source.item.deleted, true);
  assert.equal(source.item.citation_uuid, 'source-a');
  const mask = apply(decoder, { c: 1, p: '/mask/urls', o: 'append', v: ['file://fixture'] });
  assert.deepEqual(mask.mask.urls, ['file://fixture']);
});

test('a patch can remove and recreate its root without retaining a removal sentinel', () => {
  const decoder = codec.create();
  apply(decoder, { v: { old: true } });
  assert.deepEqual(apply(decoder, { o: 'patch', v: [
    { o: 'remove', p: '' }, { o: 'add', p: '/fresh', v: true }
  ] }), { fresh: true });
});

test('snapshots use copy-on-write; rejected batches cannot partially mutate old state', () => {
  const decoder = codec.create();
  const input = { values: ['a'], untouched: { x: 1 } };
  const before = apply(decoder, { v: input });
  input.values[0] = 'outside';
  const after = apply(decoder, { p: '/values/0', o: 'append', v: 'b' });
  assert.equal(before.values[0], 'a');
  assert.equal(after.values[0], 'ab');
  assert.equal(after.untouched, before.untouched, 'unchanged branches are not cloned on each token');
  assert.equal(decoder.apply({ p: '', o: 'patch', v: [
    { p: '/values/0', o: 'append', v: 'c' }, { p: '/constructor/evil', o: 'add', v: true }
  ] }).ok, false);
  assert.equal(after.values[0], 'ab');
  assert.equal(decoder.apply({ v: 'd' }).ok, false, 'lost patches cannot resume ambiguously');
  decoder.reset();
  assert.deepEqual(apply(decoder, { v: { restored: true } }), { restored: true });
});

for (const key of ['__proto__', 'constructor', 'prototype']) {
  test(`unsafe ${key} is rejected in paths and nested values`, () => {
    const decoder = codec.create();
    apply(decoder, { v: {} });
    assert.equal(decoder.apply({ p: `/${key}/polluted`, o: 'add', v: true }).ok, false);
    decoder.reset();
    assert.equal(decoder.apply({ v: JSON.parse(`{"nested":{"${key}":{}}}`) }).ok, false);
    assert.equal({}.polluted, undefined);
  });
}

test('bounded channel count, paths, patch arrays and retained values fail closed', () => {
  for (const delta of [
    { c: -1, v: {} }, { c: 1.5, v: {} }, { c: 1e12, v: {} },
    { p: '/' + 'x/'.repeat(40), v: 1 }, { o: 'unknown', v: {} },
    { o: 'patch', v: Array.from({ length: 129 }, () => ({ o: 'add', p: '/x', v: 1 })) },
    { v: { text: 'x'.repeat(2 * 1024 * 1024 + 1) } }, { p: '/values/99999999', o: 'add', v: 1 }
  ]) assert.equal(codec.create().apply(delta).ok, false);
  const decoder = codec.create();
  for (let i = 0; i < 32; i++) apply(decoder, { c: i, v: {} });
  assert.equal(decoder.apply({ c: 32, v: {} }).ok, false);
});

test('SSE event labels activate delta v1 and stream resolved frames into native session', () => {
  const session = policy.createSession({ now: () => 100 });
  session.begin();
  let done = 0;
  const decoder = policy.createSseDecoder((payload) => session.accept(payload), () => { done++; });
  const wire = 'event: delta_encoding\ndata: v1\n\n' +
    event('delta', { c: 13, v: frame('hello') }) +
    event('delta', { c: 1, o: 'add', p: '', v: { type: 'status', status: 'working' } }) +
    event('delta', { c: 13, o: 'append', p: '/message/content/parts/0', v: ' world' }) +
    event('delta', { v: '!' }) + event('delta', { o: 'replace', p: '/message/status', v: 'finished_successfully' }) +
    'data: [DONE]\n\n';
  for (let i = 0; i < wire.length; i += 7) decoder.push(wire.slice(i, i + 7));
  assert.equal(session.current('/c/delta-fixture').text, 'hello world!');
  assert.equal(session.current('/c/delta-fixture').state, 'completed');
  assert.equal(done, 1);
});

test('unknown encoding, delta before marker and broken JSON cannot be called successful', () => {
  for (const wire of [
    event('delta_encoding', 'v2') + event('delta', { c: 13, v: frame('wrong') }),
    event('delta_encoding', 'v2') + event('message', frame('also wrong')),
    event('delta', { c: 13, v: frame('wrong') }),
    event('delta_encoding', 'v1') + 'event: delta\ndata: {broken\n\n'
  ]) {
    const frames = []; let ending;
    const decoder = policy.createSseDecoder((value) => frames.push(value), (result) => { ending = result; });
    decoder.push(wire + 'data: [DONE]\n\n');
    assert.equal(frames.length, 0);
    assert.equal(ending.error, 'delta_decode');
  }
});

test('raw socket compatibility shares patch logic and does not mix channels', () => {
  const session = policy.createSession({ now: () => 100 });
  session.begin();
  session.accept({ c: 13, v: frame('a') });
  session.accept({ c: 1, v: { type: 'metadata' } });
  session.accept({ c: 13, o: 'append', p: '/message/content/parts/0', v: 'b' });
  assert.equal(session.current('/c/delta-fixture').text, 'ab');
});

test('raw explicit root seeds and inherited replace are supported without losing first state', () => {
  const session = policy.createSession({ now: () => 100 });
  session.accept({ c: 13, p: '', o: 'add', v: frame('a') });
  session.accept({ p: '/message/content/parts/0', o: 'replace', v: 'b' });
  session.accept({ v: 'c' });
  assert.equal(session.current('/c/delta-fixture').text, 'c');
});

test('invalid compact payloads cannot bypass the codec through full-envelope rendering', () => {
  const session = policy.createSession({ now: () => 100 });
  session.accept({ c: 13, v: frame('last good') });
  const bad = frame('must not render');
  bad.message.metadata = JSON.parse('{"__proto__":{"polluted":true}}');
  assert.equal(session.accept({ c: 13, v: bad }), false);
  assert.equal(session.current('/c/delta-fixture').text, 'last good');
  assert.equal(session.accept({ p: '', o: 'replace', v: frame('still ambiguous') }), false);
  session.begin();
  assert.equal(session.accept({ c: 13, v: frame('new turn') }), true);
});

test('retained input budget is bounded even when many root replacements are valid individually', () => {
  const decoder = codec.create();
  const text = 'x'.repeat(524288);
  for (let i = 0; i < 15; i++) apply(decoder, { o: 'add', p: '', v: { text } });
  assert.equal(decoder.apply({ v: { text } }).ok, false);
});

test('oversized SSE frames fail closed rather than discarding a prefix and applying a suffix', () => {
  let ending; const frames = [];
  const decoder = policy.createSseDecoder((value) => frames.push(value), (result) => { ending = result; });
  decoder.push('data: ' + 'x'.repeat(2 * 1024 * 1024));
  decoder.push(event('delta', { c: 1, v: frame('not a new stream') }));
  assert.equal(frames.length, 0);
  assert.equal(ending.error, 'delta_decode');
});

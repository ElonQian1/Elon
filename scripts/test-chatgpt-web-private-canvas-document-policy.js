'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const policy = require('../android/app/src/main/assets/chatgpt_web_private_canvas_document_policy.js');
const { doc, fixture } = require('./fixtures/chatgpt-web-private-canvas-documents.cjs');

test('server codepoint positions convert to native UTF-16 and back without splitting emoji', () => {
  const source = 'A\u{1f600}BC', comments = [{ id: 'synthetic_comment', start: 1, end: 3, content: 'Note' }];
  const value = policy.parse([doc({ content: source, comments })])[0];
  assert.deepEqual(value.comments, [{ id: 'synthetic_comment', start: 1, end: 4, content: 'Note' }]);
  assert.deepEqual(policy.draft(value, value).comments, comments);
  assert.equal(Object.isFrozen(value), true); assert.equal(Object.isFrozen(value.comments[0]), true);
});

test('moving a preserved native comment anchor serializes against the edited text, not the old text', async () => {
  const f = fixture(); Object.assign(f.rows[0], { content: 'AB', comments: [{ id: 'c1', start: 1, end: 2, content: 'Note' }] });
  const read = await f.list(), saved = await f.save(read.ticket, { content: '\u{1f600}AB',
    comments: [{ id: 'c1', start: 3, end: 4, content: 'Note' }] });
  assert.equal(saved.code, 'canvas_saved');
  const body = JSON.parse(f.requests.find(value => value.init.method === 'POST').init.body);
  assert.deepEqual(body.comments, [{ id: 'c1', start: 2, end: 3, content: 'Note' }]);
  assert.deepEqual(saved.documents[0].comments, [{ id: 'c1', start: 3, end: 4, content: 'Note' }]);
});

for (const patch of [{ id: '../other' }, { id: 'temp-td-user:synthetic' }, { id: null }, { title: '' },
  { title: 'bad\nname' }, { textdoc_type: 'unknown' }, { version: 0 }, { version: 1.2 }, { version: '4' },
  { comments: null }, { content: null }, { content: 'x'.repeat(128 * 1024 + 1) }, { content: '\ud800' }]) {
  test('invalid original schema is never projected or truncated: ' + Object.keys(patch)[0] + ':' + String(Object.values(patch)[0]).slice(0, 30), () => {
    assert.throws(() => policy.parse([doc(patch)]), /^Error: canvas_/);
  });
}

test('list completeness and identity require a bounded array with unique original IDs', () => {
  for (const value of [{ textdocs: [doc()] }, null, [doc(), doc()], Array.from({ length: 201 }, (_, i) => doc({ id: 'id_' + i }))]) {
    assert.throws(() => policy.parse(value), /canvas_list_unconfirmed/);
  }
  assert.deepEqual(policy.parse([]), []);
});

for (const [name, change] of [
  ['missing', () => []],
  ['duplicate', value => [value, value]],
  ['changed ID', value => [{ ...value, id: 'other' }]],
  ['changed text', value => [{ ...value, content: 'Other note' }]],
  ['collapsed anchor', value => [{ ...value, start: 1, end: 1 }]],
  ['surrogate boundary', value => [{ ...value, start: 2, end: 3 }]],
  ['negative', value => [{ ...value, start: -1 }]],
  ['outside', value => [{ ...value, end: 5 }]],
  ['reversed', value => [{ ...value, start: 3, end: 1 }]],
  ['fractional', value => [{ ...value, start: 0.5 }]],
]) {
  test('text save cannot silently remove, corrupt or replace comments: ' + name, async () => {
    const f = fixture(); Object.assign(f.rows[0], { content: 'A\u{1f600}B',
      comments: [{ id: 'c1', start: 1, end: 2, content: 'Original note' }] });
    const read = await f.list(), before = read.documents[0];
    const saved = await f.save(read.ticket, { content: before.content, comments: change(before.comments[0]) });
    assert.equal(saved.ok, false); assert.equal(saved.attempted, false);
    assert.equal(f.requests.length, 1);
  });
}

test('empty code source and zero-width pre-existing comments round-trip', () => {
  const value = policy.parse([doc({ content: '', textdoc_type: 'code/typescript',
    comments: [{ id: 'c1', start: 0, end: 0, content: '' }] })])[0];
  assert.deepEqual(policy.draft(value, value), { content: '', comments: [{ id: 'c1', start: 0, end: 0, content: '' }] });
});

test('comment ordering is irrelevant to readback, but positions and content must match', () => {
  const rows = [{ id: 'c1', start: 0, end: 1, content: 'One' }, { id: 'c2', start: 1, end: 2, content: 'Two' }];
  const before = policy.parse([doc({ content: 'ABC', comments: rows })])[0], expected = policy.draft(before, before);
  assert.equal(policy.matches({ ...before, documentVersion: 5, comments: [...before.comments].reverse() }, before, expected, 5), true);
  assert.equal(policy.matches({ ...before, documentVersion: 5, comments: [before.comments[0]] }, before, expected, 5), false);
  assert.equal(policy.matches(undefined, before, expected, 5), false);
});

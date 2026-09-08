'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');
const policy = require('../android/app/src/main/assets/chatgpt_web_private_stream_policy');

const frame = (overrides = {}) => ({ conversation_id: 'synthetic-thread', message: {
  id: 'synthetic-answer', author: { role: 'assistant' }, status: 'in_progress',
  content: { content_type: 'text', parts: ['Synthetic public answer.'] }, ...overrides
} });

for (const channel of [undefined, null, 'final']) {
  test(`public answer channel ${channel} still streams`, () => {
    assert.equal(policy.assistantFrame(frame({ channel, recipient: 'all' })).text,
      'Synthetic public answer.');
  });
}

for (const [name, overrides] of [
  ['analysis', { channel: 'analysis' }],
  ['commentary is not the final answer', { channel: 'commentary' }],
  ['unknown channel', { channel: 'future-channel' }],
  ['tool-directed text', { channel: 'final', recipient: 'synthetic-tool' }],
  ['hidden message', { metadata: { is_visually_hidden_from_conversation: true } }],
  ['hidden reasoning group', { metadata: { is_visually_hidden_reasoning_group: true } }],
  ['internal debug message', { metadata: { debug_internal_only: true } }]
]) {
  test(`${name} cannot become an answer or replace its stream identity`, () => {
    const hidden = frame({ ...overrides, id: 'synthetic-hidden',
      content: { content_type: 'text', parts: ['Synthetic non-answer payload.'] } });
    assert.equal(policy.assistantFrame(hidden), null);
    const session = policy.createSession({ now: () => 1 });
    session.accept(frame());
    assert.equal(session.accept(hidden), false);
    assert.equal(session.current('/c/synthetic-thread').id, 'synthetic-answer');
    assert.equal(session.current('/c/synthetic-thread').text, 'Synthetic public answer.');
  });
}

test('compact analysis patches stay private until a final answer arrives', () => {
  const session = policy.createSession({ now: () => 1 });
  assert.equal(session.accept({ v: frame({ channel: 'analysis' }), c: 1 }), false);
  assert.equal(session.accept({ p: '/message/content/parts/0', o: 'append', v: ' More.' }), false);
  assert.equal(session.current('/c/synthetic-thread'), null);
  assert.equal(session.accept({ v: frame({ channel: 'final' }), c: 1 }), true);
  assert.equal(session.current('/c/synthetic-thread').text, 'Synthetic public answer.');
});

test('public search progress does not expose its tool payload', () => {
  const session = policy.createSession({ now: () => 1 });
  assert.equal(session.accept(frame({ recipient: 'synthetic-tool', channel: 'analysis',
    metadata: { reasoning_title: 'Searching the web' } })), true);
  const value = session.current('/c/synthetic-thread');
  assert.equal(value.text, '');
  assert.equal(value.progressLabel, 'Searching the web');
});

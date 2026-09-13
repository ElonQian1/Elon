'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { install } = require('./fixtures/chatgpt-owned-stream-transport');
const CID = '11111111-1111-4111-8111-111111111111';
const UID = '22222222-2222-4222-8222-222222222222';
const AID = '33333333-3333-4333-8333-333333333333';
const fixture = () => {
  const page = { location: { href: 'https://chatgpt.com/c/' + CID }, __elonChatGptDocumentToken: 'synthetic-token' };
  return { page, transport: install(page), binding: { conversationId: CID, userMessageId: UID, current: () => true } };
};
const answer = text => ({ data: { conversation_id: CID, message: { id: AID,
  author: { role: 'assistant' }, status: 'in_progress', content: { parts: [text] } } } });

test('file-only admission is explicit and never synthesizes an empty text bubble', () => {
  const { transport, binding } = fixture();
  assert.equal(transport.preparePrivateSend('', UID), false);
  assert.equal(transport.beginPrivateStream(binding), null);
  assert.equal(transport.preparePrivateSend('', UID, 'true'), false);
  assert.equal(transport.preparePrivateSend('', UID, true), true);
  assert.deepEqual(JSON.parse(JSON.stringify(transport.mergeMessages([], '/c/' + CID))), []);
  const sink = transport.beginPrivateStream(binding);
  assert.ok(sink);
  sink.push(answer('synthetic file response'));
  assert.equal(transport.current('/c/' + CID).text, 'synthetic file response');
  sink.finish(); transport.dispose();
});

test('rendering an optimistic user cannot consume the independent stream admission', () => {
  const { transport, binding } = fixture();
  transport.preparePrivateSend('synthetic prompt', UID);
  transport.mergeMessages([{ id: UID, role: 'user', content: [{ type: 'text', text: 'synthetic prompt' }] }], '/c/' + CID);
  const sink = transport.beginPrivateStream(binding);
  assert.ok(sink);
  assert.equal(transport.beginPrivateStream(binding), null, 'one admission cannot replace its active sink');
  sink.push(answer('first sink remains current'));
  assert.equal(transport.current('/c/' + CID).text, 'first sink remains current');
  transport.dispose();
});

test('wrong IDs and unprepared/reset/finished/disposed owners cannot start streams', () => {
  for (const boundary of ['reset', 'prepareSend', 'finishPrivateSend', 'dispose']) {
    const { transport, binding } = fixture();
    transport.preparePrivateSend('', UID, true);
    assert.equal(transport.beginPrivateStream({ ...binding, userMessageId: AID }), null);
    transport[boundary]();
    assert.equal(transport.beginPrivateStream(binding), null);
    transport.dispose();
  }
  const { transport } = fixture();
  transport.dispose();
  assert.equal(transport.preparePrivateSend('after disposal', UID), false);
});

test('replacement and document boundaries invalidate previously returned sinks', () => {
  const { page, transport, binding } = fixture();
  transport.preparePrivateSend('', UID, true);
  const old = transport.beginPrivateStream(binding);
  transport.preparePrivateSend('another turn', AID);
  assert.throws(() => old.push(answer('late')), /context_changed/);
  const next = transport.beginPrivateStream({ ...binding, userMessageId: AID });
  page.__elonChatGptDocumentToken = 'new-document';
  assert.throws(() => next.push(answer('late')), /context_changed/);
  transport.dispose();
});

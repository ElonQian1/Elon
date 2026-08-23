'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');

const policy = require(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'assets',
  'chatgpt_web_private_stream_policy.js'
));

const payload = (text, status = 'in_progress') => ({
  conversation_id: 'conversation-one',
  message: {
    id: 'assistant-one',
    author: { role: 'assistant' },
    status,
    content: { parts: [text] }
  }
});

assert.deepEqual(policy.assistantFrame(payload('first')), {
  id: 'assistant-one',
  conversationId: 'conversation-one',
  text: 'first',
  state: 'streaming'
});
assert.equal(policy.assistantFrame({ message: { author: { role: 'user' } } }), null);
assert.equal(policy.assistantFrame({
  message: {
    id: 'assistant-object-parts',
    author: { role: 'assistant' },
    content: { parts: [{ text: 'first' }, { content: 'second' }] }
  }
}).text, 'first\nsecond');

const decoded = [];
let done = 0;
const decoder = policy.createSseDecoder((value) => decoded.push(value), () => { done += 1; });
decoder.push('event: message\ndata: {"message":{"author":{"role":"assistant"},');
decoder.push('"content":{"parts":["hel');
decoder.push('lo"]}}}\n\ndata: [DONE]\n\n');
assert.equal(decoded.length, 1);
assert.equal(decoded[0].message.content.parts[0], 'hello');
assert.equal(done, 1);

let now = 1000;
const session = policy.createSession({ now: () => now });
session.begin();
assert.equal(session.accept(payload('hello')), true);
assert.equal(session.current('/c/conversation-one').state, 'streaming');
assert.equal(session.current('/c/another-conversation'), null);

const user = { id: 'user-one', role: 'user', state: 'completed', content: [{ type: 'text', text: 'question' }] };
let merged = session.merge([user], '/c/conversation-one');
assert.equal(merged.length, 2);
assert.equal(merged[1].id, 'private-stream:assistant-one');
assert.equal(merged[1].content[0].text, 'hello');
assert.equal(merged[1].state, 'streaming');

assert.equal(session.accept(payload('hello world', 'finished_successfully')), true);
assert.equal(session.finish(), true);
merged = session.merge([user, {
  id: 'assistant-one',
  role: 'assistant',
  state: 'completed',
  content: [{ type: 'markdown', text: 'hello' }, { type: 'citation', text: 'source' }]
}], '/c/conversation-one');
assert.equal(merged.length, 2);
assert.equal(merged[1].content[0].text, 'hello world');
assert.equal(merged[1].content[1].type, 'citation');

const longerDom = policy.mergeMessages([{
  id: 'assistant-one',
  role: 'assistant',
  state: 'completed',
  content: [{ type: 'markdown', text: 'hello world from DOM' }]
}], {
  id: 'assistant-one',
  text: 'hello world',
  state: 'streaming'
});
assert.equal(longerDom[0].content[0].text, 'hello world from DOM');
assert.equal(longerDom[0].state, 'streaming');

merged = session.merge([user, {
  id: 'assistant-one',
  role: 'assistant',
  state: 'completed',
  content: [{ type: 'markdown', text: 'hello world' }]
}], '/c/conversation-one');
assert.equal(merged.length, 2);
assert.equal(session.current('/c/conversation-one'), null);

session.begin();
session.accept(payload('stale'));
now += 5 * 60 * 1000 + 1;
assert.equal(session.current('/c/conversation-one'), null);

console.log('CHATGPT_WEB_PRIVATE_STREAM_POLICY_TESTS=passed');

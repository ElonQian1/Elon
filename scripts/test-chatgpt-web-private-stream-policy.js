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
  citations: [],
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
assert.deepEqual(policy.progressFrame({
  c: 1,
  v: {
    conversation_id: 'conversation-one',
    message: {
      author: { role: 'assistant' },
      content: { content_type: 'code', parts: [{ text: 'private tool instruction' }] },
      metadata: { reasoning_title: '正在搜索 South Korea stock market' }
    }
  }
}), {
  conversationId: 'conversation-one',
  progressLabel: '正在搜索 South Korea stock market',
  state: 'streaming'
});

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

const compactSession = policy.createSession({ now: () => 2000 });
compactSession.begin();
assert.equal(compactSession.accept({
  c: 1,
  v: {
    conversation_id: 'conversation-compact',
    message: {
      id: 'assistant-search',
      author: { role: 'assistant' },
      status: 'finished_successfully',
      content: { content_type: 'code', parts: [{ text: 'private tool instruction' }] },
      metadata: { reasoning_title: '正在搜索 South Korea stock market' }
    }
  }
}), true);
assert.equal(
  compactSession.current('/c/conversation-compact').progressLabel,
  '正在搜索 South Korea stock market'
);
assert.equal(compactSession.accept({
  c: 13,
  v: {
    conversation_id: 'conversation-compact',
    message: {
      id: 'assistant-compact',
      author: { role: 'assistant' },
      status: 'in_progress',
      content: { content_type: 'text', parts: [''] },
      metadata: { content_references: [] }
    }
  }
}), true);
const compactPlaceholder = compactSession.merge([], '/c/conversation-compact');
assert.equal(compactPlaceholder.length, 1);
assert.equal(compactPlaceholder[0].state, 'streaming');
assert.equal(compactPlaceholder[0].content[0].text, '');
assert.equal(compactSession.accept({
  o: 'append',
  p: '/message/content/parts/0',
  v: 'KOSPI opened higher'
}), true);
assert.equal(compactSession.accept({ v: ' and remained volatile.' }), true);
assert.equal(
  compactSession.current('/c/conversation-compact').text,
  'KOSPI opened higher and remained volatile.'
);
assert.equal(compactSession.accept({
  o: 'patch',
  p: '',
  v: [
    { o: 'append', p: '/message/content/parts/0', v: ' Final.' },
    { o: 'replace', p: '/message/status', v: 'finished_successfully' }
  ]
}), true);
assert.equal(compactSession.current('/c/conversation-compact').state, 'completed');
assert.equal(
  compactSession.current('/c/conversation-compact').text,
  'KOSPI opened higher and remained volatile. Final.'
);

console.log('CHATGPT_WEB_PRIVATE_STREAM_POLICY_TESTS=passed');

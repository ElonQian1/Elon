'use strict';

const assert = require('assert');
const path = require('path');

const policy = require(path.resolve(
  __dirname,
  '../android/app/src/main/assets/chatgpt_web_adapter_message_portal_policy.js'
));

function rect(top, bottom) {
  return { top, bottom };
}

const messages = [rect(80, 220), rect(300, 470)];

assert.equal(
  policy.inferMessageIndex({
    semantic: 'copy',
    role: 'button',
    actionRect: rect(224, 252),
    messageRects: messages,
    viewportHeight: 800
  }),
  0,
  'a portal action row belongs to the closest preceding message'
);

assert.equal(
  policy.inferMessageIndex({
    semantic: 'more',
    role: 'menuitem',
    actionRect: rect(224, 252),
    messageRects: messages,
    viewportHeight: 800
  }),
  -1,
  'a real overlay menu item must remain owned by the overlay'
);

assert.equal(
  policy.inferMessageIndex({
    semantic: 'settings',
    role: 'button',
    actionRect: rect(224, 252),
    messageRects: messages,
    viewportHeight: 800
  }),
  -1,
  'an unrelated overlay action must not be assigned to a message'
);

assert.equal(
  policy.inferMessageIndex({
    semantic: 'share',
    role: 'button',
    actionRect: rect(650, 680),
    messageRects: messages,
    viewportHeight: 800
  }),
  -1,
  'a portal action outside the bounded message neighborhood is rejected'
);

assert.equal(
  policy.inferMessageIndex({
    semantic: 'feedback',
    role: '',
    actionRect: rect(270, 290),
    messageRects: messages,
    viewportHeight: 800
  }),
  1,
  'when two messages are nearby the closest message wins deterministically'
);

const messageNode = {
  id: '',
  getAttribute(name) {
    if (name === 'data-testid') return 'conversation-turn-42';
    return null;
  },
  getBoundingClientRect() {
    return rect(80, 220);
  }
};

assert.equal(
  policy.inferMessageContext({
    semantic: 'copy',
    role: 'button',
    actionRect: rect(224, 252),
    messages: [messageNode],
    viewportHeight: 800
  }),
  'conversation-turn-42',
  'the inferred owner uses the same stable message context identifier'
);

console.log('CHATGPT_MESSAGE_PORTAL_POLICY=passed');

'use strict';

const assert = require('assert');
const policy = require('../android/app/src/main/assets/chatgpt_web_adapter_message_action_policy.js');

function control(attributes, textContent) {
  return {
    textContent: textContent || '',
    getAttribute(name) {
      return Object.prototype.hasOwnProperty.call(attributes, name) ? attributes[name] : '';
    }
  };
}

assert.strictEqual(policy.isRegenerateSignal('Regenerate response'), true);
assert.strictEqual(policy.isRegenerateSignal('Try again'), true);
assert.strictEqual(policy.isRegenerateSignal('重新生成'), true);
assert.strictEqual(policy.isRegenerateSignal('Copy'), false);
assert.strictEqual(policy.isOverflowSignal('More actions'), true);
assert.strictEqual(policy.isOverflowSignal('更多操作'), true);
assert.strictEqual(policy.isOverflowSignal('Share'), false);
assert.strictEqual(
  policy.isRegenerateControl(control({ 'data-testid': 'regenerate-response-button' })),
  true
);
assert.strictEqual(
  policy.isOverflowControl(control({ 'aria-label': 'More actions' })),
  true
);
assert.strictEqual(
  policy.isRegenerateControl(control({ role: 'menuitem' }, '重新回答')),
  true
);

console.log('CHATGPT_MESSAGE_ACTION_POLICY_TEST=passed');

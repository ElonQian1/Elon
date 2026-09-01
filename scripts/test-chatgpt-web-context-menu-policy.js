'use strict';

const assert = require('assert');
const policy = require('../android/app/src/main/assets/chatgpt_web_adapter_context_menu_policy.js');

const control = { semantic: 'conversation_options', contextId: 'conversation_1' };
assert.equal(policy.shouldArm(control), true);
assert.equal(policy.shouldArm({ semantic: 'conversation_options', contextId: '' }), false);
assert.equal(policy.shouldArm({ semantic: 'more', contextId: 'conversation_1' }), false);

let clicks = 0;
assert.equal(policy.activate(control, { click: () => { clicks += 1; } }), true);
assert.equal(clicks, 1, 'the exact conversation trigger is activated once');
assert.equal(policy.activate({ semantic: 'more', contextId: 'conversation_1' }, {
  click: () => { clicks += 1; }
}), false);
assert.equal(policy.activate(control, null), false);
assert.equal(policy.activate(control, { click: () => { throw new Error('detached'); } }), false);
assert.equal(clicks, 1, 'unsupported and failed controls do not produce another activation');

process.stdout.write('chatgpt context menu policy tests passed\n');

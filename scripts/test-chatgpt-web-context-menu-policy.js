'use strict';

const assert = require('assert');
const policy = require('../android/app/src/main/assets/chatgpt_web_adapter_context_menu_policy.js');

const control = { semantic: 'conversation_options', contextId: 'conversation_1' };
assert.equal(policy.shouldArm(control), true);
assert.equal(policy.shouldArm({ semantic: 'conversation_options', contextId: '' }), false);
assert.equal(policy.shouldArm({ semantic: 'more', contextId: 'conversation_1' }), false);
assert.equal(policy.isProjectMoveStep({
  semantic: 'save_to_project', region: 'overlay', contextId: 'conversation_1', enabled: true
}), true);
assert.equal(policy.isProjectMoveStep({
  semantic: 'project', region: 'overlay', role: 'menuitem', enabled: true
}), true);
assert.equal(policy.isProjectMoveStep({
  semantic: 'confirm', region: 'overlay', role: 'button', enabled: true
}), true);
assert.equal(policy.isProjectMoveStep({
  semantic: 'action', label: '确认', region: 'overlay', role: 'button', enabled: true
}), true);
assert.equal(policy.isProjectMoveStep({
  semantic: 'project', region: 'navigation', role: 'menuitem', enabled: true
}), false);
assert.equal(policy.isProjectMoveStep({
  semantic: 'project', region: 'overlay', role: 'link', enabled: true
}), false);

let clicks = 0;
assert.equal(policy.activate(control, { click: () => { clicks += 1; } }), true);
assert.equal(clicks, 1, 'the exact conversation trigger is activated once');
assert.equal(policy.activate({
  semantic: 'save_to_project', region: 'overlay', contextId: 'conversation_1', enabled: true
}, { click: () => { clicks += 1; } }), true);
assert.equal(policy.activate({
  semantic: 'project', region: 'overlay', role: 'menuitem', enabled: true
}, { click: () => { clicks += 1; } }), true);
assert.equal(policy.activate({
  semantic: 'confirm', region: 'overlay', role: 'button', enabled: true
}, { click: () => { clicks += 1; } }), true);
assert.equal(clicks, 4, 'the exact project move steps are each activated once');
assert.equal(policy.activate({ semantic: 'more', contextId: 'conversation_1' }, {
  click: () => { clicks += 1; }
}), false);
assert.equal(policy.activate(control, null), false);
assert.equal(policy.activate(control, { click: () => { throw new Error('detached'); } }), false);
assert.equal(clicks, 4, 'unsupported and failed controls do not produce another activation');

process.stdout.write('chatgpt context menu policy tests passed\n');

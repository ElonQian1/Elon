'use strict';

const assert = require('assert');
const policy = require('../android/app/src/main/assets/chatgpt_web_adapter_context_menu_policy.js');

const control = {
  semantic: 'conversation_options',
  contextId: 'conversation_1',
  region: 'header'
};
assert.equal(policy.shouldArm(control), true);
assert.equal(policy.requiresNativeTouch(control), true);
assert.equal(policy.canUseAfterTouchMiss(control, 'conversation_1', 0), true);
assert.equal(policy.canUseAfterTouchMiss(control, 'conversation_2', 0), false);
assert.equal(policy.canUseAfterTouchMiss(control, 'conversation_1', 1), false);
assert.equal(policy.canUseAfterTouchMiss({ ...control, region: 'navigation' }, 'conversation_1', 0), false);
assert.equal(policy.shouldArm({ semantic: 'conversation_options', contextId: '' }), false);
assert.equal(policy.requiresNativeTouch({ semantic: 'more', contextId: 'conversation_1' }), false);
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
const pointerEvents = [];
function FakePointerEvent(type) { this.type = type; }
assert.equal(policy.activate(control, { click: () => { clicks += 1; } }), false);
assert.equal(clicks, 0, 'the conversation trigger falls through to the native touch bridge');
assert.equal(policy.activateAfterTouchMiss(
  control,
  {
    ownerDocument: { defaultView: { PointerEvent: FakePointerEvent } },
    dispatchEvent: (event) => { pointerEvents.push(event.type); },
    click: () => { clicks += 1; }
  },
  'conversation_1',
  0
), true);
assert.deepStrictEqual(pointerEvents, ['pointerdown', 'pointerup']);
assert.equal(clicks, 0, 'the guarded fallback uses the menu pointer contract instead of click');
assert.equal(policy.activateAfterTouchMiss(
  control,
  { click: () => { clicks += 1; } },
  'conversation_2',
  0
), false);
assert.equal(clicks, 0, 'a stale conversation cannot receive the fallback click');
assert.equal(policy.activate({
  semantic: 'save_to_project', region: 'overlay', contextId: 'conversation_1', enabled: true
}, { click: () => { clicks += 1; } }), true);
assert.equal(policy.activate({
  semantic: 'project', region: 'overlay', role: 'menuitem', enabled: true
}, { click: () => { clicks += 1; } }), true);
assert.equal(policy.activate({
  semantic: 'confirm', region: 'overlay', role: 'button', enabled: true
}, { click: () => { clicks += 1; } }), true);
assert.equal(clicks, 3, 'the exact project move steps are each activated once');
assert.equal(policy.activate({ semantic: 'more', contextId: 'conversation_1' }, {
  click: () => { clicks += 1; }
}), false);
assert.equal(policy.activate(control, null), false);
assert.equal(policy.activate(control, { click: () => { throw new Error('detached'); } }), false);
assert.equal(clicks, 3, 'unsupported and failed controls do not produce another activation');

process.stdout.write('chatgpt context menu policy tests passed\n');

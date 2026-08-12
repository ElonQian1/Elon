'use strict';

const assert = require('node:assert/strict');
const policy = require(
  '../android/app/src/main/assets/chatgpt_web_adapter_composer_tool_state_policy.js'
);

assert.equal(policy.semantic({
  region: 'composer',
  signal: 'composer-regenerate retry',
  label: '搜索'
}), 'web_search');
assert.equal(policy.semantic({
  region: 'composer',
  signal: 'search the web',
  label: 'Search'
}), 'web_search');
assert.equal(policy.semantic({
  region: 'header',
  signal: 'search the web',
  label: 'Search'
}), '');
assert.equal(policy.semantic({
  region: 'composer',
  signal: 'search chats',
  label: '搜索聊天'
}), '');

assert.equal(policy.controlSelected({
  semantic: 'web_search',
  region: 'composer',
  directSelected: false
}), false);
assert.equal(policy.controlSelected({
  semantic: 'web_search',
  region: 'overlay',
  directSelected: false
}), false);
assert.equal(policy.controlSelected({
  semantic: 'tool',
  region: 'composer',
  directSelected: true
}), true);

assert.equal(policy.optionSelected({
  semantic: 'web_search',
  directSelected: false,
  directKnown: false,
  activeInComposer: true
}), true);
assert.equal(policy.optionSelected({
  semantic: 'web_search',
  directSelected: false,
  directKnown: true,
  activeInComposer: true
}), false);
assert.equal(policy.optionSelected({
  semantic: 'web_search',
  directSelected: false,
  directKnown: false,
  activeInComposer: false
}), false);
assert.equal(policy.optionSelected({
  semantic: 'tool',
  directSelected: true,
  activeInComposer: false
}), true);

assert.deepEqual(policy.directSelection({ ariaChecked: 'true' }), {
  known: true,
  selected: true
});
assert.deepEqual(policy.directSelection({ ariaPressed: 'false' }), {
  known: true,
  selected: false
});
assert.deepEqual(policy.directSelection({ dataState: 'active' }), {
  known: true,
  selected: true
});
assert.deepEqual(policy.directSelection({ dataState: 'unchecked' }), {
  known: true,
  selected: false
});
assert.deepEqual(policy.directSelection({}), {
  known: false,
  selected: false
});

const tracker = policy.createSelectionTracker();
assert.equal(tracker.value('web_search', false), false);
assert.equal(tracker.observe('web_search', true), true);
assert.equal(tracker.value('web_search', false), true);
assert.equal(tracker.value('web_search', false), true, 'a hidden composer control must not erase cached state');
assert.equal(tracker.observe('web_search', false), false);
assert.equal(tracker.value('web_search', true), false);

process.stdout.write('CHATGPT_COMPOSER_TOOL_STATE_POLICY=passed\n');

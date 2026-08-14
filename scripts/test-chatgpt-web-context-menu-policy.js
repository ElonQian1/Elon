'use strict';

const assert = require('assert');
const policy = require('../android/app/src/main/assets/chatgpt_web_adapter_context_menu_policy.js');

const sidebar = {};
const menu = {};
assert.equal(policy.shouldArm({ semantic: 'conversation_options', contextId: 'conversation_1' }), true);
assert.equal(policy.shouldArm({ semantic: 'more', contextId: 'conversation_1' }), false);
assert.equal(policy.hasNewRoot([sidebar], [sidebar, menu]), true);
assert.equal(policy.hasNewRoot([sidebar], [sidebar]), false);

let retryCount = 0;
let scheduled = null;
const retryWhenMissing = policy.prepare(
  { semantic: 'conversation_options', contextId: 'conversation_1' },
  () => [sidebar],
  (task) => { scheduled = task; }
);
retryWhenMissing(() => { retryCount += 1; });
scheduled();
assert.equal(retryCount, 1);

let roots = [sidebar];
const skipWhenOpened = policy.prepare(
  { semantic: 'conversation_options', contextId: 'conversation_1' },
  () => roots,
  (task) => { scheduled = task; }
);
roots = [sidebar, menu];
skipWhenOpened(() => { retryCount += 1; });
scheduled();
assert.equal(retryCount, 1);

sidebar.menu = '';
const reusedRoot = policy.prepare(
  { semantic: 'conversation_options', contextId: 'conversation_1' },
  () => [sidebar],
  (task) => { scheduled = task; },
  260,
  (root) => root.menu
);
sidebar.menu = 'rename|archive|delete';
reusedRoot(() => { retryCount += 1; });
scheduled();
assert.equal(retryCount, 1, 'a menu mounted into an existing root must not be clicked closed');
process.stdout.write('chatgpt context menu policy tests passed\n');

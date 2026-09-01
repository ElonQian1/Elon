'use strict';

const assert = require('assert');
const policy = require('../android/app/src/main/assets/chatgpt_web_adapter_context_menu_policy.js');

function drain(tasks, limit = 32) {
  let count = 0;
  while (tasks.length) {
    assert.ok(count < limit, 'context menu observation must remain bounded');
    tasks.shift()();
    count += 1;
  }
}

const sidebar = {};
const menu = {};
assert.equal(policy.shouldArm({ semantic: 'conversation_options', contextId: 'conversation_1' }), true);
assert.equal(policy.shouldArm({ semantic: 'more', contextId: 'conversation_1' }), false);
assert.equal(policy.hasNewRoot([sidebar], [sidebar, menu]), true);
assert.equal(policy.hasNewRoot([sidebar], [sidebar]), false);

let tasks = [];
let openedCount = 0;
let timedOutCount = 0;
const missingMenu = policy.prepare(
  { semantic: 'conversation_options', contextId: 'conversation_1' },
  () => [sidebar],
  (task) => { tasks.push(task); },
  100,
  undefined,
  300
);
assert.equal(missingMenu(
  () => { openedCount += 1; },
  () => { timedOutCount += 1; }
), true);
assert.equal(missingMenu.isOpen(), false);
drain(tasks);
assert.equal(openedCount, 0);
assert.equal(timedOutCount, 1, 'a missing menu must fail after a bounded observation window');

let roots = [sidebar];
tasks = [];
const newlyMountedMenu = policy.prepare(
  { semantic: 'conversation_options', contextId: 'conversation_1' },
  () => roots,
  (task) => { tasks.push(task); },
  100,
  undefined,
  300
);
newlyMountedMenu(
  () => { openedCount += 1; },
  () => { timedOutCount += 1; }
);
roots = [sidebar, menu];
assert.equal(newlyMountedMenu.isOpen(), true);
drain(tasks);
assert.equal(openedCount, 1, 'a newly mounted menu confirms the command');
assert.equal(timedOutCount, 1);

sidebar.menu = '';
tasks = [];
const reusedRoot = policy.prepare(
  { semantic: 'conversation_options', contextId: 'conversation_1' },
  () => [sidebar],
  (task) => { tasks.push(task); },
  100,
  (root) => root.menu,
  300
);
reusedRoot(
  () => { openedCount += 1; },
  () => { timedOutCount += 1; }
);
sidebar.menu = 'menuitem:rename|menuitem:archive|menuitem:delete';
drain(tasks);
assert.equal(openedCount, 2, 'a menu mounted into an existing root confirms the command');
assert.equal(timedOutCount, 1);

tasks = [];
const unchangedRoot = policy.prepare(
  { semantic: 'conversation_options', contextId: 'conversation_1' },
  () => [sidebar],
  (task) => { tasks.push(task); },
  100,
  (root) => root.menu,
  300
);
unchangedRoot(
  () => { openedCount += 1; },
  () => { timedOutCount += 1; }
);
drain(tasks);
assert.equal(openedCount, 2);
assert.equal(timedOutCount, 2, 'an unchanged pre-existing overlay is not mistaken for this menu');

let expanded = false;
const expandedTransition = policy.prepare(
  { semantic: 'conversation_options', contextId: 'conversation_1' },
  () => [sidebar],
  (task) => { tasks.push(task); },
  100,
  (root) => root.menu,
  300,
  () => expanded
);
assert.equal(expandedTransition.isOpen(), false);
expanded = true;
assert.equal(expandedTransition.isOpen(), true, 'a false-to-true trigger transition confirms the menu');

const alreadyExpanded = policy.prepare(
  { semantic: 'conversation_options', contextId: 'conversation_1' },
  () => [sidebar],
  (task) => { tasks.push(task); },
  100,
  (root) => root.menu,
  300,
  () => true
);
assert.equal(alreadyExpanded.isOpen(), false, 'a pre-existing expanded state is not fresh evidence');

assert.equal(policy.prepare(
  { semantic: 'more', contextId: 'conversation_1' },
  () => [],
  (task) => { tasks.push(task); }
), null);
process.stdout.write('chatgpt context menu policy tests passed\n');

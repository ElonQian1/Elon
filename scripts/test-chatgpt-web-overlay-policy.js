'use strict';

const assert = require('assert');
const policy = require('../android/app/src/main/assets/chatgpt_web_adapter_overlay_policy.js');

function node(label, parentElement = null, attributes = {}) {
  return {
    id: '',
    textContent: label,
    parentElement,
    getAttribute(name) { return attributes[name] || ''; },
    hasAttribute(name) { return Object.prototype.hasOwnProperty.call(attributes, name); }
  };
}

const menu = node('', null, { role: 'menu' });
const rename = node('Rename chat', menu, { role: 'menuitem' });
const archive = node('Archive', menu, { role: 'menuitem' });
const unrelated = node('Open conversation', menu);
menu.actions = [rename, archive, unrelated];
const sidebar = node('');
sidebar.actions = [node('Conversation one', sidebar), node('Conversation two', sidebar)];
const documentMock = {
  querySelectorAll(selector) {
    return selector.includes('data-radix') ? [sidebar] : [rename, archive, unrelated];
  }
};
const visible = () => true;
const actionable = (value) => value.actions || [];

assert.strictEqual(policy.isManagementAction(rename), true);
assert.strictEqual(policy.isManagementAction(unrelated), false);
assert.deepStrictEqual(policy.visibleRoots(documentMock, visible, actionable), [menu, sidebar]);
assert.match(policy.contextMenuSignature(menu, visible, actionable), /menuitem:Rename chat/);

const portal = node('', null, { 'data-headlessui-portal': '' });
portal.actions = menu.actions;
assert.deepStrictEqual(
  policy.rankedRoots([sidebar, portal, menu], visible, actionable),
  [menu, portal, sidebar],
  'the focused menu root wins over its portal wrapper and unrelated overlays'
);
process.stdout.write('chatgpt overlay policy tests passed\n');

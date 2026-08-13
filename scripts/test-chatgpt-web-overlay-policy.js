'use strict';

const assert = require('assert');
const policy = require('../android/app/src/main/assets/chatgpt_web_adapter_overlay_policy.js');

function node(label, parentElement = null) {
  return {
    id: '',
    textContent: label,
    parentElement,
    getAttribute() { return ''; }
  };
}

const menu = node('');
const rename = node('Rename chat', menu);
const archive = node('Archive', menu);
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
assert.deepStrictEqual(policy.visibleRoots(documentMock, visible, actionable), [sidebar, menu]);
process.stdout.write('chatgpt overlay policy tests passed\n');

'use strict';

const assert = require('assert');
const policy = require(
  '../android/app/src/main/assets/chatgpt_web_adapter_sidebar_control_policy.js'
);

function node(attributes, rect, parentElement = null) {
  return {
    id: attributes.id || '',
    textContent: attributes.textContent || '',
    parentElement,
    getAttribute(name) { return attributes[name] || ''; },
    getBoundingClientRect() { return rect; },
    querySelectorAll() { return this.actions || []; }
  };
}

const viewport = { width: 400, height: 800 };
const sidebar = node({}, { left: 0, top: 0, right: 360, bottom: 800, width: 360, height: 800 });
const account = node(
  { 'data-testid': 'accounts-profile-button', 'aria-haspopup': 'menu', textContent: 'Private account name' },
  { left: 16, top: 720, right: 220, bottom: 768, width: 204, height: 48 },
  sidebar
);
const anonymousMenu = node(
  { 'aria-haspopup': 'menu', textContent: 'Private account name' },
  { left: 16, top: 660, right: 220, bottom: 708, width: 204, height: 48 },
  sidebar
);
const conversation = node(
  { href: '/c/private-id', 'aria-haspopup': 'menu' },
  { left: 16, top: 730, right: 220, bottom: 778, width: 204, height: 48 },
  sidebar
);
sidebar.actions = [account, anonymousMenu, conversation];
const documentMock = {
  querySelectorAll(selector) {
    return selector.includes('aside') ? [sidebar] : [];
  }
};

assert.strictEqual(policy.isSidebarScope(sidebar, () => true, viewport.width, viewport.height), true);
assert.strictEqual(policy.isAccountTrigger(account, sidebar), true);
assert.strictEqual(policy.isAccountTrigger(anonymousMenu, sidebar), true);
assert.strictEqual(policy.isAccountTrigger(conversation, sidebar), false);
assert.deepStrictEqual(
  policy.findAccountTriggers(documentMock, () => true, viewport.width, viewport.height),
  [account, anonymousMenu]
);
assert.ok(!policy.attributeSignal(account).includes('Private account name'));

process.stdout.write('chatgpt sidebar control policy tests passed\n');

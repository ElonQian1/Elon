'use strict';

const assert = require('node:assert/strict');
const policy = require(
  '../android/app/src/main/assets/chatgpt_web_adapter_composer_tool_state_policy.js'
);

assert.equal(policy.semantic({
  region: 'composer',
  signal: 'composer-web-search retry search',
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
}), true);
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
  activeInComposer: true
}), true);
assert.equal(policy.optionSelected({
  semantic: 'web_search',
  directSelected: false,
  activeInComposer: false
}), false);
assert.equal(policy.optionSelected({
  semantic: 'tool',
  directSelected: true,
  activeInComposer: false
}), true);

process.stdout.write('CHATGPT_COMPOSER_TOOL_STATE_POLICY=passed\n');

'use strict';

const policy = require('../android/app/src/main/assets/chatgpt_web_adapter_page_semantic_policy.js');

function expectEqual(actual, expected, name) {
  if (actual !== expected) {
    throw new Error(`${name}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}

expectEqual(policy.classify({
  pathname: '/scheduled',
  region: 'content',
  signal: 'Pause weekly briefing'
}), 'tasks', 'scheduled task action');

expectEqual(policy.classify({
  pathname: '/scheduled',
  region: 'content',
  signal: 'open-sidebar-button'
}), 'navigation', 'scheduled sidebar trigger');

expectEqual(policy.classify({
  pathname: '/scheduled/history',
  region: 'content',
  signal: 'Suggested automation'
}), 'tasks', 'scheduled child route');

expectEqual(policy.classify({
  pathname: '/',
  region: 'content',
  signal: 'Unknown action'
}), '', 'unrelated page');

expectEqual(policy.classify({
  pathname: '/scheduled',
  region: 'header',
  signal: 'Unknown action'
}), '', 'scheduled non-content action');

console.log('CHATGPT_WEB_PAGE_SEMANTIC_POLICY=passed');

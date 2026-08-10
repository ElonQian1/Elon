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
  pathname: '/library',
  region: 'content',
  signal: 'Recent file'
}), 'library', 'library page action');

expectEqual(policy.classify({
  pathname: '/plugins/store',
  region: 'content',
  signal: 'Connector card'
}), 'apps', 'apps page action');

expectEqual(policy.classify({
  pathname: '/g/g-p-a1b2c3/project',
  region: 'content',
  signal: 'Project action'
}), 'project', 'project page action');

expectEqual(policy.classify({
  pathname: '/g/g-p-a1b2c3/c/conversation-id',
  region: 'content',
  signal: 'Unknown action'
}), '', 'project conversation remains unclassified');

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

expectEqual(policy.classify({
  pathname: '/',
  path: '/c/conversation_123',
  region: 'overlay',
  signal: 'Open chat options',
  isLink: false
}), 'conversation_options', 'conversation options');

expectEqual(policy.classify({
  pathname: '/',
  path: '/g/g-p-project_123/project',
  region: 'overlay',
  signal: 'Project row',
  isLink: true
}), 'project', 'project navigation row');

expectEqual(policy.classify({ pathname: '/', path: '/', signal: '主页', isLink: true }), 'home', 'home');
expectEqual(policy.classify({ pathname: '/', signal: '插件' }), 'apps', 'apps');
expectEqual(policy.classify({ pathname: '/', signal: '已置顶' }), 'pinned', 'pinned');
expectEqual(policy.classify({ pathname: '/', signal: '下载应用' }), 'download_app', 'download app');
expectEqual(policy.classify({ pathname: '/', signal: '整理聊天' }), 'conversation_group', 'chat group');

console.log('CHATGPT_WEB_PAGE_SEMANTIC_POLICY=passed');

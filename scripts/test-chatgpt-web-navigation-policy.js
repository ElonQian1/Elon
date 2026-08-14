'use strict';

const policy = require(
  '../android/app/src/main/assets/chatgpt_web_adapter_navigation_policy.js'
);

function expectEqual(actual, expected, name) {
  if (actual !== expected) {
    throw new Error(`${name}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}

expectEqual(
  policy.classify('Project workspace', '/g/g-p-demo_123/project'),
  'projects',
  'project root route'
);
expectEqual(
  policy.classify('Project workspace', '/g/g-p-demo_123'),
  'projects',
  'project shorthand route'
);
expectEqual(
  policy.classify('Project conversation', '/g/g-p-demo_123/c/conversation_456'),
  'navigation',
  'project conversation is not a feature page'
);
expectEqual(
  policy.classify('Projects and recent conversations', ''),
  'navigation',
  'project disclosure header is not a feature page'
);
expectEqual(policy.classify('Library', '/library'), 'library', 'library route');
expectEqual(
  policy.classify('sidebar-item-recall', ''),
  'library',
  'current official recall test id maps to library'
);
expectEqual(policy.classify('Tasks', '/scheduled'), 'tasks', 'task route');
expectEqual(policy.classify('Apps', '/plugins'), 'apps', 'apps route');
expectEqual(policy.classify('Health', '/health'), 'health', 'health route');
expectEqual(policy.classify('个人财务', '/finances'), 'finances', 'finances route');
expectEqual(policy.classify('工作', '/work'), 'work', 'work route');
expectEqual(policy.isConversationPath('/c/demo_123'), true, 'normal conversation path');
expectEqual(
  policy.isConversationPath('/g/g-p-demo_123/c/conversation_456'),
  true,
  'project conversation path'
);
expectEqual(policy.isProjectRoute('/g/g-p-demo_123/project'), true, 'project route predicate');
expectEqual(policy.isProjectRoute('/g/g-p-demo_123/c/conversation_456'), false, 'conversation route predicate');

console.log('CHATGPT_WEB_NAVIGATION_POLICY=passed');

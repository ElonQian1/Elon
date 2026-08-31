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
  pathname: '/images',
  region: 'content',
  signal: 'Open generated image'
}), 'images', 'image gallery page action');

expectEqual(policy.classify({
  pathname: '/health',
  path: '',
  region: 'content'
}), 'health', 'health page action');

expectEqual(policy.classify({
  pathname: '/finances',
  path: '',
  region: 'content'
}), 'finances', 'finances page action');

expectEqual(policy.classify({
  pathname: '/work',
  path: '',
  region: 'content'
}), 'work', 'work page action');

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
  pathname: '/c/current_conversation_123',
  region: 'header',
  signal: 'More options'
}), 'conversation_options', 'current conversation header options');
expectEqual(policy.conversationContextId({
  semantic: 'conversation_options',
  region: 'header',
  pathname: '/c/Current_conversation_123'
}), 'Current_conversation_123', 'header options inherit the current conversation context');
expectEqual(policy.conversationContextId({
  semantic: 'conversation_options',
  region: 'overlay',
  path: '/c/Sidebar_conversation_456',
  pathname: '/c/current_conversation_123'
}), 'Sidebar_conversation_456', 'sidebar options retain their related conversation context');
expectEqual(policy.conversationContextId({
  semantic: 'more',
  region: 'header',
  pathname: '/c/current_conversation_123'
}), '', 'generic header controls do not inherit conversation context');
expectEqual(policy.selectRelatedConversationPath({
  triggerLabel: '打开“今晚行情分析”的对话选项',
  candidates: [
    { path: '/c/other_conversation', label: '其他会话' },
    { path: '/c/market_conversation', label: '今晚行情分析' }
  ]
}), '/c/market_conversation', 'sidebar options recover their path from the referenced row label');
expectEqual(policy.selectRelatedConversationPath({
  triggerLabel: '打开对话选项',
  candidates: [
    { path: '/c/first_conversation', label: '第一条' },
    { path: '/c/second_conversation', label: '第二条' }
  ]
}), '', 'ambiguous sidebar options fail closed');

expectEqual(policy.classify({
  pathname: '/',
  path: '/g/g-p-project_123/project',
  region: 'overlay',
  signal: 'Project row',
  isLink: true
}), 'project', 'project navigation row');

expectEqual(policy.classify({
  pathname: '/c/conversation_123',
  region: 'content',
  signal: 'project-save-turn-action-button'
}), 'save_to_project', 'save response to project test id');

expectEqual(policy.classify({
  pathname: '/c/conversation_123',
  region: 'content',
  signal: 'Save to project'
}), 'save_to_project', 'save response to project label');

expectEqual(policy.classify({
  pathname: '/c/conversation_123',
  region: 'overlay',
  signal: 'menu-item',
  label: 'Add to project'
}), 'save_to_project', 'conversation add-to-project menu label');

expectEqual(policy.classify({
  pathname: '/c/conversation_123',
  region: 'overlay',
  signal: '菜单项',
  label: '移动到项目'
}), 'save_to_project', 'conversation move-to-project menu label');

expectEqual(policy.classify({ pathname: '/', path: '/', signal: '主页', isLink: true }), 'home', 'home');
expectEqual(policy.classify({ pathname: '/', signal: '插件' }), 'apps', 'apps');
expectEqual(policy.classify({ pathname: '/', signal: '已置顶' }), 'pinned', 'pinned');
expectEqual(policy.classify({ pathname: '/', signal: '下载应用' }), 'download_app', 'download app');
expectEqual(policy.classify({ pathname: '/', signal: '整理聊天' }), 'conversation_group', 'chat group');
expectEqual(policy.classify({ pathname: '/', signal: '临时聊天' }), 'temporary_chat', 'temporary chat');
expectEqual(policy.classify({ pathname: '/', signal: 'Temporary chat' }), 'temporary_chat', 'temporary chat English');
expectEqual(policy.classify({ pathname: '/', signal: 'Temporary conversation' }), 'temporary_chat', 'temporary conversation');
expectEqual(policy.classify({ pathname: '/', signal: '关闭临时聊天' }), 'temporary_chat', 'close temporary chat');
expectEqual(policy.classify({ pathname: '/', signal: 'Exit temporary chat' }), 'temporary_chat', 'exit temporary chat English');
const inactiveTemporaryChat = policy.temporaryChatState({ signal: '临时聊天' });
expectEqual(inactiveTemporaryChat.semantic, 'temporary_chat', 'temporary chat inactive semantic');
expectEqual(inactiveTemporaryChat.selected, false, 'temporary chat inactive state');
expectEqual(inactiveTemporaryChat.stateSettable, true, 'temporary chat inactive settable state');
const activeTemporaryChat = policy.temporaryChatState({ signal: '关闭临时聊天' });
expectEqual(activeTemporaryChat.semantic, 'temporary_chat', 'temporary chat active semantic');
expectEqual(activeTemporaryChat.selected, true, 'temporary chat active state');
expectEqual(activeTemporaryChat.stateSettable, true, 'temporary chat active settable state');
expectEqual(policy.temporaryChatState({ signal: '关闭设置' }), null, 'unrelated close is not temporary chat');
expectEqual(
  policy.planTemporaryChatSelection(false, false).needsActivation,
  false,
  'temporary chat repeated inactive state is idempotent'
);
expectEqual(
  policy.planTemporaryChatSelection(true, true).needsActivation,
  false,
  'temporary chat repeated active state is idempotent'
);
expectEqual(
  policy.planTemporaryChatSelection(false, true).needsActivation,
  true,
  'temporary chat activates once when desired state differs'
);
expectEqual(
  policy.planTemporaryChatSelection(true, false).needsActivation,
  true,
  'temporary chat deactivates once when desired state differs'
);
expectEqual(
  policy.planTemporaryChatSelection(null, true).ok,
  false,
  'temporary chat unknown current state fails closed'
);
expectEqual(policy.classify({
  pathname: '/', region: 'overlay', signal: 'Personalization'
}), 'personalization', 'account personalization');
expectEqual(policy.classify({
  pathname: '/', region: 'overlay', signal: '个人资料'
}), 'profile', 'account profile');
expectEqual(policy.classify({
  pathname: '/', region: 'overlay', signal: '帮助'
}), 'help', 'account help');
expectEqual(policy.classify({
  pathname: '/', region: 'overlay', signal: '退出登录'
}), 'logout', 'account logout');
expectEqual(policy.classify({
  pathname: '/', region: 'overlay', signal: 'Manage subscription'
}), 'plan', 'account plan');
expectEqual(policy.classify({
  pathname: '/', region: 'overlay', signal: 'Account owner Pro'
}), 'plan', 'account pro plan');
expectEqual(policy.classify({
  pathname: '/', region: 'overlay', signal: 'account menuitem', label: 'Example User Pro'
}), 'plan', 'account tier row');
expectEqual(policy.classify({
  pathname: '/', region: 'content', signal: 'Manage subscription'
}), '', 'account semantics stay overlay scoped');
expectEqual(policy.classify({
  pathname: '/c/example', region: 'overlay', label: '6月1日，11:01'
}), 'timestamp', 'Chinese message timestamp');
expectEqual(policy.classify({
  pathname: '/c/example', region: 'overlay', label: 'June 1, 2026, 11:01 AM'
}), 'timestamp', 'English message timestamp');
expectEqual(policy.classify({
  pathname: '/c/example', region: 'content', label: '6月1日，11:01'
}), '', 'timestamp semantics stay overlay scoped');
expectEqual(policy.classify({
  pathname: '/',
  signal: 'sidebar-section-toggle account-owned-id 整理聊天',
  label: '整理聊天'
}), 'conversation_group', 'chat group uses its visible label');
expectEqual(policy.classify({
  pathname: '/',
  signal: 'Dynamic account-owned name',
  section: '项目'
}), 'project', 'dynamic project row');

console.log('CHATGPT_WEB_PAGE_SEMANTIC_POLICY=passed');

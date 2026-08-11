(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptNavigationPolicy = Object.freeze(policy);
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  function clean(value) {
    return String(value || '').replace(/\s+/g, ' ').trim().toLowerCase();
  }

  function isConversationPath(path) {
    const value = clean(path);
    return /^\/c\/[a-z0-9_-]+$/.test(value) ||
      /^\/g\/g-p-[a-z0-9_-]+\/c\/[a-z0-9_-]+(?:\/|$)/.test(value);
  }

  function isProjectRoute(path) {
    const value = clean(path);
    return /^\/projects?(?:\/|$)/.test(value) ||
      /^\/g\/g-p-[a-z0-9_-]+(?:\/project)?\/?$/.test(value);
  }

  function classify(label, path) {
    const value = clean(label + ' ' + path);
    if (isConversationPath(path)) return 'navigation';
    if (isProjectRoute(path)) return 'projects';
    if (/library|文件库|资料库/.test(value)) return 'library';
    if (/scheduled|schedule|task|已安排|任务/.test(value)) return 'tasks';
    if (/\bgpt(s)?\b|探索.?gpt|发现.?gpt/.test(value)) return 'gpts';
    if (/memory|记忆/.test(value)) return 'memory';
    if (/plugin|connector|app(s)?\b|插件|应用/.test(value)) return 'apps';
    if (/setting|account|profile|设置|账号|账户/.test(value)) return 'settings';
    if (/more|更多/.test(value)) return 'more';
    return 'navigation';
  }

  return Object.freeze({ classify, isConversationPath, isProjectRoute });
});

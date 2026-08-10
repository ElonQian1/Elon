(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptPageSemanticPolicy = Object.freeze(policy);
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  function clean(value) {
    return String(value || '').replace(/\s+/g, ' ').trim().toLowerCase();
  }

  const contentRoutes = Object.freeze([
    { pattern: /^\/scheduled(?:\/|$)/, semantic: 'tasks' },
    { pattern: /^\/library(?:\/|$)/, semantic: 'library' },
    { pattern: /^\/plugins(?:\/|$)/, semantic: 'apps' },
    { pattern: /^\/g\/g-p-[a-z0-9_-]+\/project(?:\/|$)/, semantic: 'project' }
  ]);

  function classify(input) {
    const pathname = clean(input && input.pathname);
    const path = clean(input && input.path);
    const region = clean(input && input.region);
    const signal = clean(input && input.signal);
    const label = clean(input && input.label);
    const context = clean(input && input.context);
    const section = clean(input && input.section);
    const combined = [signal, context, path].filter(Boolean).join(' ');

    if (/open[-\s]?sidebar|open sidebar|打开(?:侧边栏|边栏)/.test(signal)) {
      return 'navigation';
    }
    if (/^\/c\/[a-z0-9_-]{1,160}$/.test(path) && !(input && input.isLink)) {
      return 'conversation_options';
    }
    if (/^\/g\/g-p-[a-z0-9_-]+(?:\/project)?$/.test(path) || /project|项目/.test(context)) {
      return 'project';
    }
    if (path === '/' && input && input.isLink) return 'home';
    if (/download\s+(?:the\s+)?(?:chatgpt\s+)?app|下载(?:\s*chatgpt)?应用/.test(combined)) {
      return 'download_app';
    }
    if (region === 'overlay') {
      if (/personalization|personalise|personalize|个性化/.test(combined)) return 'personalization';
      if (/profile|personal\s+(?:info|details)|个人资料|个人信息/.test(signal)) return 'profile';
      if (/log\s*out|sign\s*out|退出登录|登出/.test(combined)) return 'logout';
      if (/help|support|帮助|支持/.test(combined)) return 'help';
      if (/upgrade|subscription|manage\s+plan|\bpro\b|套餐|订阅/.test(combined)) return 'plan';
    }
    if (/plugin|connector|\bapps?\b|插件|应用/.test(combined)) return 'apps';
    if (/\bpinned\b|已置顶|置顶内容/.test(combined)) return 'pinned';
    if (/^(?:chats?|聊天|整理聊天)$/.test(label || signal)) return 'conversation_group';
    if (/^(?:projects?|项目)$/.test(section)) return 'project';
    if (region === 'content') {
      const route = contentRoutes.find((candidate) => candidate.pattern.test(pathname));
      if (route) return route.semantic;
    }
    return '';
  }

  return Object.freeze({ classify });
});

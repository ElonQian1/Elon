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
    { pattern: /^\/health(?:\/|$)/, semantic: 'health' },
    { pattern: /^\/finances?(?:\/|$)/, semantic: 'finances' },
    { pattern: /^\/work(?:\/|$)/, semantic: 'work' },
    { pattern: /^\/scheduled(?:\/|$)/, semantic: 'tasks' },
    { pattern: /^\/library(?:\/|$)/, semantic: 'library' },
    { pattern: /^\/plugins(?:\/|$)/, semantic: 'apps' },
    { pattern: /^\/g\/g-p-[a-z0-9_-]+\/project(?:\/|$)/, semantic: 'project' }
  ]);

  function isTimestampLabel(value) {
    const text = clean(value);
    return /timestamp|消息时间/.test(text) ||
      /(?:today|yesterday|今天|昨天)[,，]?\s*\d{1,2}:\d{2}(?:\s*[ap]\.?m\.?)?/.test(text) ||
      /(?:\d{4}年)?\d{1,2}月\d{1,2}日[,，]?\s*\d{1,2}:\d{2}/.test(text) ||
      /\d{4}[\/-]\d{1,2}[\/-]\d{1,2}[,，\s]+\d{1,2}:\d{2}/.test(text) ||
      /(?:jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:tember)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)\s+\d{1,2}(?:,\s*\d{4})?[,\s]+\d{1,2}:\d{2}(?:\s*[ap]\.?m\.?)?/.test(text);
  }

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
      if (isTimestampLabel(label || signal)) return 'timestamp';
      if (/\bprofile\b|personal\s+(?:details|info)|个人(?:资料|信息)/.test(combined)) return 'profile';
      if (/personalization|personalise|personalize|个性化/.test(combined)) return 'personalization';
      if (/log\s*out|sign\s*out|退出登录|登出/.test(combined)) return 'logout';
      if (/help|support|帮助|支持/.test(combined)) return 'help';
      if (
        /upgrade|subscription|manage\s+plan|套餐|订阅/.test(combined) ||
        /\b(?:pro|plus|business|team|enterprise)\b|(?:专业|团队|企业)版/.test(combined + ' ' + label)
      ) return 'plan';
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

  return Object.freeze({ classify, isTimestampLabel });
});

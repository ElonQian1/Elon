(function (root, factory) {
  'use strict';

  const projectPolicy = typeof module !== 'undefined' && module.exports
    ? require('./chatgpt_web_adapter_project_policy.js')
    : root && root.__elonChatGptProjectPolicy;
  const policy = factory(projectPolicy);
  if (typeof module !== 'undefined' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptPageSemanticPolicy = Object.freeze(policy);
})(typeof window !== 'undefined' ? window : null, function (projectPolicy) {
  'use strict';

  function clean(value) {
    return String(value || '').replace(/\s+/g, ' ').trim().toLowerCase();
  }

  function conversationIdentity(value) {
    return projectPolicy && typeof projectPolicy.conversationId === 'function'
      ? projectPolicy.conversationId(value)
      : '';
  }

  const contentRoutes = Object.freeze([
    { pattern: /^\/health(?:\/|$)/, semantic: 'health' },
    { pattern: /^\/finances?(?:\/|$)/, semantic: 'finances' },
    { pattern: /^\/work(?:\/|$)/, semantic: 'work' },
    { pattern: /^\/scheduled(?:\/|$)/, semantic: 'tasks' },
    { pattern: /^\/images?(?:\/|$)/, semantic: 'images' },
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

  function temporaryChatState(input) {
    const value = clean([
      input && input.signal,
      input && input.label
    ].filter(Boolean).join(' '));
    if (!/temporary\s+(?:chat|conversation)|临时(?:聊天|对话)|暫時聊天/.test(value)) {
      return null;
    }
    return Object.freeze({
      semantic: 'temporary_chat',
      selected: /(?:close|exit|leave|end|disable|turn\s+off)\s+(?:the\s+)?temporary|关闭临时|退出临时|结束临时/.test(value),
      stateSettable: true
    });
  }

  function planTemporaryChatSelection(currentSelected, desiredSelected) {
    if (typeof currentSelected !== 'boolean' || typeof desiredSelected !== 'boolean') {
      return Object.freeze({ ok: false, needsActivation: false });
    }
    return Object.freeze({
      ok: true,
      needsActivation: currentSelected !== desiredSelected
    });
  }

  function conversationContextId(input) {
    const semantic = clean(input && input.semantic);
    if (semantic !== 'conversation' && semantic !== 'conversation_options') return '';
    const relatedPath = String(input && input.path || '').trim();
    const currentPath = String(input && input.pathname || '').trim();
    const relatedIdentity = conversationIdentity(relatedPath);
    if (relatedIdentity) return relatedIdentity;
    if (
      semantic === 'conversation_options' &&
      clean(input && input.region) === 'header' &&
      conversationIdentity(currentPath)
    ) return conversationIdentity(currentPath);
    return '';
  }

  function selectRelatedConversationPath(input) {
    const candidates = Array.isArray(input && input.candidates)
      ? input.candidates
      : [];
    const conversations = candidates
      .map((candidate) => ({
        path: String(candidate && candidate.path || '').trim(),
        label: clean(candidate && candidate.label)
      }))
      .filter((candidate) => conversationIdentity(candidate.path));
    const uniquePaths = Array.from(new Set(conversations.map((candidate) => candidate.path)));
    if (uniquePaths.length === 1) return uniquePaths[0];
    if (uniquePaths.length === 0) return '';

    const triggerLabel = clean(input && input.triggerLabel);
    const referencedLabel = triggerLabel
      .replace(/^(?:open|show)\s+[“\"']?/i, '')
      .replace(/[”\"']?\s*(?:conversation|chat)\s+(?:options|actions|menu)$/i, '')
      .replace(/^打开[“\"']?/, '')
      .replace(/[”\"']?的?(?:对话|聊天)?(?:选项|操作|菜单)$/, '')
      .trim();
    if (!referencedLabel) return '';
    const matches = conversations.filter((candidate) => candidate.label === referencedLabel);
    const matchingPaths = Array.from(new Set(matches.map((candidate) => candidate.path)));
    return matchingPaths.length === 1 ? matchingPaths[0] : '';
  }

  function classify(input) {
    const pathname = clean(input && input.pathname);
    const path = clean(input && input.path);
    const region = clean(input && input.region);
    const signal = clean(input && input.signal);
    const label = clean(input && input.label);
    const context = clean(input && input.context);
    const section = clean(input && input.section);
    const combined = [signal, label, context, path].filter(Boolean).join(' ');

    if (/open[-\s]?sidebar|open sidebar|打开(?:侧边栏|边栏)/.test(signal)) {
      return 'navigation';
    }
    if (
      conversationIdentity(pathname) &&
      region === 'header' &&
      /\bmore\b|options?|menu|更多|操作|菜单/.test(signal)
    ) return 'conversation_options';
    if (conversationIdentity(path) && !(input && input.isLink)) {
      return 'conversation_options';
    }
    if (
      /project-save-turn-action-button/.test(signal) ||
      /^(?:save|add|move)(?:\s+(?:this|the))?\s+(?:chat|conversation)?\s*(?:to|into)?\s*project$/.test(label) ||
      /^(?:save|add|move)(?:\s+(?:this|the))?\s+(?:chat|conversation)?\s*(?:to|into)?\s*project$/.test(signal) ||
      /^(?:保存|添加|移动|存入)(?:到|至)?项目$/.test(label) ||
      /^(?:保存|添加|移动|存入)(?:到|至)?项目$/.test(signal)
    ) {
      return 'save_to_project';
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
    const temporaryChat = temporaryChatState({ signal, label });
    if (temporaryChat) return temporaryChat.semantic;
    if (/^(?:projects?|项目)$/.test(section)) return 'project';
    if (region === 'content') {
      const route = contentRoutes.find((candidate) => candidate.pattern.test(pathname));
      if (route) return route.semantic;
    }
    return '';
  }

  return Object.freeze({
    classify,
    conversationIdentity,
    conversationContextId,
    isTimestampLabel,
    planTemporaryChatSelection,
    selectRelatedConversationPath,
    temporaryChatState
  });
});

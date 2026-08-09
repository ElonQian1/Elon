(function () {
  'use strict';

  if (window.__elonChatGptConversations || location.origin !== 'https://chatgpt.com') return;

  const MAX_CONVERSATIONS = 100;
  const CONVERSATION_PATH = /^\/c\/[A-Za-z0-9_-]{1,160}$/;

  function cleanText(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\s+/g, ' ').trim();
  }

  function label(node) {
    return cleanText([
      node && node.getAttribute('aria-label'),
      node && node.getAttribute('title'),
      node && node.textContent
    ].filter(Boolean).join(' ')).toLowerCase();
  }

  function conversationPath(node) {
    try {
      const url = new URL(node.getAttribute('href') || '', location.origin);
      return url.origin === location.origin && CONVERSATION_PATH.test(url.pathname)
        ? url.pathname
        : '';
    } catch {
      return '';
    }
  }

  function conversationLinks() {
    return Array.from(document.querySelectorAll('a[href*="/c/"]'))
      .filter((node) => conversationPath(node));
  }

  function readConversations() {
    const seen = new Set();
    return conversationLinks().map((node) => {
      const path = conversationPath(node);
      if (!path || seen.has(path)) return null;
      seen.add(path);
      const titleNode = node.querySelector('[title], [dir="auto"], span') || node;
      const title = cleanText(
        node.getAttribute('data-conversation-title')
          || titleNode.getAttribute('title')
          || titleNode.textContent
          || node.getAttribute('aria-label')
      ).slice(0, 160);
      if (!title) return null;
      return {
        id: path.slice(3),
        title,
        path,
        active: path === location.pathname || node.getAttribute('aria-current') === 'page'
      };
    }).filter(Boolean).slice(0, MAX_CONVERSATIONS);
  }

  function findSidebarButton(open) {
    const selector = open
      ? '[data-testid*="open-sidebar" i], button[aria-label*="open sidebar" i], button[aria-label*="打开边栏" i], button[aria-label*="打开侧边栏" i]'
      : '[data-testid*="close-sidebar" i], button[aria-label*="close sidebar" i], button[aria-label*="关闭边栏" i], button[aria-label*="关闭侧边栏" i]';
    const direct = document.querySelector(selector);
    if (direct) return direct;
    const needles = open
      ? ['open sidebar', '打开边栏', '打开侧边栏']
      : ['close sidebar', '关闭边栏', '关闭侧边栏'];
    return Array.from(document.querySelectorAll('button')).find((button) =>
      needles.some((needle) => label(button).includes(needle))
    ) || null;
  }

  function waitForConversations(onReady, onTimeout) {
    const started = Date.now();
    function poll() {
      const conversations = readConversations();
      if (conversations.length) return onReady(conversations);
      if (Date.now() - started >= 3000) return onTimeout();
      window.setTimeout(poll, 100);
    }
    poll();
  }

  function requestList(emitEvent, result) {
    const existing = readConversations();
    if (existing.length) {
      emitEvent({ type: 'conversation_snapshot', conversations: existing });
      return result('list_conversations', true, '');
    }

    const open = findSidebarButton(true);
    if (!open) return result('list_conversations', false, '未找到官网会话侧栏入口。');
    open.click();
    waitForConversations(
      (conversations) => {
        emitEvent({ type: 'conversation_snapshot', conversations });
        result('list_conversations', true, '');
        const close = findSidebarButton(false);
        if (close) close.click();
      },
      () => result('list_conversations', false, '官网会话列表尚未加载完成。')
    );
  }

  function openConversation(path, result) {
    if (!CONVERSATION_PATH.test(path)) {
      return result('open_conversation', false, '会话地址无效。');
    }
    const target = conversationLinks().find((node) => conversationPath(node) === path);
    result('open_conversation', true, '');
    if (target) target.click();
    else location.assign(new URL(path, location.origin).href);
  }

  function capabilities() {
    const available = !!findSidebarButton(true) || conversationLinks().length > 0;
    return available ? ['conversation_list', 'conversation_search'] : [];
  }

  window.__elonChatGptConversations = Object.freeze({
    capabilities,
    openConversation,
    requestList
  });
})();

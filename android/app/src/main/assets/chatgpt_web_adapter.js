(function () {
  'use strict';

  if (window.__elonChatGptBridge || location.origin !== 'https://chatgpt.com') return;
  const nativeBridge = window.elonChatGptNative;
  if (!nativeBridge || typeof nativeBridge.postMessage !== 'function') return;

  const MAX_MESSAGES = 80;
  const MAX_MESSAGE_LENGTH = 40000;
  let emitTimer = 0;
  let lastSnapshot = '';
  let sequence = 0;

  function emitEvent(event) {
    nativeBridge.postMessage(JSON.stringify({
      schema: 'yilong.ai.ui.v1',
      providerId: 'chatgpt',
      source: 'official_web',
      conversationId: location.pathname,
      sequence: ++sequence,
      emittedAt: new Date().toISOString(),
      event
    }));
  }

  function cleanText(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\n{3,}/g, '\n\n').trim();
  }

  function comparableText(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\r\n/g, '\n').trim();
  }

  function messageRole(node) {
    const own = node.getAttribute('data-message-author-role');
    const nested = node.querySelector('[data-message-author-role]');
    const role = own || (nested && nested.getAttribute('data-message-author-role')) || '';
    return role === 'user' || role === 'assistant' ? role : '';
  }

  function messageContent(node) {
    const roleNode = node.matches('[data-message-author-role]')
      ? node
      : node.querySelector('[data-message-author-role]') || node;
    const content = roleNode.querySelector('.markdown, [data-message-content], .whitespace-pre-wrap');
    return cleanText((content || roleNode).innerText || (content || roleNode).textContent)
      .slice(0, MAX_MESSAGE_LENGTH);
  }

  function messageNodes() {
    const main = document.querySelector('main');
    if (!main) return [];
    const turns = Array.from(main.querySelectorAll('[data-testid^="conversation-turn-"]'));
    if (turns.length) return turns;
    return Array.from(main.querySelectorAll('[data-message-author-role]'));
  }

  function readMessages() {
    const seen = new Set();
    return messageNodes().slice(-MAX_MESSAGES).map((node, index) => {
      const role = messageRole(node);
      const content = messageContent(node);
      const baseId = node.getAttribute('data-message-id')
        || node.getAttribute('data-testid')
        || node.id
        || role + '-' + index;
      const id = seen.has(baseId) ? baseId + '-' + index : baseId;
      seen.add(id);
      return { id, role, state: 'completed', content: [{ type: 'text', text: content }] };
    }).filter((message) => message.role && message.content);
  }

  function isVisible(node) {
    if (!node) return false;
    const rect = node.getBoundingClientRect();
    const style = window.getComputedStyle(node);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
  }

  function findComposer() {
    const selectors = [
      '[data-testid="prompt-textarea"]',
      'form [contenteditable="true"]',
      'form textarea',
      'main [contenteditable="true"]',
      'textarea[placeholder]'
    ];
    for (const selector of selectors) {
      const match = Array.from(document.querySelectorAll(selector)).find(isVisible);
      if (match) return match;
    }
    return null;
  }

  function composerValue(composer) {
    if (!composer) return '';
    if (composer instanceof HTMLTextAreaElement || composer instanceof HTMLInputElement) {
      return String(composer.value || '');
    }
    return String(composer.innerText || composer.textContent || '');
  }

  function hasVisibleLoginEntry() {
    const loginLabels = new Set(['log in', 'login', 'sign in', '登录', '登入']);
    return Array.from(document.querySelectorAll('a, button, [role="button"]')).some((node) => {
      if (!isVisible(node)) return false;
      const label = cleanText(node.getAttribute('aria-label') || node.textContent).toLowerCase();
      const href = String(node.getAttribute('href') || '').toLowerCase();
      return loginLabels.has(label) || href.includes('/auth/login');
    });
  }

  function isAuthenticated() {
    const path = location.pathname.toLowerCase();
    if (path.startsWith('/auth') || path.startsWith('/cdn-cgi')) return false;
    const profile = document.querySelector(
      '[data-testid="profile-button"], [data-testid="accounts-profile-button"], button[aria-label*="profile" i]'
    );
    if (isVisible(profile)) return true;
    return !!findComposer() && !hasVisibleLoginEntry();
  }

  function findButton(testId, labels) {
    const direct = document.querySelector('[data-testid="' + testId + '"]');
    if (isVisible(direct)) return direct;
    const needles = labels.map((label) => label.toLowerCase());
    return Array.from(document.querySelectorAll('button')).find((button) => {
      if (!isVisible(button)) return false;
      const label = cleanText(button.getAttribute('aria-label') || button.textContent).toLowerCase();
      return needles.some((needle) => label.includes(needle));
    }) || null;
  }

  function isStreaming() {
    return !!findButton('stop-button', ['stop generating', 'stop', '停止生成', '停止']);
  }

  function snapshot() {
    const streaming = isStreaming();
    const messages = readMessages();
    if (streaming && messages.length && messages[messages.length - 1].role === 'assistant') {
      messages[messages.length - 1].state = 'streaming';
    }
    const event = {
      type: 'message_snapshot',
      title: cleanText(document.title.replace(/\s*[-|]\s*ChatGPT.*$/i, '')),
      url: location.origin + location.pathname,
      draft: composerValue(findComposer()).slice(0, 20000),
      messages,
      authenticated: isAuthenticated(),
      composerReady: !!findComposer(),
      streaming
    };
    const fingerprint = JSON.stringify(event);
    if (fingerprint === lastSnapshot) return;
    lastSnapshot = fingerprint;
    emitEvent(event);
  }

  function scheduleSnapshot() {
    clearTimeout(emitTimer);
    emitTimer = window.setTimeout(snapshot, 120);
  }

  function setComposerValue(composer, value) {
    composer.focus();
    if (composer instanceof HTMLTextAreaElement || composer instanceof HTMLInputElement) {
      const prototype = composer instanceof HTMLTextAreaElement
        ? HTMLTextAreaElement.prototype
        : HTMLInputElement.prototype;
      const setter = Object.getOwnPropertyDescriptor(prototype, 'value').set;
      setter.call(composer, value);
    } else {
      const selection = window.getSelection();
      const range = document.createRange();
      range.selectNodeContents(composer);
      selection.removeAllRanges();
      selection.addRange(range);
      const inserted = document.execCommand('insertText', false, value);
      if (!inserted) composer.replaceChildren(document.createTextNode(value));
    }
    composer.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText', data: value }));
    composer.dispatchEvent(new Event('change', { bubbles: true }));
    return comparableText(composerValue(composer)) === comparableText(value);
  }

  function result(action, ok, detail) {
    nativeBridge.postMessage(JSON.stringify({ type: 'command_result', action, ok, detail: detail || '' }));
  }

  function findSendButton(composer) {
    const scope = composer && composer.closest('form');
    const candidates = [
      scope && scope.querySelector('[data-testid="send-button"]'),
      scope && scope.querySelector('button[aria-label*="send" i]'),
      scope && scope.querySelector('button[type="submit"]'),
      findButton('send-button', ['send', '发送'])
    ];
    return candidates.find((button) =>
      isVisible(button) && !button.disabled && button.getAttribute('aria-disabled') !== 'true'
    ) || null;
  }

  function waitForReady(check, timeoutMs, onReady, onTimeout) {
    const started = Date.now();
    function poll() {
      const value = check();
      if (value) return onReady(value);
      if (Date.now() - started >= timeoutMs) return onTimeout();
      window.setTimeout(poll, 60);
    }
    poll();
  }

  function sendPrompt(value, expectedDraft) {
    const composer = findComposer();
    if (!composer) return result('send_prompt', false, '未找到输入框，请切换网页模式。');
    if (comparableText(composerValue(composer)) !== comparableText(expectedDraft)) {
      return result('send_prompt', false, '网页草稿已变化，请返回官网确认后重试。');
    }
    if (!setComposerValue(composer, value)) {
      return result('send_prompt', false, '官方输入框未接受文本，请返回官网重试。');
    }
    waitForReady(
      () => findSendButton(composer),
      2500,
      (button) => {
        result('send_prompt', true, '已交给官方网页发送。');
        button.click();
        scheduleSnapshot();
      },
      () => result('send_prompt', false, '发送按钮尚未就绪，请返回官网重试。')
    );
  }

  function startGoogleLogin() {
    const candidates = Array.from(document.querySelectorAll('button, a, [role="button"]'));
    const target = candidates.find((node) => {
      if (!isVisible(node)) return false;
      const label = cleanText([
        node.getAttribute('aria-label'),
        node.getAttribute('data-provider'),
        node.textContent
      ].filter(Boolean).join(' ')).toLowerCase();
      return label.includes('google');
    });
    if (!target) return result('start_google_login', false, '官方 Google 登录入口尚未就绪。');
    target.click();
    result('start_google_login', true, '');
  }

  function runCommand(raw) {
    let command = {};
    try { command = JSON.parse(String(raw || '{}')); }
    catch { return result('unknown', false, '命令格式无效。'); }
    const action = String(command.action || '');
    if (action === 'snapshot') return snapshot();
    if (action === 'send_prompt') {
      return sendPrompt(
        String(command.value || '').slice(0, 20000),
        String(command.expectedDraft || '').slice(0, 20000)
      );
    }
    if (action === 'start_google_login') return startGoogleLogin();
    if (action === 'stop_generation') {
      const stop = findButton('stop-button', ['stop generating', 'stop', '停止生成', '停止']);
      if (!stop) return result(action, false, '当前没有正在生成的回复。');
      stop.click();
      result(action, true, '');
      return scheduleSnapshot();
    }
    if (action === 'new_conversation') {
      if (comparableText(composerValue(findComposer()))) {
        return result(action, false, '网页中有未发送草稿，请先处理草稿。');
      }
      const button = findButton('create-new-chat-button', ['new chat', '新聊天', '新建聊天']);
      const link = Array.from(
        document.querySelectorAll('a[href="/"], a[data-testid="create-new-chat-button"]')
      ).find(isVisible);
      const target = button || link;
      if (!target) return result(action, false, '未找到新建会话入口。');
      result(action, true, '');
      target.click();
      return scheduleSnapshot();
    }
    result(action || 'unknown', false, '不支持的本地命令。');
  }

  window.__elonChatGptBridge = Object.freeze({ command: runCommand });
  emitEvent({
    type: 'adapter_ready',
    capabilities: ['streaming', 'conversation_history', 'draft_sync', 'google_login_entry']
  });
  new MutationObserver(scheduleSnapshot).observe(document.documentElement, {
    childList: true,
    subtree: true,
    characterData: true
  });
  window.addEventListener('popstate', scheduleSnapshot);
  snapshot();
})();

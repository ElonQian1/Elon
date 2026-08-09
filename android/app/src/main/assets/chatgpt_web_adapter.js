(function () {
  'use strict';

  if (window.__elonChatGptBridge || location.origin !== 'https://chatgpt.com') return;
  const nativeBridge = window.elonChatGptNative;
  if (!nativeBridge || typeof nativeBridge.postMessage !== 'function') return;
  const conversationAdapter = window.__elonChatGptConversations;
  const messageAdapter = window.__elonChatGptMessages;
  const composerAdapter = window.__elonChatGptComposer;

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
    const direct = testId ? document.querySelector('[data-testid="' + testId + '"]') : null;
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

  function detectCapabilities() {
    const capabilities = [
      'streaming',
      'conversation_history',
      'draft_sync',
      'new_conversation',
      'google_login_entry'
    ];
    if (conversationAdapter) capabilities.push(...conversationAdapter.capabilities());
    if (messageAdapter) capabilities.push(...messageAdapter.capabilities());
    if (composerAdapter) capabilities.push(...composerAdapter.capabilities(findComposer()));
    return Array.from(new Set(capabilities));
  }

  function snapshot() {
    const streaming = isStreaming();
    const messages = messageAdapter ? messageAdapter.readMessages(streaming) : [];
    const event = {
      type: 'message_snapshot',
      title: cleanText(document.title.replace(/\s*[-|]\s*ChatGPT.*$/i, '')),
      url: location.origin + location.pathname,
      draft: composerValue(findComposer()).slice(0, 20000),
      messages,
      authenticated: isAuthenticated(),
      composerReady: !!findComposer(),
      streaming,
      currentModel: composerAdapter ? composerAdapter.currentModel(findComposer()) : '',
      attachments: composerAdapter ? composerAdapter.readAttachments(findComposer()) : [],
      dictationActive: composerAdapter ? composerAdapter.dictationActive(findComposer()) : false,
      capabilities: detectCapabilities()
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
    if (action === 'list_model_options' && composerAdapter) {
      return composerAdapter.requestOptions('model', findComposer(), emitEvent, result);
    }
    if (action === 'list_composer_tools' && composerAdapter) {
      return composerAdapter.requestOptions('tools', findComposer(), emitEvent, result);
    }
    if (action === 'collect_model_options' && composerAdapter) {
      return composerAdapter.collectRequestedOptions('model', findComposer(), emitEvent, result);
    }
    if (action === 'collect_composer_tools' && composerAdapter) {
      return composerAdapter.collectRequestedOptions('tools', findComposer(), emitEvent, result);
    }
    if (action === 'select_model_option' && composerAdapter) {
      return composerAdapter.selectOption(
        'model', String(command.value || ''), findComposer(), emitEvent, result, scheduleSnapshot
      );
    }
    if (action === 'select_composer_tool' && composerAdapter) {
      return composerAdapter.selectOption(
        'tools', String(command.value || ''), findComposer(), emitEvent, result, scheduleSnapshot
      );
    }
    if (action === 'open_model_selector' && composerAdapter) {
      return composerAdapter.openOfficial('model', findComposer(), emitEvent, result);
    }
    if (action === 'open_composer_tools' && composerAdapter) {
      return composerAdapter.openOfficial('tools', findComposer(), emitEvent, result);
    }
    if (action === 'start_dictation' && composerAdapter) {
      return composerAdapter.startDictation(findComposer(), emitEvent, result);
    }
    if (action === 'remove_attachment' && composerAdapter) {
      return composerAdapter.removeAttachment(String(command.value || ''), emitEvent, result);
    }
    if (action === 'dismiss_composer_menu' && composerAdapter) {
      return composerAdapter.dismissOpenMenu(result);
    }
    if (action === 'list_conversations' && conversationAdapter) {
      return conversationAdapter.requestList(emitEvent, result);
    }
    if (action === 'open_conversation' && conversationAdapter) {
      if (comparableText(composerValue(findComposer()))) {
        return result(action, false, '网页中有未发送草稿，请先处理草稿。');
      }
      return conversationAdapter.openConversation(String(command.value || ''), result);
    }
    if (action === 'regenerate_response' && messageAdapter) {
      return messageAdapter.regenerate(result);
    }
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
    capabilities: detectCapabilities()
  });
  new MutationObserver(scheduleSnapshot).observe(document.documentElement, {
    childList: true,
    subtree: true,
    characterData: true
  });
  window.addEventListener('popstate', scheduleSnapshot);
  snapshot();
})();

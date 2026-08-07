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

  function findComposer() {
    return document.querySelector('[data-testid="prompt-textarea"], textarea[placeholder], form textarea, main [contenteditable="true"]');
  }

  function findButton(testId, labels) {
    const direct = document.querySelector('[data-testid="' + testId + '"]');
    if (direct) return direct;
    const needles = labels.map((label) => label.toLowerCase());
    return Array.from(document.querySelectorAll('button')).find((button) => {
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
      url: location.href,
      messages,
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
      composer.replaceChildren(document.createTextNode(value));
    }
    composer.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText', data: value }));
    composer.dispatchEvent(new Event('change', { bubbles: true }));
  }

  function result(action, ok, detail) {
    nativeBridge.postMessage(JSON.stringify({ type: 'command_result', action, ok, detail: detail || '' }));
  }

  function sendPrompt(value) {
    const composer = findComposer();
    if (!composer) return result('send_prompt', false, '未找到输入框，请切换网页模式。');
    setComposerValue(composer, value);
    window.setTimeout(() => {
      const button = findButton('send-button', ['send', '发送']);
      if (!button || button.disabled) return result('send_prompt', false, '发送按钮尚未就绪。');
      button.click();
      result('send_prompt', true, '');
      scheduleSnapshot();
    }, 80);
  }

  function runCommand(raw) {
    let command = {};
    try { command = JSON.parse(String(raw || '{}')); }
    catch { return result('unknown', false, '命令格式无效。'); }
    const action = String(command.action || '');
    if (action === 'snapshot') return snapshot();
    if (action === 'send_prompt') return sendPrompt(String(command.value || '').slice(0, 20000));
    if (action === 'stop_generation') {
      const stop = findButton('stop-button', ['stop generating', 'stop', '停止生成', '停止']);
      if (!stop) return result(action, false, '当前没有正在生成的回复。');
      stop.click();
      result(action, true, '');
      return scheduleSnapshot();
    }
    if (action === 'new_conversation') {
      const button = findButton('create-new-chat-button', ['new chat', '新聊天', '新建聊天']);
      const link = document.querySelector('a[href="/"], a[data-testid="create-new-chat-button"]');
      const target = button || link;
      if (!target) return result(action, false, '未找到新建会话入口。');
      target.click();
      result(action, true, '');
      return scheduleSnapshot();
    }
    result(action || 'unknown', false, '不支持的本地命令。');
  }

  window.__elonChatGptBridge = Object.freeze({ command: runCommand });
  emitEvent({ type: 'adapter_ready', capabilities: ['streaming', 'conversation_history'] });
  new MutationObserver(scheduleSnapshot).observe(document.documentElement, {
    childList: true,
    subtree: true,
    characterData: true
  });
  window.addEventListener('popstate', scheduleSnapshot);
  snapshot();
})();

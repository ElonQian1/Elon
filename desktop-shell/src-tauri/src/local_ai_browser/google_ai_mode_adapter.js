(function () {
  'use strict';

  const ALLOWED_ORIGINS = new Set(['https://google.com', 'https://www.google.com']);
  if (window.__elonGoogleAiModeBridge || !ALLOWED_ORIGINS.has(location.origin)) return;
  const nativeBridge = window.elonGoogleAiModeNative;
  if (!nativeBridge || typeof nativeBridge.postMessage !== 'function') return;

  let emitTimer = 0;
  let lastSnapshot = '';
  let sequence = 0;

  function emitEvent(event) {
    nativeBridge.postMessage(JSON.stringify({
      schema: 'yilong.ai.ui.v1',
      providerId: 'google-ai-mode',
      source: 'official_web',
      conversationId: location.pathname,
      sequence: ++sequence,
      emittedAt: new Date().toISOString(),
      event
    }));
  }

  function cleanText(value) {
    return String(value || '')
      .replace(/\u00a0/g, ' ')
      .replace(/[ \t]+\n/g, '\n')
      .replace(/\n{3,}/g, '\n\n')
      .trim();
  }

  function isVisible(node) {
    if (!(node instanceof Element)) return false;
    const rect = node.getBoundingClientRect();
    const style = window.getComputedStyle(node);
    return rect.width > 0 && rect.height > 0
      && style.display !== 'none' && style.visibility !== 'hidden';
  }

  function isAiModePage() {
    if (location.pathname === '/aimode') return true;
    if (location.pathname !== '/search') return false;
    const params = new URLSearchParams(location.search);
    return params.get('udm') === '50' || params.get('aep') === '11';
  }

  function findComposer() {
    if (!isAiModePage()) return null;
    const selectors = [
      'main textarea',
      'main [role="searchbox"]',
      'main [role="textbox"][contenteditable="true"]',
      'form textarea',
      'form [role="searchbox"]',
      'form [contenteditable="true"]',
      'textarea[placeholder]'
    ];
    for (const selector of selectors) {
      const matches = Array.from(document.querySelectorAll(selector)).filter(isVisible);
      const preferred = matches.find((node) => {
        const label = cleanText([
          node.getAttribute('aria-label'),
          node.getAttribute('placeholder')
        ].filter(Boolean).join(' ')).toLowerCase();
        return /ask|search|anything|follow.?up|提问|搜索|追问|输入/.test(label);
      });
      if (preferred || matches[0]) return preferred || matches[0];
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

  function setComposerValue(composer, value) {
    composer.focus();
    if (composer instanceof HTMLTextAreaElement || composer instanceof HTMLInputElement) {
      const prototype = composer instanceof HTMLTextAreaElement
        ? HTMLTextAreaElement.prototype
        : HTMLInputElement.prototype;
      const setter = Object.getOwnPropertyDescriptor(prototype, 'value');
      if (!setter || typeof setter.set !== 'function') return false;
      setter.set.call(composer, value);
    } else {
      composer.textContent = value;
    }
    composer.dispatchEvent(new InputEvent('input', {
      bubbles: true,
      composed: true,
      inputType: 'insertText',
      data: value
    }));
    composer.dispatchEvent(new Event('change', { bubbles: true }));
    return cleanText(composerValue(composer)) === cleanText(value);
  }

  function nodeLabel(node) {
    return cleanText([
      node.getAttribute('aria-label'),
      node.getAttribute('title'),
      node.textContent
    ].filter(Boolean).join(' ')).toLowerCase();
  }

  function findButton(labels, root) {
    const needles = labels.map((label) => label.toLowerCase());
    return Array.from((root || document).querySelectorAll('button, [role="button"]')).find((node) => {
      if (!isVisible(node) || node.matches(':disabled') || node.getAttribute('aria-disabled') === 'true') {
        return false;
      }
      const label = nodeLabel(node);
      return needles.some((needle) => label.includes(needle));
    }) || null;
  }

  function hasVisibleLoginEntry() {
    return Array.from(document.querySelectorAll('a, button, [role="button"]')).some((node) => {
      if (!isVisible(node)) return false;
      const label = nodeLabel(node);
      const href = String(node.getAttribute('href') || '').toLowerCase();
      return /^(sign in|log in|login|登录|登入)$/.test(label)
        || href.includes('accounts.google.com/servicelogin');
    });
  }

  function isAuthenticated() {
    const account = document.querySelector([
      'a[aria-label*="Google Account" i]',
      'button[aria-label*="Google Account" i]',
      '[data-ogsr-up]'
    ].join(','));
    return isVisible(account) || (!!findComposer() && !hasVisibleLoginEntry());
  }

  function isStreaming() {
    if (findButton(['stop response', 'stop generating', '停止回答', '停止生成', '停止'])) return true;
    return Array.from(document.querySelectorAll('main [aria-busy="true"], main [role="progressbar"]'))
      .some(isVisible);
  }

  function currentQuery() {
    const params = new URLSearchParams(location.search);
    const query = cleanText(params.get('q')).slice(0, 40000);
    if (query) return query;
    const heading = Array.from(document.querySelectorAll('main h1, main [data-user-query], main [data-query]'))
      .find(isVisible);
    return cleanText(heading && heading.textContent).slice(0, 40000);
  }

  function citationParts(container) {
    if (!container) return [];
    const seen = new Set();
    const parts = [];
    for (const link of container.querySelectorAll('a[href^="https://"]')) {
      if (!isVisible(link)) continue;
      try {
        const url = new URL(link.href);
        if (ALLOWED_ORIGINS.has(url.origin)) continue;
        const safeUrl = url.origin + url.pathname;
        if (seen.has(safeUrl)) continue;
        seen.add(safeUrl);
        parts.push({
          type: 'citation',
          title: cleanText(link.textContent || link.getAttribute('aria-label')).slice(0, 160),
          url: safeUrl.slice(0, 1200)
        });
        if (parts.length >= 12) break;
      } catch (_) {}
    }
    return parts;
  }

  function answerCandidate() {
    const selectors = [
      'main [data-container-id]',
      'main [data-snhf]',
      'main [data-attrid*="ai" i]',
      'main article',
      'main [role="region"]',
      '[role="main"] article',
      '[role="main"] [role="region"]'
    ];
    const candidates = Array.from(document.querySelectorAll(selectors.join(',')))
      .filter((node) => isVisible(node) && !node.querySelector('textarea, [contenteditable="true"]'))
      .map((node) => {
        const text = cleanText(node.innerText).slice(0, 40000);
        const citations = citationParts(node);
        const score = Math.min(text.length, 6000) + citations.length * 800;
        return { node, text, citations, score };
      })
      .filter((item) => item.text.length >= 40)
      .sort((left, right) => right.score - left.score || left.text.length - right.text.length);
    return candidates[0] || null;
  }

  function visibleMessages(streaming) {
    const query = currentQuery();
    const answer = answerCandidate();
    const messages = [];
    if (query) {
      messages.push({
        id: 'google-query-' + query.slice(0, 80),
        role: 'user',
        state: 'completed',
        content: [{ type: 'text', text: query }]
      });
    }
    if (answer) {
      let text = answer.text;
      if (query && text.startsWith(query)) text = cleanText(text.slice(query.length));
      const content = text ? [{ type: 'text', text }] : [];
      content.push(...answer.citations);
      if (content.length) {
        messages.push({
          id: 'google-answer-' + String(sequence + 1),
          role: 'assistant',
          state: streaming ? 'streaming' : 'completed',
          content
        });
      }
    }
    return messages;
  }

  function snapshot() {
    const composer = findComposer();
    const streaming = isStreaming();
    const event = {
      type: 'message_snapshot',
      title: cleanText(document.title.replace(/\s*[-|]\s*Google.*$/i, '')).slice(0, 160),
      url: location.origin + location.pathname,
      draft: composerValue(composer).slice(0, 20000),
      messages: visibleMessages(streaming),
      authenticated: isAuthenticated(),
      composerReady: !!composer,
      streaming,
      currentModel: 'Google AI 模式',
      capabilities: ['streaming', 'citations', 'new_conversation']
    };
    const fingerprint = JSON.stringify(event);
    if (fingerprint === lastSnapshot) return;
    lastSnapshot = fingerprint;
    emitEvent(event);
  }

  function scheduleSnapshot() {
    clearTimeout(emitTimer);
    emitTimer = window.setTimeout(snapshot, 320);
  }

  function result(action, ok, detail) {
    nativeBridge.postMessage(JSON.stringify({
      type: 'command_result',
      action,
      ok: !!ok,
      detail: cleanText(detail).slice(0, 240)
    }));
  }

  function sendPrompt(value, expectedDraft) {
    const composer = findComposer();
    if (!composer) return result('send_prompt', false, 'Google AI 模式输入框尚未就绪，请显示官方窗口确认功能可用。');
    const current = cleanText(composerValue(composer));
    if (current !== cleanText(expectedDraft)) {
      return result('send_prompt', false, '官方网页草稿已经变化，请返回官方窗口确认后再发送。');
    }
    if (!setComposerValue(composer, value)) {
      return result('send_prompt', false, 'Google 官方输入框未接受文本，请返回官方窗口重试。');
    }
    const form = composer.closest('form');
    const button = findButton(['send', 'submit', 'search', '发送', '提交', '搜索'], form || document);
    if (button) {
      result('send_prompt', true, '已交给 Google AI 模式官方网页发送。');
      button.click();
    } else if (form && typeof form.requestSubmit === 'function') {
      result('send_prompt', true, '已交给 Google AI 模式官方网页发送。');
      form.requestSubmit();
    } else {
      return result('send_prompt', false, 'Google AI 模式发送入口尚未就绪，请返回官方窗口重试。');
    }
    scheduleSnapshot();
  }

  function runCommand(raw) {
    let command = {};
    try { command = JSON.parse(String(raw || '{}')); }
    catch (_) { return result('unknown', false, '命令格式无效。'); }
    const action = String(command.action || '');
    if (action === 'snapshot') return snapshot();
    if (action === 'send_prompt') {
      return sendPrompt(
        String(command.value || '').slice(0, 20000),
        String(command.expectedDraft || '').slice(0, 20000)
      );
    }
    if (action === 'stop_generation') {
      const stop = findButton(['stop response', 'stop generating', '停止回答', '停止生成', '停止']);
      if (!stop) return result(action, false, '当前没有可停止的 Google AI 模式回复。');
      stop.click();
      result(action, true, '');
      return scheduleSnapshot();
    }
    if (action === 'new_conversation') {
      result(action, true, '');
      location.assign('https://www.google.com/aimode');
      return;
    }
    result(action || 'unknown', false, 'Google AI 模式不支持这个本地动作。');
  }

  window.__elonGoogleAiModeBridge = Object.freeze({ command: runCommand });
  emitEvent({
    type: 'adapter_ready',
    capabilities: ['streaming', 'citations', 'new_conversation']
  });
  new MutationObserver(scheduleSnapshot).observe(document.documentElement, {
    childList: true,
    subtree: true,
    characterData: true
  });
  window.addEventListener('popstate', scheduleSnapshot);
  snapshot();
})();

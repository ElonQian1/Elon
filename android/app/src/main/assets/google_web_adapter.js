(function () {
  'use strict';

  const allowedOrigins = new Set(['https://google.com', 'https://www.google.com']);
  const adapterVersion = Number(window.__elonGoogleWebAdapterVersion || 0);
  const nativeBridge = window.elonGoogleWebNative;
  if (!allowedOrigins.has(location.origin) || !adapterVersion || !nativeBridge ||
      typeof nativeBridge.postMessage !== 'function') return;
  if (window.__elonGoogleWebBridge && window.__elonGoogleWebBridge.version === adapterVersion) return;

  let sequence = 0;
  let emitTimer = 0;
  let lastSnapshot = '';

  function cleanText(value) {
    return String(value || '')
      .replace(/\u00a0/g, ' ')
      .replace(/[ \t]+\n/g, '\n')
      .replace(/\n{3,}/g, '\n\n')
      .trim();
  }

  function emitEvent(event) {
    nativeBridge.postMessage(JSON.stringify({
      schema: 'yilong.ai.ui.v1',
      adapterVersion,
      providerId: 'google_web',
      source: 'official_web',
      conversationId: location.pathname,
      sequence: ++sequence,
      emittedAt: new Date().toISOString(),
      event
    }));
  }

  function emitResult(action, ok, detail) {
    nativeBridge.postMessage(JSON.stringify({
      adapterVersion,
      type: 'command_result',
      action: String(action || '').slice(0, 40),
      ok: !!ok,
      detail: cleanText(detail).slice(0, 160)
    }));
  }

  function isVisible(node) {
    if (!(node instanceof Element)) return false;
    const rect = node.getBoundingClientRect();
    const style = window.getComputedStyle(node);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' &&
      style.visibility !== 'hidden';
  }

  function isAiModePage() {
    if (location.pathname === '/aimode') return true;
    if (location.pathname !== '/search') return false;
    const params = new URLSearchParams(location.search);
    return params.get('udm') === '50' || params.get('aep') === '11';
  }

  function nodeLabel(node) {
    return cleanText([
      node.getAttribute('aria-label'),
      node.getAttribute('title'),
      node.textContent
    ].filter(Boolean).join(' ')).toLowerCase();
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
      const preferred = matches.find((node) => /ask|search|anything|follow.?up|提问|搜索|追问|输入/i.test(
        cleanText([node.getAttribute('aria-label'), node.getAttribute('placeholder')]
          .filter(Boolean).join(' '))
      ));
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
        ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
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

  function isStreaming() {
    if (findButton(['stop response', 'stop generating', '停止回答', '停止生成', '停止'])) return true;
    return Array.from(document.querySelectorAll('main [aria-busy="true"], main [role="progressbar"]'))
      .some(isVisible);
  }

  function currentQuery() {
    const query = cleanText(new URLSearchParams(location.search).get('q')).slice(0, 40000);
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
        if (allowedOrigins.has(url.origin)) continue;
        const safeUrl = url.origin + url.pathname;
        if (seen.has(safeUrl)) continue;
        seen.add(safeUrl);
        const title = cleanText(link.textContent || link.getAttribute('aria-label')) || url.hostname;
        parts.push({
          type: 'citation',
          text: title.slice(0, 160),
          title: title.slice(0, 160),
          url: safeUrl.slice(0, 1200),
          targetKind: 'external',
          targetHost: url.hostname.slice(0, 253)
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
    return Array.from(document.querySelectorAll(selectors.join(',')))
      .filter((node) => isVisible(node) && !node.querySelector('textarea, [contenteditable="true"]'))
      .map((node) => {
        const text = cleanText(node.innerText).slice(0, 40000);
        const citations = citationParts(node);
        return { text, citations, score: Math.min(text.length, 6000) + citations.length * 800 };
      })
      .filter((item) => item.text.length >= 40)
      .sort((left, right) => right.score - left.score || left.text.length - right.text.length)[0] || null;
  }

  function visibleMessages(streaming) {
    const query = currentQuery();
    const answer = answerCandidate();
    const messages = [];
    if (query) messages.push({
      id: 'google-query-current',
      role: 'user',
      state: 'completed',
      content: [{ type: 'text', text: query }]
    });
    if (answer) {
      let text = answer.text;
      if (query && text.startsWith(query)) text = cleanText(text.slice(query.length));
      const content = text ? [{ type: 'text', text }] : [];
      content.push(...answer.citations);
      if (content.length) messages.push({
        id: 'google-answer-current',
        role: 'assistant',
        state: streaming ? 'streaming' : 'completed',
        content
      });
    }
    return messages;
  }

  function isAuthenticated() {
    return Array.from(document.querySelectorAll(
      'a[aria-label*="Google Account" i], button[aria-label*="Google Account" i], [data-ogsr-up]'
    )).some(isVisible);
  }

  function snapshot() {
    const composer = findComposer();
    const streaming = isStreaming();
    const event = {
      type: 'message_snapshot',
      title: cleanText(document.title.replace(/\s*[-|]\s*Google.*$/i, '')).slice(0, 160),
      url: location.href.slice(0, 8192),
      draft: composerValue(composer).slice(0, 20000),
      messages: visibleMessages(streaming),
      authenticated: isAuthenticated(),
      pageKind: isAiModePage() ? 'conversation' : 'unknown',
      loginRequired: false,
      composerReady: !!composer,
      streaming,
      currentModel: 'Google AI 模式',
      attachments: [],
      dictationActive: false,
      capabilities: ['streaming', 'citations', 'new_conversation', 'conversation_history']
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

  function sendPrompt(value, expectedDraft) {
    const composer = findComposer();
    if (!composer) return emitResult('send_prompt', false, 'Google AI 输入框尚未就绪。');
    if (cleanText(composerValue(composer)) !== cleanText(expectedDraft)) {
      return emitResult('send_prompt', false, '官方网页草稿已经变化，请确认后重试。');
    }
    if (!setComposerValue(composer, value)) {
      return emitResult('send_prompt', false, 'Google 官方输入框未接受文本。');
    }
    const form = composer.closest('form');
    const button = findButton(['send', 'submit', 'search', '发送', '提交', '搜索'], form || document);
    if (button) {
      emitResult('send_prompt', true, '');
      button.click();
    } else if (form && typeof form.requestSubmit === 'function') {
      emitResult('send_prompt', true, '');
      form.requestSubmit();
    } else {
      return emitResult('send_prompt', false, 'Google AI 发送入口尚未就绪。');
    }
    scheduleSnapshot();
  }

  function runCommand(raw) {
    let command = {};
    try { command = JSON.parse(String(raw || '{}')); }
    catch (_) { return emitResult('unknown', false, '命令格式无效。'); }
    const action = String(command.action || '');
    if (action === 'snapshot') return snapshot();
    if (action === 'send_prompt') return sendPrompt(
      String(command.value || '').slice(0, 20000),
      String(command.expectedDraft || '').slice(0, 20000)
    );
    if (action === 'stop_generation') {
      const stop = findButton(['stop response', 'stop generating', '停止回答', '停止生成', '停止']);
      if (!stop) return emitResult(action, false, '当前没有可停止的回答。');
      stop.click();
      emitResult(action, true, '');
      return scheduleSnapshot();
    }
    if (action === 'new_conversation') {
      emitResult(action, true, '');
      location.assign('https://www.google.com/aimode');
      return;
    }
    emitResult(action || 'unknown', false, 'Google AI 不支持这个本地动作。');
  }

  window.__elonGoogleWebBridge = Object.freeze({ version: adapterVersion, command: runCommand });
  emitEvent({
    type: 'adapter_ready',
    capabilities: ['streaming', 'citations', 'new_conversation', 'conversation_history']
  });
  new MutationObserver(scheduleSnapshot).observe(document.documentElement, {
    childList: true,
    subtree: true,
    characterData: true
  });
  window.addEventListener('popstate', scheduleSnapshot);
  snapshot();
})();

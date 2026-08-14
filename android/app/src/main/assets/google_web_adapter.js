(function () {
  'use strict';

  const allowedOrigins = new Set(['https://google.com', 'https://www.google.com']);
  const adapterVersion = Number(window.__elonGoogleWebAdapterVersion || 0);
  const documentToken = String(window.__elonGoogleWebDocumentToken || '');
  const nativeBridge = window.elonGoogleWebNative;
  const messageExtractor = window.__elonGoogleWebMessageExtractor;
  const composerBridge = window.__elonGoogleWebComposerBridge;
  const sendPolicy = window.__elonGoogleWebSendPolicy;
  if (!allowedOrigins.has(location.origin) || !adapterVersion ||
      !/^doc_[a-z0-9_]{3,80}$/.test(documentToken) || !nativeBridge ||
      typeof nativeBridge.postMessage !== 'function' || !messageExtractor || !composerBridge || !sendPolicy ||
      typeof messageExtractor.extract !== 'function') return;
  if (window.__elonGoogleWebBridge &&
      window.__elonGoogleWebBridge.version === adapterVersion &&
      window.__elonGoogleWebBridge.documentToken === documentToken) return;

  let sequence = 0;
  let emitTimer = 0;
  let lastSnapshot = '';
  let lastDiagnostics = '';

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
      documentToken,
      sequence: ++sequence,
      emittedAt: new Date().toISOString(),
      event
    }));
  }

  function emitResult(action, ok, detail) {
    nativeBridge.postMessage(JSON.stringify({
      adapterVersion,
      documentToken,
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
    if (location.pathname === '/webhp') {
      return new URLSearchParams(location.search).get('aep') === '11';
    }
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
    return composerBridge.find();
  }

  function composerValue(composer) {
    return composerBridge.value(composer);
  }

  function setComposerValue(composer, value) {
    return composerBridge.setValue(composer, value);
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

  function hasVisibleLoginEntry() {
    return Array.from(document.querySelectorAll('a, button, [role="button"]')).some((node) => {
      if (!isVisible(node)) return false;
      const label = nodeLabel(node);
      const href = String(node.getAttribute('href') || '').toLowerCase();
      return /(?:^|\s)(sign in|log in|login|登录|登入)(?:\s|$)/.test(label)
        || href.includes('accounts.google.com')
        || href.includes('/accounts/');
    });
  }

  function isAuthenticated() {
    const account = document.querySelector([
      'a[aria-label*="Google Account" i]',
      'button[aria-label*="Google Account" i]',
      'a[aria-label*="Google 账号" i]',
      'button[aria-label*="Google 账号" i]',
      'a[aria-label*="Google 帐号" i]',
      'button[aria-label*="Google 帐号" i]',
      'a[aria-label*="Google 帳戶" i]',
      'button[aria-label*="Google 帳戶" i]',
      'a[href*="SignOutOptions"]',
      'img[src*="googleusercontent.com"]'
    ].join(','));
    return isVisible(account) || (!!findComposer() && !hasVisibleLoginEntry());
  }

  function snapshot() {
    const composer = findComposer();
    const streaming = isStreaming();
    const extraction = messageExtractor.extract(composer, streaming);
    const event = {
      type: 'message_snapshot',
      title: cleanText(document.title.replace(/\s*[-|]\s*Google.*$/i, '')).slice(0, 160),
      url: location.href.slice(0, 8192),
      draft: composerValue(composer).slice(0, 20000),
      messages: extraction.messages,
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
    const diagnostics = messageExtractor.diagnostics(composer, extraction) + '|' +
      composerBridge.diagnostics();
    if (diagnostics && diagnostics !== lastDiagnostics) {
      lastDiagnostics = diagnostics;
      emitResult('dom_diagnostics', true, diagnostics);
    }
  }

  function scheduleSnapshot() {
    clearTimeout(emitTimer);
    emitTimer = window.setTimeout(snapshot, 320);
  }

  function sendPrompt(value, expectedDraft) {
    const composer = findComposer();
    if (!composer) return emitResult('send_prompt', false, 'Google AI 输入框尚未就绪。');
    const prompt = cleanText(value);
    const draft = cleanText(composerValue(composer));
    const reconciliation = sendPolicy.reconcile(draft, cleanText(expectedDraft), prompt);
    if (!reconciliation.allowed) {
      scheduleSnapshot();
      return emitResult('send_prompt', false, '官方网页草稿已经变化，请确认后重试。');
    }
    if (reconciliation.write && !setComposerValue(composer, value)) {
      return emitResult('send_prompt', false, 'Google 官方输入框未接受文本。');
    }
    const form = composerBridge.form(composer);
    const scope = form || composer.parentElement?.parentElement?.parentElement || composer.parentElement;
    const button = composerBridge.findAction(composer, ['send', 'submit', '发送', '提交']) ||
      findButton(['send', 'submit', '发送', '提交'], scope || document);
    const beforeHref = location.href;
    let submitted = false;
    if (button) {
      button.click();
      submitted = true;
    } else if (form && typeof form.requestSubmit === 'function') {
      form.requestSubmit();
      submitted = true;
    } else {
      submitted = composerBridge.pressEnter(composer);
    }
    if (!submitted) return emitResult('send_prompt', false, 'Google AI 发送入口尚未就绪。');

    const startedAt = Date.now();
    function confirmSubmission() {
      const currentComposer = findComposer();
      const streaming = isStreaming();
      const extraction = messageExtractor.extract(currentComposer, streaming);
      const userMessage = extraction.messages.find((message) => message.role === 'user');
      const queryMatches = !!userMessage && cleanText(
        (userMessage.content || []).map((part) => part.text || '').join('\n')
      ) === prompt;
      if (sendPolicy.confirmed({
        hrefChanged: location.href !== beforeHref,
        streaming,
        queryMatches,
        currentDraft: cleanText(composerValue(currentComposer)),
        prompt
      })) {
        messageExtractor.rememberQuery(prompt);
        emitResult('send_prompt', true, '');
        return scheduleSnapshot();
      }
      if (Date.now() - startedAt >= 8000) {
        scheduleSnapshot();
        return emitResult('send_prompt', false, 'Google 官方页未确认发送，请重试。');
      }
      window.setTimeout(confirmSubmission, 160);
    }
    window.setTimeout(confirmSubmission, 160);
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
      messageExtractor.clearRememberedQuery();
      emitResult(action, true, '');
      location.assign('https://www.google.com/aimode');
      return;
    }
    emitResult(action || 'unknown', false, 'Google AI 不支持这个本地动作。');
  }

  window.__elonGoogleWebBridge = Object.freeze({
    version: adapterVersion,
    documentToken,
    command: runCommand
  });
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

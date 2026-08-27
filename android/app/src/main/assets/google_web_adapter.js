(function () {
  'use strict';

  const allowedOrigins = new Set(['https://google.com', 'https://www.google.com']);
  const adapterVersion = Number(window.__elonGoogleWebAdapterVersion || 0);
  const documentToken = String(window.__elonGoogleWebDocumentToken || '');
  const nativeBridge = window.elonGoogleWebNative;
  const messageExtractor = window.__elonGoogleWebMessageExtractor;
  const composerBridge = window.__elonGoogleWebComposerBridge;
  const sendPolicy = window.__elonGoogleWebSendPolicy;
  const privateReplyObserver = window.__elonGoogleWebPrivateReplyObserver;
  const privateReplyReconciler = window.__elonGoogleWebPrivateReplyReconciler;
  const privateThreadDirectory = window.__elonGoogleWebPrivateThreadDirectory;
  const privateResearchTap = window.__elonGoogleWebPrivateResponseTap;
  if (!allowedOrigins.has(location.origin) || !adapterVersion ||
      !/^doc_[a-z0-9_]{3,80}$/.test(documentToken) || !nativeBridge ||
      typeof nativeBridge.postMessage !== 'function' || !messageExtractor || !composerBridge || !sendPolicy ||
      typeof messageExtractor.extract !== 'function' ||
      typeof messageExtractor.currentQueryMatches !== 'function' ||
      typeof messageExtractor.hasCurrentQuery !== 'function' ||
      !privateReplyReconciler || typeof privateReplyReconciler.apply !== 'function' ||
      typeof composerBridge.findSubmitAction !== 'function') return;
  if (window.__elonGoogleWebBridge &&
      window.__elonGoogleWebBridge.version === adapterVersion &&
      window.__elonGoogleWebBridge.documentToken === documentToken) return;

  let sequence = 0;
  let emitTimer = 0;
  let lastSnapshot = '';
  let lastDiagnostics = '';
  let lastPrivateReplyDiagnostics = '';
  let lastPrivateDirectorySnapshot = '';
  let observer = null;
  let disposed = false;
  const SUBMIT_READY_TIMEOUT_MS = 1600;
  const MAX_NAVIGATION_PROMPT_LENGTH = 4000;

  function cleanText(value) {
    return String(value || '')
      .replace(/\u00a0/g, ' ')
      .replace(/[ \t]+\n/g, '\n')
      .replace(/\n{3,}/g, '\n\n')
      .trim();
  }

  function emitEvent(event) {
    if (disposed) return;
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

  function emitResult(action, ok, detail, requestId) {
    if (disposed) return;
    const event = {
      adapterVersion,
      documentToken,
      type: 'command_result',
      action: String(action || '').slice(0, 40),
      ok: !!ok,
      detail: cleanText(detail).slice(0, 160)
    };
    if (/^mcp_[a-z0-9]{1,32}$/.test(String(requestId || ''))) event.requestId = requestId;
    nativeBridge.postMessage(JSON.stringify(event));
  }

  function emitPrivateDirectorySnapshot() {
    if (!privateThreadDirectory || typeof privateThreadDirectory.snapshot !== 'function') return;
    const conversations = privateThreadDirectory.snapshot();
    if (!Array.isArray(conversations) || !conversations.length) return;
    const fingerprint = JSON.stringify(conversations);
    if (fingerprint === lastPrivateDirectorySnapshot) return;
    lastPrivateDirectorySnapshot = fingerprint;
    emitEvent({
      type: 'conversation_snapshot',
      conversations: conversations.map((value) => ({
        id: value.id,
        title: value.title,
        path: value.path,
        providerUrl: value.providerUrl,
        active: location.href === value.providerUrl,
        groupLabel: 'Google AI 搜索',
        activityDates: []
      })),
      projects: [],
      collection: {
        observedCount: conversations.length,
        source: 'official_private',
        officialLoadState: 'ready'
      }
    });
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
    if (disposed) return;
    const composer = findComposer();
    const streaming = isStreaming();
    const extraction = messageExtractor.extract(composer, streaming);
    const privateReply = privateReplyObserver && typeof privateReplyObserver.snapshot === 'function'
      ? privateReplyObserver.snapshot()
      : null;
    if (privateReplyReconciler.apply(extraction.messages, privateReply, location.href)) {
      extraction.answerFound = true;
      extraction.observedMessageCount = extraction.messages.length;
    }
    const effectiveStreaming = streaming || !!(privateReply && privateReply.streaming);
    const event = {
      type: 'message_snapshot',
      title: cleanText(document.title.replace(/\s*[-|]\s*Google.*$/i, '')).slice(0, 160),
      url: location.href.slice(0, 8192),
      draft: composerValue(composer).slice(0, 20000),
      messages: extraction.messages,
      observedMessageCount: extraction.observedMessageCount,
      messageWindowStart: extraction.messageWindowStart,
      authenticated: isAuthenticated(),
      pageKind: isAiModePage() ? 'conversation' : 'unknown',
      loginRequired: false,
      composerReady: !!composer,
      streaming: effectiveStreaming,
      currentModel: 'Google AI 模式',
      attachments: [],
      dictationActive: false,
      capabilities: ['streaming', 'citations', 'rich_text', 'new_conversation', 'conversation_history']
    };
    const fingerprint = JSON.stringify(event);
    if (fingerprint !== lastSnapshot) {
      lastSnapshot = fingerprint;
      emitEvent(event);
      const diagnostics = messageExtractor.diagnostics(composer, extraction) + '|' +
        composerBridge.diagnostics();
      if (diagnostics && diagnostics !== lastDiagnostics) {
        lastDiagnostics = diagnostics;
        emitResult('dom_diagnostics', true, diagnostics);
      }
    }
    if (privateResearchTap && typeof privateResearchTap.drain === 'function') {
      privateResearchTap.drain().forEach((detail) => {
        emitResult('research_network_observation', true, detail);
      });
    }
    if (privateResearchTap && privateReplyObserver &&
        typeof privateReplyObserver.diagnostics === 'function') {
      const privateDiagnostics = privateReplyObserver.diagnostics();
      if (privateDiagnostics && privateDiagnostics !== lastPrivateReplyDiagnostics) {
        lastPrivateReplyDiagnostics = privateDiagnostics;
        emitResult('research_network_observation', true, 'v1|reply|' + privateDiagnostics);
      }
    }
  }

  function scheduleSnapshot() {
    if (disposed) return;
    clearTimeout(emitTimer);
    emitTimer = window.setTimeout(snapshot, 320);
  }

  function firstPromptNavigationUrl(prompt) {
    const url = new URL('https://www.google.com/search');
    url.searchParams.set('udm', '50');
    url.searchParams.set('aep', '11');
    url.searchParams.set('q', prompt);
    return url.href;
  }

  function sendPrompt(value, expectedDraft, ownedComposer, requestId) {
    const composer = findComposer();
    if (!composer) return emitResult('send_prompt', false, 'Google AI 输入框尚未就绪。', requestId);
    const prompt = cleanText(value);
    if (privateResearchTap && typeof privateResearchTap.observePrompt === 'function') {
      privateResearchTap.observePrompt(prompt);
    }
    if (typeof privateReplyReconciler.observePrompt === 'function') {
      const baseline = messageExtractor.extract(composer, isStreaming());
      privateReplyReconciler.observePrompt(baseline.messages, prompt, location.href);
    }
    if (privateReplyObserver && typeof privateReplyObserver.observePrompt === 'function') {
      privateReplyObserver.observePrompt(prompt);
    }
    const draft = cleanText(composerValue(composer));
    const reconciliation = sendPolicy.reconcile(
      draft,
      cleanText(expectedDraft),
      prompt,
      ownedComposer === true
    );
    if (!reconciliation.allowed) {
      scheduleSnapshot();
      return emitResult('send_prompt', false, '官方网页草稿已经变化，请确认后重试。', requestId);
    }
    if (reconciliation.write && !setComposerValue(composer, value)) {
      return emitResult('send_prompt', false, 'Google 官方输入框未接受文本。', requestId);
    }
    const beforeHref = location.href;
    const submitStartedAt = Date.now();
    let blockingMenuDismissals = 0;

    function confirmSubmission(startedAt) {
      const currentComposer = findComposer();
      const streaming = isStreaming();
      const queryMatches = messageExtractor.currentQueryMatches(prompt);
      if (sendPolicy.confirmed({
        hrefChanged: location.href !== beforeHref,
        streaming,
        queryMatches,
        currentDraft: cleanText(composerValue(currentComposer)),
        prompt
      })) {
        messageExtractor.rememberQuery(prompt);
        emitResult('send_prompt', true, '', requestId);
        return scheduleSnapshot();
      }
      if (Date.now() - startedAt >= 8000) {
        scheduleSnapshot();
        return emitResult('send_prompt', false, 'Google 官方页未确认发送，请重试。', requestId);
      }
      window.setTimeout(() => confirmSubmission(startedAt), 160);
    }

    function submitWhenReady() {
      const currentComposer = findComposer() || composer;
      const elapsedMs = Date.now() - submitStartedAt;
      if (blockingMenuDismissals < 3 &&
          typeof composerBridge.dismissBlockingMenu === 'function' &&
          composerBridge.dismissBlockingMenu()) {
        blockingMenuDismissals += 1;
        return window.setTimeout(submitWhenReady, 120);
      }
      const form = composerBridge.form(currentComposer);
      const scope = form || currentComposer.parentElement?.parentElement?.parentElement ||
        currentComposer.parentElement;
      const button = composerBridge.findSubmitAction(currentComposer) ||
        composerBridge.findAction(currentComposer, ['send', 'submit', '发送', '提交']) ||
        findButton(['send', 'submit', '发送', '提交'], scope || document);
      const navigationFallbackAllowed = elapsedMs >= SUBMIT_READY_TIMEOUT_MS &&
        prompt.length <= MAX_NAVIGATION_PROMPT_LENGTH && !messageExtractor.hasCurrentQuery();
      const step = sendPolicy.submissionStep({
        buttonReady: !!button,
        formReady: !!form && typeof form.requestSubmit === 'function',
        enterAvailable: !!currentComposer,
        navigationFallbackAllowed,
        elapsedMs,
        timeoutMs: SUBMIT_READY_TIMEOUT_MS
      });
      if (step === 'wait') return window.setTimeout(submitWhenReady, 80);
      let submitted = false;
      if (step === 'button') {
        button.click();
        submitted = true;
      } else if (step === 'form') {
        form.requestSubmit();
        submitted = true;
      } else if (step === 'enter') {
        submitted = composerBridge.pressEnter(currentComposer);
      } else if (step === 'navigate') {
        messageExtractor.rememberQuery(prompt);
        location.assign(firstPromptNavigationUrl(prompt));
        return;
      }
      if (!submitted) return emitResult('send_prompt', false, 'Google AI 发送入口尚未就绪。', requestId);
      confirmSubmission(Date.now());
    }

    submitWhenReady();
  }

  function runCommand(raw) {
    if (disposed) return;
    let command = {};
    try { command = JSON.parse(String(raw || '{}')); }
    catch (_) { return emitResult('unknown', false, '命令格式无效。'); }
    const action = String(command.action || '');
    const requestId = /^mcp_[a-z0-9]{1,32}$/.test(String(command.requestId || ''))
      ? String(command.requestId)
      : '';
    if (action === 'snapshot') return snapshot();
    if (action === 'send_prompt') return sendPrompt(
      String(command.value || '').slice(0, 20000),
      String(command.expectedDraft || '').slice(0, 20000),
      command.ownedComposer === true,
      requestId
    );
    if (action === 'stop_generation') {
      const stop = findButton(['stop response', 'stop generating', '停止回答', '停止生成', '停止']);
      if (!stop) return emitResult(action, false, '当前没有可停止的回答。', requestId);
      stop.click();
      emitResult(action, true, '', requestId);
      return scheduleSnapshot();
    }
    if (action === 'new_conversation') {
      messageExtractor.clearRememberedQuery();
      emitResult(action, true, '', requestId);
      location.assign('https://www.google.com/aimode');
      return;
    }
    emitResult(action || 'unknown', false, 'Google AI 不支持这个本地动作。', requestId);
  }

  function dispose() {
    if (disposed) return;
    disposed = true;
    clearTimeout(emitTimer);
    emitTimer = 0;
    if (observer) observer.disconnect();
    window.removeEventListener('popstate', scheduleSnapshot);
  }

  window.__elonGoogleWebBridge = Object.freeze({
    version: adapterVersion,
    documentToken,
    command: runCommand,
    dispose
  });
  emitEvent({
    type: 'adapter_ready',
    capabilities: ['streaming', 'citations', 'rich_text', 'new_conversation', 'conversation_history']
  });
  observer = new MutationObserver(scheduleSnapshot);
  const observeDocument = () => {
    const root = document.documentElement;
    if (!(root instanceof Node)) return false;
    observer.observe(root, {
      childList: true,
      subtree: true,
      characterData: true
    });
    return true;
  };
  if (!observeDocument()) {
    window.addEventListener('DOMContentLoaded', () => {
      if (disposed) return;
      observeDocument();
      scheduleSnapshot();
    }, { once: true });
  }
  window.addEventListener('popstate', scheduleSnapshot);
  if (privateReplyObserver && typeof privateReplyObserver.setListener === 'function') {
    privateReplyObserver.setListener(scheduleSnapshot);
  }
  if (privateThreadDirectory && typeof privateThreadDirectory.setListener === 'function') {
    privateThreadDirectory.setListener(emitPrivateDirectorySnapshot);
    emitPrivateDirectorySnapshot();
  }
  snapshot();
})();

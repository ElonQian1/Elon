(function () {
  'use strict';

  if (window.__elonChatGptBridge || location.origin !== 'https://chatgpt.com') return;
  const nativeBridge = window.elonChatGptNative;
  if (!nativeBridge || typeof nativeBridge.postMessage !== 'function') return;
  const conversationAdapter = window.__elonChatGptConversations;
  const messageAdapter = window.__elonChatGptMessages;
  const composerAdapter = window.__elonChatGptComposer;
  const navigationAdapter = window.__elonChatGptNavigation;
  const layoutAdapter = window.__elonChatGptLayout;
  const snapshotSchedulerModule = window.__elonChatGptSnapshotScheduler;
  const streamingPolicyModule = window.__elonChatGptStreamingPolicy;
  const skinAdapter = window.__elonChatGptSkin;
  const privateTransport = window.__elonChatGptPrivateTransport;
  const privateStreamTransport = window.__elonChatGptPrivateStreamTransport;
  const privateSendObserver = window.__elonChatGptPrivateSendObserver;
  const privateConversationDirectory = window.__elonChatGptPrivateConversationDirectory;
  const authenticationPolicy = window.__elonChatGptAuthenticationPolicy;
  const adapterVersion = Number(window.__elonChatGptAdapterVersion || 0);
  const documentToken = String(window.__elonChatGptDocumentToken || '');
  if (!/^doc_[a-z0-9_]{3,80}$/.test(documentToken)) return;

  let lastSnapshot = '';
  let sequence = 0;
  let disposed = false;
  let observer = null;
  let snapshotScheduler = null;
  let privateStreamUnsubscribe = null;
  let lastPrivateDirectorySnapshot = '';
  let streamingSnapshotMode = false;
  let privateStreamingSnapshotMode = false;
  let skinMode = false;
  const SEND_BUTTON_POLL_MS = 60;
  const SEND_BUTTON_SETTLE_MS = 180;
  const SEND_BUTTON_TIMEOUT_MS = 4000;
  const SEND_ACCEPT_TIMEOUT_MS = 3000;
  const streamingPolicy = streamingPolicyModule && streamingPolicyModule.create({
    now: Date.now,
    scheduleTimer: (delayMs, action) => window.setTimeout(action, delayMs),
    cancelTimer: (timer) => clearTimeout(timer)
  });
  function emitEvent(event) {
    if (disposed) return;
    nativeBridge.postMessage(JSON.stringify({
      schema: 'yilong.ai.ui.v1',
      adapterVersion,
      documentToken,
      providerId: 'chatgpt',
      source: 'official_web',
      conversationId: location.pathname,
      sequence: ++sequence,
      emittedAt: new Date().toISOString(),
      event
    }));
  }

  function emitPrivateDirectorySnapshot() {
    if (!privateConversationDirectory ||
        typeof privateConversationDirectory.snapshot !== 'function') return;
    const value = optional(null, () => privateConversationDirectory.snapshot());
    if (!value || !Array.isArray(value.conversations) || !Array.isArray(value.projects) ||
        (!value.conversations.length && !value.projects.length)) return;
    const fingerprint = JSON.stringify(value);
    if (fingerprint === lastPrivateDirectorySnapshot) return;
    lastPrivateDirectorySnapshot = fingerprint;
    emitEvent({
      type: 'conversation_snapshot',
      conversations: value.conversations,
      projects: value.projects,
      scopeProjectId: null,
      collection: {
        scrollerFound: false,
        scrolled: false,
        scrollRestored: true,
        reachedEnd: false,
        truncated: value.conversations.length >= 200,
        timedOut: false,
        observedCount: value.conversations.length,
        steps: 0,
        complete: false,
        source: 'official_private',
        officialLoadState: 'ready'
      }
    });
  }

  function cleanText(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\n{3,}/g, '\n\n').trim();
  }

  function comparableText(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\r\n/g, '\n').trim();
  }

  function optional(fallback, read) {
    try { return read(); }
    catch { return fallback; }
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

  function hasLoginEntry() {
    return Array.from(document.querySelectorAll('a, button, [role="button"]')).some((node) => {
      if (node.isConnected === false) return false;
      const label = cleanText(node.getAttribute('aria-label') || node.textContent).toLowerCase();
      const href = String(node.getAttribute('href') || '').toLowerCase();
      return authenticationPolicy && authenticationPolicy.isLoginEntry({ label, href });
    });
  }

  function hasProfileEntry() {
    const profile = document.querySelector(
      '[data-testid="profile-button"], [data-testid="accounts-profile-button"], ' +
      'button[aria-label*="profile" i], button[aria-label*="account" i], ' +
      'button[aria-label*="个人资料" i], button[aria-label*="账号" i]'
    );
    return !!profile && profile.isConnected !== false;
  }

  function visibleAccessText() {
    const scope = document.querySelector('main') || document.body;
    if (!scope) return '';
    return Array.from(scope.querySelectorAll('h1, h2, h3, p, [role="alert"], [role="dialog"], button, a'))
      .filter(isVisible)
      .slice(0, 80)
      .map((element) => cleanText(element.textContent).slice(0, 240))
      .filter(Boolean)
      .join(' ')
      .slice(0, 4000);
  }

  function accessDecision(pageKind, composer, privateAccess) {
    const signals = {
      pageKind,
      composerReady: !!composer,
      hasLoginEntry: hasLoginEntry(),
      visibleText: visibleAccessText(),
      privateStatus: privateAccess && privateAccess.status
    };
    if (authenticationPolicy && typeof authenticationPolicy.accessDecision === 'function') {
      return authenticationPolicy.accessDecision(signals);
    }
    const loginRequired = pageKind === 'auth';
    return { blocked: loginRequired, loginRequired, reason: loginRequired ? 'login_required' : '', source: loginRequired ? 'visible_page' : '' };
  }

  function isAuthenticated(loginRequired, composerReady) {
    const signals = {
      loginRequired,
      hasLoginEntry: hasLoginEntry(),
      hasProfileEntry: hasProfileEntry(),
      composerReady
    };
    if (authenticationPolicy && typeof authenticationPolicy.isAuthenticated === 'function') {
      return authenticationPolicy.isAuthenticated(signals);
    }
    return !signals.loginRequired && !signals.hasLoginEntry &&
      (signals.hasProfileEntry || signals.composerReady);
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

  function readStreamingState() {
    return streamingPolicyModule
      ? streamingPolicyModule.readState(streamingPolicy, messageAdapter, document, findComposer(), isVisible)
      : { active: false, assistantKey: '' };
  }
  function detectCapabilities(composer) {
    const capabilities = [
      'streaming',
      'conversation_history',
      'draft_sync',
      'new_conversation',
      'google_login_entry'
    ];
    if (conversationAdapter) capabilities.push(...conversationAdapter.capabilities());
    if (messageAdapter) capabilities.push(...messageAdapter.capabilities());
    if (composerAdapter) capabilities.push(...composerAdapter.capabilities(composer));
    if (navigationAdapter) capabilities.push(...navigationAdapter.capabilities());
    return Array.from(new Set(capabilities));
  }

  function snapshot() {
    if (disposed) return;
    const composer = findComposer();
    const pageKind = optional('unknown', () => layoutAdapter && typeof layoutAdapter.pageKind === 'function'
      ? layoutAdapter.pageKind()
      : 'unknown');
    const privateAccess = optional(null, () => privateStreamTransport &&
      typeof privateStreamTransport.access === 'function'
      ? privateStreamTransport.access()
      : null);
    const access = optional({ blocked: false, loginRequired: false, reason: '', source: '' }, () =>
      accessDecision(pageKind, composer, privateAccess));
    const loginRequired = access.loginRequired === true;
    const dictationActive = optional(false, () => composerAdapter ? composerAdapter.dictationActive(composer) : false);
    const streamingState = optional({ active: false, assistantKey: '' }, readStreamingState);
    const privateStream = optional(null, () => privateStreamTransport &&
      typeof privateStreamTransport.current === 'function'
      ? privateStreamTransport.current(location.pathname)
      : null);
    const streaming = access.blocked !== true &&
      (streamingState.active || !!(privateStream && privateStream.state === 'streaming'));
    if (access.blocked === true && streamingPolicy) streamingPolicy.reset();
    streamingSnapshotMode = streaming;
    privateStreamingSnapshotMode = access.blocked !== true &&
      !!(privateStream && privateStream.state === 'streaming');
    const messageWindow = optional({ messages: [], observedCount: 0, startIndex: 0 }, () =>
      messageAdapter && typeof messageAdapter.readMessageWindow === 'function'
        ? messageAdapter.readMessageWindow(streaming, streamingState.assistantKey)
        : { messages: messageAdapter ? messageAdapter.readMessages(streaming) : [], observedCount: 0, startIndex: 0 }
    );
    const domMessages = Array.isArray(messageWindow.messages) ? messageWindow.messages : [];
    const messages = optional(domMessages, () => privateStreamTransport &&
      typeof privateStreamTransport.mergeMessages === 'function'
      ? privateStreamTransport.mergeMessages(domMessages, location.pathname)
      : domMessages);
    const event = {
      type: 'message_snapshot',
      title: cleanText(document.title.replace(/\s*[-|]\s*ChatGPT.*$/i, '')),
      url: location.origin + location.pathname,
      draft: composerValue(composer).slice(0, 20000),
      messages,
      observedMessageCount: Math.max(messages.length, Number(messageWindow.observedCount) || 0),
      messageWindowStart: Math.max(0, Number(messageWindow.startIndex) || 0),
      authenticated: isAuthenticated(loginRequired, !!composer),
      pageKind,
      loginRequired,
      accessReason: access.reason || '',
      accessSource: access.source || '',
      composerReady: !!composer,
      streaming,
      streamingStatus: cleanText(privateStream && privateStream.progressLabel).slice(0, 220),
      currentModel: optional('', () => composerAdapter ? composerAdapter.currentModel(composer) : ''),
      attachments: optional([], () => composerAdapter ? composerAdapter.readAttachments(composer) : []),
      dictationActive,
      capabilities: optional([], () => detectCapabilities(composer))
    };
    const fingerprint = JSON.stringify(event);
    if (fingerprint !== lastSnapshot) {
      lastSnapshot = fingerprint;
      emitEvent(event);
    }
    optional(undefined, () => layoutAdapter && layoutAdapter.emitSnapshot(emitEvent));
    if (streamingPolicy) streamingPolicy.scheduleNext(
      streaming,
      () => scheduleSnapshot(true),
      { privateStreamObserved: privateStreamingSnapshotMode }
    );
  }

  function scheduleSnapshot(recordsOrActive) {
    if (disposed || skinMode || !snapshotScheduler) return;
    const forced = recordsOrActive === true;
    if (!forced && privateStreamingSnapshotMode) return;
    const active = forced || streamingSnapshotMode;
    snapshotScheduler.schedule(active);
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

  function result(action, ok, detail, requestId) {
    if (disposed) return;
    const event = {
      type: 'command_result',
      adapterVersion,
      documentToken,
      action,
      ok,
      detail: detail || ''
    };
    if (requestId) event.requestId = requestId;
    nativeBridge.postMessage(JSON.stringify(event));
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

  function waitForStableSendButton(composer, expectedValue, onReady, onTimeout) {
    const started = Date.now();
    let readySince = 0;
    let readyButton = null;
    function poll() {
      const button = findSendButton(composer);
      const draftMatches = comparableText(composerValue(composer)) === comparableText(expectedValue);
      if (button && draftMatches) {
        if (button !== readyButton) {
          readyButton = button;
          readySince = Date.now();
        }
        if (Date.now() - readySince >= SEND_BUTTON_SETTLE_MS) return onReady(button);
      } else {
        readyButton = null;
        readySince = 0;
      }
      if (Date.now() - started >= SEND_BUTTON_TIMEOUT_MS) return onTimeout();
      window.setTimeout(poll, SEND_BUTTON_POLL_MS);
    }
    poll();
  }

  function waitForSendAccepted(composer, expectedValue, sendMarker, onAccepted, onTimeout) {
    const started = Date.now();
    function poll() {
      if (privateSendObserver && typeof privateSendObserver.dispatchedAfter === 'function' &&
          privateSendObserver.dispatchedAfter(sendMarker)) {
        return onAccepted('official_request_dispatched');
      }
      const currentValue = comparableText(composerValue(composer));
      if (!currentValue || currentValue !== comparableText(expectedValue) || readStreamingState().active) {
        return onAccepted('official_page_accepted');
      }
      if (Date.now() - started >= SEND_ACCEPT_TIMEOUT_MS) return onTimeout();
      window.setTimeout(poll, SEND_BUTTON_POLL_MS);
    }
    poll();
  }

  function sendPrompt(value, expectedDraft, respond) {
    const composer = findComposer();
    const assistantBeforeSend = streamingPolicyModule && streamingPolicyModule.messageObservation(messageAdapter);
    if (!composer) return respond('send_prompt', false, '未找到输入框，请切换网页模式。');
    if (comparableText(composerValue(composer)) !== comparableText(expectedDraft)) {
      return respond('send_prompt', false, '网页草稿已变化，请返回官网确认后重试。');
    }
    if (!setComposerValue(composer, value)) {
      return respond('send_prompt', false, '官方输入框未接受文本，请返回官网重试。');
    }
    waitForStableSendButton(
      composer,
      value,
      (button) => {
        const sendMarker = privateSendObserver && typeof privateSendObserver.marker === 'function'
          ? privateSendObserver.marker()
          : null;
        button.click();
        scheduleSnapshot(true);
        waitForSendAccepted(
          composer,
          value,
          sendMarker,
          (acceptance) => {
            if (streamingPolicy) streamingPolicy.begin(assistantBeforeSend);
            respond(
              'send_prompt',
              true,
              acceptance === 'official_request_dispatched'
                ? '官网发送请求已提交。'
                : '官方网页已确认发送。'
            );
            scheduleSnapshot();
          },
          () => respond('send_prompt', false, '官方网页未确认发送，请重试。')
        );
      },
      () => respond('send_prompt', false, '发送按钮尚未就绪，请返回官网重试。')
    );
  }

  function setDraft(value, expectedDraft, respond) {
    const composer = findComposer();
    if (!composer) return respond('set_draft', false, '未找到输入框，请切换网页模式。');
    if (comparableText(composerValue(composer)) !== comparableText(expectedDraft)) {
      return respond('set_draft', false, '网页草稿已变化，请返回官网确认后重试。');
    }
    if (!setComposerValue(composer, value)) {
      return respond('set_draft', false, '官方输入框未接受文本，请返回官网重试。');
    }
    respond('set_draft', true, '');
    scheduleSnapshot();
  }

  function startGoogleLogin(respond) {
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
    if (!target) return respond('start_google_login', false, '官方 Google 登录入口尚未就绪。');
    target.click();
    respond('start_google_login', true, '');
  }

  function runCommand(raw) {
    let command = {};
    try { command = JSON.parse(String(raw || '{}')); }
    catch { return result('unknown', false, '命令格式无效。'); }
    const action = String(command.action || '');
    const rawRequestId = String(command.requestId || '');
    const requestId = /^mcp_[a-z0-9]{1,32}$/.test(rawRequestId) ? rawRequestId : '';
    if (String(command.documentToken || '') !== documentToken) {
      return result(action || 'unknown', false, '页面已更新，请重新执行。', requestId);
    }
    const respond = (resultAction, ok, detail) => result(resultAction, ok, detail, requestId);
    if (action === 'snapshot') return snapshot();
    if (action === 'set_skin_mode') {
      if (!skinAdapter || typeof skinAdapter.setEnabled !== 'function') {
        return respond(action, false, '网页皮肤模块尚未就绪。');
      }
      const enabled = command.selected === true;
      const applied = skinAdapter.setEnabled(enabled);
      if (!applied || applied.ok !== true) {
        return respond(action, false, '当前官方页面无法启用网页皮肤。');
      }
      skinMode = enabled;
      if (skinMode) {
        if (streamingPolicy) streamingPolicy.scheduleNext(false);
        if (observer) observer.disconnect();
        if (snapshotScheduler && typeof snapshotScheduler.cancelPending === 'function') {
          snapshotScheduler.cancelPending();
        }
      } else {
        observeDocument();
        scheduleSnapshot();
      }
      return respond(action, true, '');
    }
    if (action === 'snapshot_ui_manifest' && layoutAdapter) {
      layoutAdapter.emitSnapshot(emitEvent, true);
      return respond(action, true, '');
    }
    if (action === 'invoke_ui_control' && layoutAdapter) {
      return layoutAdapter.invoke(String(command.value || ''), emitEvent, respond);
    }
    if (action === 'send_prompt') {
      return sendPrompt(
        String(command.value || '').slice(0, 20000),
        String(command.expectedDraft || '').slice(0, 20000),
        respond
      );
    }
    if (action === 'set_draft') {
      return setDraft(
        String(command.value || '').slice(0, 20000),
        String(command.expectedDraft || '').slice(0, 20000),
        respond
      );
    }
    if (action === 'start_google_login') return startGoogleLogin(respond);
    if (action === 'list_model_options' && composerAdapter) {
      return composerAdapter.requestOptions('model', findComposer(), emitEvent, respond);
    }
    if (action === 'list_composer_tools' && composerAdapter) {
      return composerAdapter.requestOptions('tools', findComposer(), emitEvent, respond);
    }
    if (action === 'collect_model_options' && composerAdapter) {
      return composerAdapter.collectRequestedOptions('model', findComposer(), emitEvent, respond);
    }
    if (action === 'collect_composer_tools' && composerAdapter) {
      return composerAdapter.collectRequestedOptions('tools', findComposer(), emitEvent, respond);
    }
    if (action === 'select_model_option' && composerAdapter) {
      return composerAdapter.selectOption(
        'model', String(command.value || ''), findComposer(), emitEvent, respond, scheduleSnapshot
      );
    }
    if (action === 'select_composer_tool' && composerAdapter) {
      return composerAdapter.selectOption(
        'tools', String(command.value || ''), findComposer(), emitEvent, respond, scheduleSnapshot
      );
    }
    if (action === 'request_attachment_upload' && composerAdapter) {
      return composerAdapter.requestAttachmentUpload(respond);
    }
    if (action === 'open_model_selector' && composerAdapter) {
      return composerAdapter.openOfficial('model', findComposer(), emitEvent, respond);
    }
    if (action === 'open_composer_tools' && composerAdapter) {
      return composerAdapter.openOfficial('tools', findComposer(), emitEvent, respond);
    }
    if (action === 'start_dictation' && composerAdapter) {
      return composerAdapter.startDictation(findComposer(), emitEvent, respond);
    }
    if (action === 'cancel_dictation' && composerAdapter) {
      return composerAdapter.cancelDictation(emitEvent, respond);
    }
    if (action === 'submit_dictation' && composerAdapter) {
      return composerAdapter.submitDictation(emitEvent, respond);
    }
    if (action === 'remove_attachment' && composerAdapter) {
      return composerAdapter.removeAttachment(String(command.value || ''), emitEvent, respond);
    }
    if (action === 'dismiss_composer_menu' && composerAdapter) {
      return composerAdapter.dismissOpenMenu(respond);
    }
    if (action === 'list_navigation' && navigationAdapter) {
      return navigationAdapter.requestList(emitEvent, respond);
    }
    if (action === 'collect_navigation' && navigationAdapter) {
      return navigationAdapter.collectList(emitEvent, respond);
    }
    if (action === 'select_navigation' && navigationAdapter) {
      return navigationAdapter.selectFeature(String(command.value || ''), emitEvent, respond);
    }
    if (action === 'dismiss_navigation' && navigationAdapter) {
      return navigationAdapter.dismiss(emitEvent, respond);
    }
    if (action === 'set_ui_control_text' && layoutAdapter) {
      return layoutAdapter.setText(
        String(command.controlId || ''), String(command.value || ''), emitEvent, respond
      );
    }
    if (action === 'set_ui_control_selected' && layoutAdapter) {
      return layoutAdapter.setSelected(
        String(command.controlId || ''), command.selected === true, emitEvent, respond
      );
    }
    if (action === 'select_ui_control_choice' && layoutAdapter) {
      return layoutAdapter.selectChoice(
        String(command.controlId || ''), Number(command.choiceIndex), emitEvent, respond
      );
    }
    if (action === 'set_ui_control_slider' && layoutAdapter) {
      return layoutAdapter.setSliderValue(
        String(command.controlId || ''), Number(command.numericValue), emitEvent, respond
      );
    }
    if (action === 'set_ui_control_expanded' && layoutAdapter) {
      return layoutAdapter.setExpanded(
        String(command.controlId || ''), command.expanded === true, emitEvent, respond
      );
    }
    if (action === 'list_conversations' && conversationAdapter) {
      return conversationAdapter.requestList(command, emitEvent, respond);
    }
    if (action === 'refresh_current_conversation') {
      if (privateTransport && privateTransport.conversationPrefetchEnabled === true &&
          typeof privateTransport.refreshCurrentConversation === 'function') {
        privateTransport.refreshCurrentConversation(location.pathname, emitEvent);
      }
      return;
    }
    if (action === 'open_conversation' && conversationAdapter) {
      if (comparableText(composerValue(findComposer()))) {
        return respond(action, false, '网页中有未发送草稿，请先处理草稿。');
      }
      const path = String(command.value || '');
      const navigate = () => conversationAdapter.openConversation(path, respond);
      if (privateTransport && privateTransport.conversationPrefetchEnabled === true &&
          typeof privateTransport.prefetchConversation === 'function') {
        const handled = privateTransport.prefetchConversation(path, emitEvent, navigate);
        if (handled) return;
      }
      return navigate();
    }
    if (action === 'open_project' && conversationAdapter) {
      if (comparableText(composerValue(findComposer()))) {
        return respond(action, false, '网页中有未发送草稿，请先处理草稿。');
      }
      return conversationAdapter.openProject(String(command.value || ''), respond);
    }
    if (action === 'regenerate_response' && messageAdapter) {
      if (streamingPolicyModule) streamingPolicyModule.begin(
        streamingPolicy, messageAdapter, { allowSameTurn: true }
      );
      return messageAdapter.regenerate(emitEvent, respond);
    }
    if (action === 'stop_generation') {
      const composer = findComposer();
      const scope = composer && composer.closest('form');
      const stop = document.querySelector('[data-testid="stop-button"]') ||
        (scope && Array.from(scope.querySelectorAll('button')).find((button) => {
          const label = cleanText([
            button.getAttribute('aria-label'),
            button.getAttribute('title'),
            button.textContent
          ].filter(Boolean).join(' ')).toLowerCase();
          return isVisible(button) && /stop (?:generating|streaming|response)|停止(?:生成|產生|回答|回覆)/.test(label);
        }));
      if (!stop) return respond(action, false, '当前没有正在生成的回复。');
      stop.click();
      respond(action, true, '');
      return scheduleSnapshot();
    }
    if (action === 'new_conversation') {
      if (comparableText(composerValue(findComposer()))) {
        return respond(action, false, '网页中有未发送草稿，请先处理草稿。');
      }
      if (!conversationAdapter) return respond(action, false, '会话适配器尚未就绪。');
      if (streamingPolicy) streamingPolicy.reset();
      if (privateStreamTransport && typeof privateStreamTransport.reset === 'function') {
        privateStreamTransport.reset();
      }
      const inspect = () => {
        const composer = findComposer();
        const messages = messageAdapter && typeof messageAdapter.readMessages === 'function'
          ? messageAdapter.readMessages(false)
          : [];
        return {
          messageCount: Array.isArray(messages) ? messages.length : 0,
          composerReady: !!composer
        };
      };
      return conversationAdapter.newConversation(inspect, (resultAction, ok, detail) => {
        respond(resultAction, ok, detail);
        if (ok) scheduleSnapshot(true);
      });
    }
    respond(action || 'unknown', false, '不支持的本地命令。');
  }

  function dispose() {
    if (disposed) return;
    if (skinAdapter && typeof skinAdapter.setEnabled === 'function') {
      skinAdapter.setEnabled(false);
    }
    disposed = true;
    if (streamingPolicy) streamingPolicy.dispose();
    if (snapshotScheduler) snapshotScheduler.dispose();
    if (typeof privateStreamUnsubscribe === 'function') privateStreamUnsubscribe();
    privateStreamUnsubscribe = null;
    if (privateConversationDirectory &&
        typeof privateConversationDirectory.setListener === 'function') {
      privateConversationDirectory.setListener(null);
    }
    if (observer) observer.disconnect();
    window.removeEventListener('popstate', scheduleSnapshot);
  }

  window.__elonChatGptBridge = Object.freeze({
    version: adapterVersion,
    command: runCommand,
    dispose
  });
  emitEvent({
    type: 'adapter_ready',
    capabilities: optional([], () => detectCapabilities(findComposer()))
  });
  snapshotScheduler = optional(null, () => snapshotSchedulerModule && snapshotSchedulerModule.create({
    scheduleTimer: (delayMs, action) => window.setTimeout(action, delayMs),
    cancelTimer: (timer) => clearTimeout(timer),
    snapshot,
    quietDelayMs: 240,
    maxDelayMs: 5000,
    activeQuietDelayMs: 80,
    activeMaxDelayMs: 700
  }));
  privateStreamUnsubscribe = optional(null, () => privateStreamTransport &&
    typeof privateStreamTransport.subscribe === 'function'
    ? privateStreamTransport.subscribe(() => scheduleSnapshot(true))
    : null);
  if (privateConversationDirectory &&
      typeof privateConversationDirectory.setListener === 'function') {
    privateConversationDirectory.setListener(emitPrivateDirectorySnapshot);
    emitPrivateDirectorySnapshot();
  }
  observer = new MutationObserver(scheduleSnapshot);
  const observeDocument = () => {
    const root = document.documentElement;
    if (!(root instanceof Node)) return false;
    observer.observe(root, {
      attributes: true,
      attributeFilter: ['aria-selected', 'aria-checked', 'aria-disabled', 'disabled', 'data-state', 'hidden'],
      childList: true,
      subtree: true,
      characterData: true
    });
    return true;
  };
  if (!observeDocument()) {
    window.addEventListener('DOMContentLoaded', () => {
      observeDocument();
      scheduleSnapshot();
    }, { once: true });
  }
  window.addEventListener('popstate', scheduleSnapshot);
  snapshot();
})();

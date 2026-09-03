(function () {
  'use strict';

  if (window.__elonChatGptBridge || location.origin !== 'https://chatgpt.com') return;
  const nativeBridge = window.elonChatGptNative;
  if (!nativeBridge || typeof nativeBridge.postMessage !== 'function') return;
  const conversationAdapter = window.__elonChatGptConversations;
  const messageAdapter = window.__elonChatGptMessages;
  const imageAssets = window.__elonChatGptImageAssets;
  const composerAdapter = window.__elonChatGptComposer;
  const navigationAdapter = window.__elonChatGptNavigation;
  const layoutAdapter = window.__elonChatGptLayout;
  const snapshotSchedulerModule = window.__elonChatGptSnapshotScheduler;
  const streamingPolicyModule = window.__elonChatGptStreamingPolicy;
  const streamWatchdogAcceptanceModule = window.__elonChatGptStreamWatchdogAcceptance;
  const skinAdapter = window.__elonChatGptSkin;
  const privateTransport = window.__elonChatGptPrivateTransport;
  const privateDictationTransport = window.__elonChatGptPrivateDictationTransport;
  const privateDictationOrchestratorModule = window.__elonChatGptPrivateDictationOrchestrator;
  const textTransactionOrchestratorModule = window.__elonChatGptTextTransactionOrchestrator;
  const privateStreamTransport = window.__elonChatGptPrivateStreamTransport;
  const attachmentTransportObserver = window.__elonChatGptAttachmentTransportObserver;
  const privateConversationDirectory = window.__elonChatGptPrivateConversationDirectory;
  const conversationDirectoryRequestsModule =
    window.__elonChatGptConversationDirectoryRequests;
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
  let privateStreamRevision = 0;
  let streamingSnapshotMode = false;
  let privateStreamingSnapshotMode = false;
  let skinMode = false;
  const streamingPolicy = streamingPolicyModule && streamingPolicyModule.create({
    now: Date.now,
    scheduleTimer: (delayMs, action) => window.setTimeout(action, delayMs),
    cancelTimer: (timer) => clearTimeout(timer)
  });
  const textTransactionOrchestrator = textTransactionOrchestratorModule &&
    typeof textTransactionOrchestratorModule.create === 'function'
    ? textTransactionOrchestratorModule.create({
      findComposer,
      composerValue,
      setComposerValue,
      comparableText,
      findButton,
      isVisible,
      readStreamingState,
      streamingPolicy,
      streamingPolicyModule,
      messageAdapter,
      scheduleSnapshot
    })
    : null;
  const streamWatchdogAcceptance = streamWatchdogAcceptanceModule &&
    streamWatchdogAcceptanceModule.create({
    probeModule: window.__elonChatGptStreamWatchdogProbe,
    streamingPolicyModule,
    now: Date.now,
    scheduleTimer: (delayMs, action) => window.setTimeout(action, delayMs),
    cancelTimer: (timer) => clearTimeout(timer),
    onResult: (requestId, ok, detail) => result('verify_private_stream_watchdog', ok, detail, requestId),
    scheduleSnapshot: () => scheduleSnapshot(true)
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

  const conversationDirectoryRequests = conversationDirectoryRequestsModule &&
    typeof conversationDirectoryRequestsModule.create === 'function'
    ? conversationDirectoryRequestsModule.create({
      conversationAdapter,
      privateDirectory: privateConversationDirectory,
      privateTransport,
      emitEvent,
      optional
    })
    : null;
  const privateDictationOrchestrator = privateDictationOrchestratorModule &&
    typeof privateDictationOrchestratorModule.create === 'function'
    ? privateDictationOrchestratorModule.create({
      transport: privateDictationTransport,
      findComposer,
      composerValue,
      setComposerValue: setComposerValueWithoutFocus,
      comparableText,
      scheduleSnapshot
    })
    : null;

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

  function readStreamingState(privateStreamState) {
    return streamingPolicyModule
      ? streamingPolicyModule.readState(
        streamingPolicy,
        messageAdapter,
        document,
        findComposer(),
        isVisible,
        { privateStreamState: String(privateStreamState || 'idle') }
      )
      : { active: false, assistantKey: '' };
  }

  function invalidatePrivateTextContext() {
    const relay = window.__elonChatGptPrivateTextTransactionRelay;
    if (relay && typeof relay.invalidateContext === 'function') relay.invalidateContext();
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
    const dictationCaptureActive = optional(false, () => composerAdapter &&
      typeof composerAdapter.dictationCaptureActive === 'function'
      ? composerAdapter.dictationCaptureActive()
      : false);
    const dictationCapturePending = optional(false, () => composerAdapter &&
      typeof composerAdapter.dictationCapturePending === 'function'
      ? composerAdapter.dictationCapturePending()
      : false);
    const privateStream = optional(null, () => privateStreamTransport &&
      typeof privateStreamTransport.current === 'function'
      ? privateStreamTransport.current(location.pathname)
      : null);
    const privateStreamState = ['streaming', 'completed'].includes(
      String(privateStream && privateStream.state || '')
    ) ? String(privateStream.state) : 'idle';
    const streamingState = optional(
      { active: false, assistantKey: '' },
      () => readStreamingState(privateStreamState)
    );
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
      privateStreamObserved: privateStreamRevision > 0,
      privateStreamRevision,
      privateStreamState,
      currentModel: optional('', () => composerAdapter ? composerAdapter.currentModel(composer) : ''),
      attachments: optional([], () => composerAdapter ? composerAdapter.readAttachments(composer) : []),
      dictationActive,
      dictationCaptureActive,
      dictationCapturePending,
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
      (schedule) => {
        scheduleSnapshot(true);
      },
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

  function updateComposerValue(composer, value, focusComposer) {
    if (focusComposer) composer.focus();
    if (composer instanceof HTMLTextAreaElement || composer instanceof HTMLInputElement) {
      const prototype = composer instanceof HTMLTextAreaElement
        ? HTMLTextAreaElement.prototype
        : HTMLInputElement.prototype;
      const setter = Object.getOwnPropertyDescriptor(prototype, 'value').set;
      setter.call(composer, value);
    } else {
      if (focusComposer) {
        const selection = window.getSelection();
        const range = document.createRange();
        range.selectNodeContents(composer);
        selection.removeAllRanges();
        selection.addRange(range);
        const inserted = document.execCommand('insertText', false, value);
        if (!inserted) composer.replaceChildren(document.createTextNode(value));
      } else {
        composer.replaceChildren(document.createTextNode(value));
      }
    }
    composer.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText', data: value }));
    composer.dispatchEvent(new Event('change', { bubbles: true }));
    return comparableText(composerValue(composer)) === comparableText(value);
  }

  function setComposerValue(composer, value) {
    return updateComposerValue(composer, value, true);
  }

  function setComposerValueWithoutFocus(composer, value) {
    return updateComposerValue(composer, value, false);
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
    respond.requestId = requestId;
    if (action === 'snapshot') return snapshot();
    if (action === 'request_image_asset') {
      if (!imageAssets || typeof imageAssets.request !== 'function') {
        return respond(action, false, 'image_asset_unavailable');
      }
      return imageAssets.request(String(command.value || ''), emitEvent).then((outcome) => {
        respond(action, outcome && outcome.ok === true, outcome && outcome.error || '');
      });
    }
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
    if (action === 'invoke_ui_control_after_touch_miss' && layoutAdapter) {
      return layoutAdapter.invokeAfterTouchMiss(String(command.value || ''), emitEvent, respond);
    }
    if (action === 'reveal_project_choice' && layoutAdapter) {
      return layoutAdapter.revealProjectChoice(String(command.value || ''), emitEvent, respond);
    }
    if (action === 'send_prompt') {
      if (!textTransactionOrchestrator) {
        return respond(action, false, '发送事务模块尚未就绪，请重试。');
      }
      return textTransactionOrchestrator.sendPrompt(
        String(command.value || '').slice(0, 20000),
        String(command.expectedDraft || '').slice(0, 20000),
        respond,
        command.allowPrivateTextTransaction === true
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
      invalidatePrivateTextContext();
      return composerAdapter.selectOption(
        'model', String(command.value || ''), findComposer(), emitEvent, respond, scheduleSnapshot
      );
    }
    if (action === 'select_composer_tool' && composerAdapter) {
      invalidatePrivateTextContext();
      return composerAdapter.selectOption(
        'tools', String(command.value || ''), findComposer(), emitEvent, respond, scheduleSnapshot
      );
    }
    if (action === 'request_attachment_upload' && composerAdapter) {
      invalidatePrivateTextContext();
      if (attachmentTransportObserver && typeof attachmentTransportObserver.arm === 'function') {
        attachmentTransportObserver.arm();
      }
      return composerAdapter.requestAttachmentUpload(respond);
    }
    if (action === 'open_model_selector' && composerAdapter) {
      return composerAdapter.openOfficial('model', findComposer(), emitEvent, respond);
    }
    if (action === 'open_composer_tools' && composerAdapter) {
      return composerAdapter.openOfficial('tools', findComposer(), emitEvent, respond);
    }
    if (action === 'private_start_dictation' && privateDictationOrchestrator) {
      return privateDictationOrchestrator.start(
        String(command.value || '').slice(0, 20000),
        String(command.expectedDraft || '').slice(0, 20000),
        respond
      );
    }
    if (action === 'private_cancel_dictation' && privateDictationOrchestrator) {
      return privateDictationOrchestrator.cancel(respond);
    }
    if (action === 'private_submit_dictation' && privateDictationOrchestrator) {
      return privateDictationOrchestrator.submit(respond);
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
      invalidatePrivateTextContext();
      return layoutAdapter.setSelected(
        String(command.controlId || ''), command.selected === true, emitEvent, respond
      );
    }
    if (action === 'select_ui_control_choice' && layoutAdapter) {
      invalidatePrivateTextContext();
      return layoutAdapter.selectChoice(
        String(command.controlId || ''), Number(command.choiceIndex), emitEvent, respond
      );
    }
    if (action === 'set_ui_control_slider' && layoutAdapter) {
      invalidatePrivateTextContext();
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
      if (conversationDirectoryRequests) {
        return conversationDirectoryRequests.requestList(command, respond);
      }
      return conversationAdapter.requestList(command, emitEvent, respond);
    }
    if (action === 'cancel_conversation_directory') {
      if (conversationDirectoryRequests) conversationDirectoryRequests.cancel();
      return respond(action, true, '');
    }
    if (action === 'probe_conversation_project') {
      if (conversationDirectoryRequests &&
          conversationDirectoryRequests.probeMembership(command, respond)) return;
      return respond(action, false, 'membership_probe_unavailable');
    }
    if (action === 'refresh_current_conversation') {
      if (privateTransport && privateTransport.conversationPrefetchEnabled === true &&
          typeof privateTransport.refreshCurrentConversation === 'function') {
        privateTransport.refreshCurrentConversation(location.pathname, emitEvent);
      }
      return;
    }
    if (action === 'verify_private_stream_watchdog') {
      const armed = streamWatchdogAcceptance && streamWatchdogAcceptance.run(
        String(command.requestId || ''), streamingSnapshotMode
      );
      if (!armed || armed.accepted !== true) respond(action, false, armed && armed.detail || 'probe_unavailable');
      return;
    }
    if (action === 'open_conversation' && conversationAdapter) {
      if (comparableText(composerValue(findComposer()))) {
        return respond(action, false, '网页中有未发送草稿，请先处理草稿。');
      }
      if (conversationDirectoryRequests) conversationDirectoryRequests.cancel();
      invalidatePrivateTextContext();
      const path = String(command.value || '');
      const navigate = () => conversationAdapter.openConversation(path, respond);
      if (privateTransport && privateTransport.conversationPrefetchEnabled === true &&
          typeof privateTransport.prefetchConversation === 'function') {
        privateTransport.prefetchConversation(path, emitEvent, null);
      }
      return navigate();
    }
    if (action === 'open_project' && conversationAdapter) {
      if (comparableText(composerValue(findComposer()))) {
        return respond(action, false, '网页中有未发送草稿，请先处理草稿。');
      }
      invalidatePrivateTextContext();
      return conversationAdapter.openProject(String(command.value || ''), respond);
    }
    if (action === 'regenerate_response' && messageAdapter) {
      if (privateStreamTransport && typeof privateStreamTransport.prepareSend === 'function') {
        privateStreamTransport.prepareSend();
      }
      if (streamingPolicyModule) streamingPolicyModule.begin(
        streamingPolicy, messageAdapter, { allowSameTurn: true }
      );
      if (textTransactionOrchestrator &&
          textTransactionOrchestrator.tryPrivateRegeneration(respond)) return;
      return messageAdapter.regenerate(emitEvent, respond);
    }
    if (action === 'stop_generation') {
      if (textTransactionOrchestrator && textTransactionOrchestrator.stopPrivate(respond)) return;
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
      invalidatePrivateTextContext();
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
    if (streamWatchdogAcceptance) streamWatchdogAcceptance.dispose();
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
    ? privateStreamTransport.subscribe(() => {
      privateStreamRevision += 1;
      scheduleSnapshot(true);
    })
    : null);
  if (conversationDirectoryRequests) conversationDirectoryRequests.installListener();
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

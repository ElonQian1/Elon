(function () {
  'use strict';

  if (window.__elonChatGptPrivateTextTransactionsEnabled !== true) return;
  if (location.origin !== 'https://chatgpt.com') return;
  const existing = window.__elonChatGptPrivateTextTransactionRelay;
  if (existing && Number(existing.version) >= 16) return;
  if (existing && typeof existing.dispose === 'function') existing.dispose();
  const policy = window.__elonChatGptPrivateTextTransactionPolicy;
  const delegateFetch = typeof window.fetch === 'function' ? window.fetch : null;
  if (!policy || !delegateFetch || typeof Request !== 'function' ||
      typeof AbortController !== 'function') return;

  const MAX_BODY_BYTES = 512 * 1024;
  const MAX_FAILURES = 2;
  const FAILURE_COOLDOWN_MS = 45 * 1000;
  const ACTIVE_TTL_MS = 15 * 1000;
  const STREAM_TTL_MS = 10 * 60 * 1000;
  const PENDING_STREAM_TTL_MS = 20 * 1000;
  const NON_REUSABLE_REQUEST_HEADERS = [
    'openai-sentinel-arkose-token',
    'openai-sentinel-chat-requirements-token',
    'openai-sentinel-chat-requirements-prepare-token',
    'openai-sentinel-proof-token',
    'openai-sentinel-turnstile-token'
  ];
  let template = null;
  let regenerateTemplate = null;
  let lastTurn = null;
  let active = null;
  let generation = 0;
  let captureGeneration = 0;
  let capturePending = false;
  let consecutiveFailures = 0;
  let cooldownUntil = 0;
  let disposed = false;
  let invalidateAfterActive = false;
  let lastCaptureCode = '';
  let lastStreamCode = '';
  let pendingStream = null;

  function requestUrl(input) {
    try { return new URL(typeof input === 'string' ? input : input && input.url, location.href); }
    catch (_) { return null; }
  }

  function requestMethod(input, init) {
    return String(init && init.method || input && input.method || 'GET').toUpperCase();
  }

  function isConversationPost(input, init) {
    const url = requestUrl(input);
    return requestMethod(input, init) === 'POST' && !!url && url.origin === location.origin &&
      /^\/(?:backend-api|backend-anon)\/(?:f\/)?conversation\/?$/.test(url.pathname);
  }

  function isPrivate(init) {
    return String(init && init.__elonPrivateTransport || '') === 'text_transaction_v1';
  }

  function requestClone(input, init, url) {
    try {
      const request = input instanceof Request
        ? new Request(input, init || {})
        : new Request(url.href, init || {});
      return request.clone();
    } catch (_) {
      return null;
    }
  }

  function hasNonReusableProof(request) {
    const headers = request && request.headers;
    if (!headers || typeof headers.get !== 'function') return true;
    let protectedHeader = false;
    if (typeof headers.forEach === 'function') {
      headers.forEach((value, name) => {
        if (value && /^openai-sentinel-/i.test(String(name))) protectedHeader = true;
      });
    }
    if (protectedHeader) return true;
    return NON_REUSABLE_REQUEST_HEADERS.some((name) => Boolean(headers.get(name)));
  }

  function captureTemplate(input, init) {
    if (disposed || isPrivate(init) || !isConversationPost(input, init)) return;
    const previous = { template, lastTurn };
    clearContext();
    const capturedGeneration = captureGeneration;
    const url = requestUrl(input);
    const request = requestClone(input, init, url);
    if (!request || !request.body) {
      lastCaptureCode = 'request_unavailable';
      return;
    }
    if (hasNonReusableProof(request)) {
      template = null;
      lastCaptureCode = 'dynamic_proof';
      return;
    }
    const pagePath = location.pathname;
    const capturedAt = Date.now();
    capturePending = true;
    Promise.resolve(request.clone().text()).then((text) => {
      if (disposed || capturedGeneration !== captureGeneration) return;
      capturePending = false;
      lastCaptureCode = 'invalid_body';
      if (!text || text.length > MAX_BODY_BYTES) return;
      let body;
      try { body = JSON.parse(text); }
      catch (_) { return; }
      const captured = policy.createTemplate(body, pagePath, capturedAt);
      if (captured) {
        template = { contract: captured, request };
        lastCaptureCode = '';
        lastStreamCode = '';
        reconcilePendingStream();
        return;
      }
      const regenerate = policy.createRegenerateTemplate(body, pagePath, capturedAt);
      if (regenerate) {
        template = previous.template;
        lastTurn = previous.lastTurn;
        regenerateTemplate = { contract: regenerate, request };
        lastCaptureCode = '';
        reconcilePendingStream();
        return;
      }
      lastCaptureCode = typeof policy.templateRejectionCode === 'function'
        ? String(policy.templateRejectionCode(body, pagePath, capturedAt) || '')
        : '';
    }).catch(function () {
      if (disposed || capturedGeneration !== captureGeneration) return;
      capturePending = false;
      lastCaptureCode = 'invalid_body';
    });
  }

  const wrappedFetch = function () {
    const args = arguments;
    captureTemplate(args[0], args[1] || {});
    return delegateFetch.apply(this, args);
  };
  window.fetch = wrappedFetch;

  function uuid() {
    if (window.crypto && typeof window.crypto.randomUUID === 'function') {
      return window.crypto.randomUUID();
    }
    const bytes = new Uint8Array(16);
    if (!window.crypto || typeof window.crypto.getRandomValues !== 'function') return '';
    window.crypto.getRandomValues(bytes);
    bytes[6] = (bytes[6] & 15) | 64;
    bytes[8] = (bytes[8] & 63) | 128;
    const hex = Array.from(bytes, (value) => value.toString(16).padStart(2, '0')).join('');
    return [hex.slice(0, 8), hex.slice(8, 12), hex.slice(12, 16),
      hex.slice(16, 20), hex.slice(20)].join('-');
  }

  function stateCode() {
    if (disposed) return 'disposed';
    if (active) return 'busy';
    if (Date.now() < cooldownUntil) return 'cooldown';
    if (capturePending) return 'capture_pending';
    if (!template) return lastCaptureCode ? 'capture_' + lastCaptureCode : 'template_unavailable';
    if (policy.ready(template.contract, location.pathname, Date.now())) return 'ready';
    return lastStreamCode ? 'stream_' + lastStreamCode : 'stream_not_confirmed';
  }

  function recordFailure() {
    consecutiveFailures += 1;
    if (consecutiveFailures >= MAX_FAILURES) cooldownUntil = Date.now() + FAILURE_COOLDOWN_MS;
  }

  function clearContext() {
    captureGeneration += 1;
    capturePending = false;
    template = null;
    regenerateTemplate = null;
    lastTurn = null;
    invalidateAfterActive = false;
    lastCaptureCode = '';
    lastStreamCode = '';
    pendingStream = null;
  }

  function invalidateContext() {
    captureGeneration += 1;
    capturePending = false;
    regenerateTemplate = null;
    lastTurn = null;
    if (active) {
      invalidateAfterActive = true;
      return false;
    }
    clearContext();
    return true;
  }

  function clearActive(token) {
    if (!active || (token != null && active.token !== token)) return false;
    if (active.timeoutId != null) window.clearTimeout(active.timeoutId);
    active = null;
    if (invalidateAfterActive) clearContext();
    return true;
  }

  function armActiveTimeout(token, controller, markTimedOut, delay = ACTIVE_TTL_MS) {
    return window.setTimeout(() => {
      if (!active || active.token !== token) return;
      markTimedOut();
      controller.abort();
      clearActive(token);
      recordFailure();
    }, delay);
  }

  function dispatchRequest(requestTemplate, body, command, kind, userMessageId) {
    const controller = new AbortController();
    let request;
    try {
      request = new Request(requestTemplate.request, {
        body: JSON.stringify(body),
        signal: controller.signal
      });
    } catch (_) {
      return Object.freeze({ dispatched: false, code: 'request_unavailable' });
    }
    const token = ++generation;
    active = {
      requestId: String(command.requestId || ''),
      controller,
      token,
      kind,
      userMessageId,
      timeoutId: null
    };
    let timedOut = false;
    active.timeoutId = armActiveTimeout(token, controller, () => { timedOut = true; });
    let responsePromise;
    try {
      responsePromise = Promise.resolve(delegateFetch(request, {
        signal: controller.signal,
        __elonPrivateTransport: 'text_transaction_v1'
      }));
    } catch (_) {
      controller.abort();
      clearActive(token);
      recordFailure();
      // Once fetch is invoked, a wrapper may throw after submitting the write.
      return Object.freeze({ dispatched: true, code: 'queued', kind, userMessageId,
        completion: Promise.resolve(Object.freeze({ status: 'unknown', code: 'synchronous_failure' })) });
    }
    const completion = responsePromise.then((response) => {
      if (timedOut) return Object.freeze({ status: 'unknown', code: 'timeout' });
      const result = policy.classifyResponse(response);
      if (result.accepted && active && active.token === token) {
        consecutiveFailures = 0;
        cooldownUntil = 0;
        window.clearTimeout(active.timeoutId);
        active.timeoutId = armActiveTimeout(
          token, controller, () => { timedOut = true; }, STREAM_TTL_MS
        );
      } else if (!result.accepted && active && active.token === token) {
        clearActive(token);
        recordFailure();
      }
      return Object.freeze({
        status: result.accepted ? 'accepted' : 'unknown',
        code: result.code
      });
    }).catch((error) => {
      const stopped = error && error.name === 'AbortError';
      const owned = clearActive(token);
      if (owned && !stopped && !timedOut) recordFailure();
      return Object.freeze({
        status: stopped && !timedOut ? 'accepted' : 'unknown',
        code: timedOut ? 'timeout' : stopped ? 'stopped' : 'network'
      });
    });
    return Object.freeze({
      dispatched: true,
      code: 'queued',
      kind,
      userMessageId,
      completion
    });
  }

  function dispatch(command) {
    const code = stateCode();
    if (code !== 'ready') return Object.freeze({ dispatched: false, code });
    const userMessageId = uuid();
    const requestUuid = uuid();
    const turnId = uuid();
    const built = policy.buildBody(
      template.contract,
      Object.assign({}, command, { pagePath: location.pathname }),
      { userMessageId, requestUuid, turnId },
      Date.now()
    );
    if (!built) return Object.freeze({ dispatched: false, code: 'invalid_command' });
    template.contract = policy.invalidate(template.contract, built.userMessageId);
    return dispatchRequest(template, built.body, command, 'send', built.userMessageId);
  }

  function dispatchRegenerate(command) {
    const code = stateCode();
    if (code !== 'ready') return Object.freeze({ dispatched: false, code });
    if (!regenerateTemplate || !lastTurn) {
      return Object.freeze({ dispatched: false, code: 'regenerate_template_unavailable' });
    }
    const requestUuid = uuid();
    const turnId = uuid();
    const built = policy.buildRegenerateBody(
      regenerateTemplate.contract,
      Object.assign({}, command, { pagePath: location.pathname }),
      { requestUuid, turnId },
      lastTurn,
      Date.now()
    );
    if (!built) return Object.freeze({ dispatched: false, code: 'invalid_regenerate_command' });
    template.contract = policy.invalidate(template.contract, built.userMessageId);
    return dispatchRequest(
      regenerateTemplate,
      built.body,
      command,
      'regenerate',
      built.userMessageId
    );
  }

  function applyAcceptedStream(accepted, stream) {
    template.contract = accepted;
    pendingStream = null;
    lastStreamCode = '';
    lastTurn = Object.freeze({
      conversationId: accepted.conversationId,
      userMessageId: active && active.userMessageId || accepted.userMessageId
    });
    if (active && String(stream.state || '') === 'completed') clearActive(active.token);
    consecutiveFailures = 0;
    cooldownUntil = 0;
    return true;
  }

  function acceptStreamValue(stream, pagePath, observedAt) {
    if (!template) return false;
    const accepted = policy.acceptStream(template.contract, stream, pagePath, observedAt);
    if (!accepted) {
      lastStreamCode = typeof policy.streamRejectionCode === 'function'
        ? String(policy.streamRejectionCode(
          template.contract,
          stream,
          pagePath,
          observedAt
        ) || 'rejected')
        : 'rejected';
      return false;
    }
    return applyAcceptedStream(accepted, stream);
  }

  function reconcilePendingStream() {
    if (!template || !pendingStream) return false;
    const receipt = pendingStream;
    pendingStream = null;
    const age = Date.now() - Number(receipt.observedAt || 0);
    if (!Number.isFinite(age) || age < 0 || age > PENDING_STREAM_TTL_MS) {
      lastStreamCode = 'receipt_expired';
      return false;
    }
    return acceptStreamValue(receipt, receipt.pagePath, receipt.observedAt);
  }

  function observeStream(stream) {
    if (disposed) return false;
    const pagePath = location.pathname;
    const observedAt = Date.now();
    if (!template) {
      const receipt = typeof policy.createStreamReceipt === 'function'
        ? policy.createStreamReceipt(stream, pagePath, observedAt)
        : null;
      if (receipt && capturePending) {
        pendingStream = receipt;
        lastStreamCode = 'awaiting_template';
      } else {
        lastStreamCode = typeof policy.streamRejectionCode === 'function'
          ? String(policy.streamRejectionCode(null, stream, pagePath, observedAt) || 'rejected')
          : 'rejected';
      }
      return false;
    }
    return acceptStreamValue(stream, pagePath, observedAt);
  }

  function stop(requestId) {
    if (!active || (requestId && active.requestId !== String(requestId))) return false;
    const current = active;
    current.controller.abort();
    clearActive(current.token);
    return true;
  }

  function state() {
    return Object.freeze({
      version: 4,
      enabled: true,
      state: stateCode(),
      active: !!active,
      activeKind: active ? active.kind : '',
      regenerateReady: !!regenerateTemplate && !!lastTurn,
      failures: Math.min(consecutiveFailures, MAX_FAILURES),
      cooldown: Date.now() < cooldownUntil
    });
  }

  window.__elonChatGptPrivateTextTransactionRelay = Object.freeze({
    version: 16,
    dispatch,
    dispatchRegenerate,
    invalidateContext,
    observeStream,
    state,
    stop,
    dispose: () => {
      if (disposed) return;
      disposed = true;
      if (active) {
        active.controller.abort();
        clearActive(active.token);
      }
      clearContext();
      if (window.fetch === wrappedFetch) window.fetch = delegateFetch;
    }
  });
})();

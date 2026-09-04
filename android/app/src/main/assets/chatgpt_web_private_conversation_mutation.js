(function (root, factory) {
  'use strict';

  const exported = Object.freeze({ version: 2, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (!root || !root.location || root.location.origin !== 'https://chatgpt.com') return;
  const current = root.__elonChatGptPrivateConversationMutation;
  if (current && Number(current.version) >= exported.version) return;
  root.__elonChatGptPrivateConversationMutation = Object.freeze(factory(root, {
    enabled: root.__elonChatGptPrivateConversationMutationsEnabled === true,
    privateTransport: root.__elonChatGptPrivateTransport,
    directory: root.__elonChatGptPrivateConversationDirectory
  }));
})(typeof window === 'object' ? window : globalThis, function (root, dependencies) {
  'use strict';

  const VERSION = 2;
  const WRITE_TIMEOUT_MS = 9000;
  const RECONCILE_TIMEOUT_MS = 4000;
  const UNCERTAIN_RECONCILE_WINDOW_MS = 16000;
  const UNCERTAIN_RECONCILE_BACKOFF_MS = Object.freeze([750, 1750, 3500, 5000]);
  const RETRY_COOLDOWN_MS = 5000;
  const CIRCUIT_COOLDOWN_MS = 45000;
  const MAX_FAILURES = 2;
  const BLOCKED_INHERITED_HEADERS = new Set([
    'connection',
    'content-length',
    'cookie',
    'host',
    'origin',
    'referer',
    'transfer-encoding',
    'user-agent'
  ]);
  const enabled = dependencies && dependencies.enabled === true;
  const privateTransport = dependencies && dependencies.privateTransport;
  const directory = dependencies && dependencies.directory;
  let active = null;
  let failures = 0;
  let cooldownUntil = 0;
  let lastOutcome = 'none';
  let lastLatencyMs = 0;

  function now() {
    return Date.now();
  }

  function sleep(delayMs) {
    return new Promise((resolve) => root.setTimeout(resolve, Math.max(0, delayMs)));
  }

  function targetFromPath(rawPath) {
    const path = String(rawPath || '').trim();
    const match = path.match(
      /^(?:\/c\/([A-Za-z0-9_-]{1,160})|\/g\/g-p-[A-Za-z0-9_-]{1,160}\/c\/([A-Za-z0-9_-]{1,160}))$/
    );
    return match ? Object.freeze({ path, id: match[1] || match[2] }) : null;
  }

  function supported() {
    return Boolean(enabled && root && typeof root.fetch === 'function' && privateTransport &&
      typeof privateTransport.acquireSameOriginRequestHeaders === 'function');
  }

  function state() {
    return Object.freeze({
      version: VERSION,
      enabled,
      supported: supported(),
      state: active ? 'busy' : cooldownUntil > now() ? 'cooldown' : supported() ? 'ready' : 'unavailable',
      failures,
      cooldownRemainingMs: Math.max(0, cooldownUntil - now()),
      lastOutcome,
      lastLatencyMs
    });
  }

  function sanitizeHeaders(source) {
    const headers = { Accept: 'application/json', 'Content-Type': 'application/json' };
    if (!source || typeof source !== 'object') return headers;
    Object.keys(source).forEach((name) => {
      const lower = String(name).toLowerCase();
      if (!lower || BLOCKED_INHERITED_HEADERS.has(lower) || lower === 'content-type') return;
      headers[name] = String(source[name]);
    });
    return headers;
  }

  function authorizationPresent(headers) {
    return Object.keys(headers).some((name) =>
      String(name).toLowerCase() === 'authorization' && /^Bearer\s+\S{8,65536}$/.test(headers[name])
    );
  }

  async function acquireHeaders() {
    const value = sanitizeHeaders(await privateTransport.acquireSameOriginRequestHeaders());
    if (!authorizationPresent(value)) throw new Error('auth_unavailable');
    return value;
  }

  async function fetchWithTimeout(url, options, timeoutMs) {
    const controller = typeof root.AbortController === 'function' ? new root.AbortController() : null;
    let timedOut = false;
    const timer = controller ? root.setTimeout(() => {
      timedOut = true;
      controller.abort();
    }, timeoutMs) : null;
    try {
      return await root.fetch(url, Object.assign({}, options, {
        signal: controller ? controller.signal : undefined
      }));
    } catch (error) {
      if (timedOut) throw new Error('timeout');
      throw error;
    } finally {
      if (timer !== null) root.clearTimeout(timer);
    }
  }

  function candidateArrays(payload) {
    if (Array.isArray(payload)) return [payload];
    if (!payload || typeof payload !== 'object') return [];
    const data = payload.data && typeof payload.data === 'object' ? payload.data : null;
    return [
      payload.items,
      payload.pins,
      payload.conversations,
      data && data.items,
      data && data.pins,
      data && data.conversations
    ].filter(Array.isArray);
  }

  function itemConversationId(value) {
    if (typeof value === 'string') return value;
    if (!value || typeof value !== 'object' || Array.isArray(value)) return '';
    return String(value.id || value.conversation_id || value.conversationId || '').trim();
  }

  function pinStateFromPayload(payload, conversationId) {
    const arrays = candidateArrays(payload);
    if (!arrays.length) return Object.freeze({ known: false, pinned: false });
    const pinned = arrays.some((values) => values.some((value) =>
      itemConversationId(value) === conversationId
    ));
    const hasMore = Boolean(payload && typeof payload === 'object' && (
      payload.has_more === true || payload.hasMore === true || payload.next_cursor || payload.nextCursor ||
      payload.data && (payload.data.has_more === true || payload.data.hasMore === true ||
        payload.data.next_cursor || payload.data.nextCursor)
    ));
    return Object.freeze({ known: pinned || !hasMore, pinned });
  }

  function acceptPinnedState(conversationId, pinned) {
    if (!directory || typeof directory.acceptPinnedState !== 'function') return false;
    try { return directory.acceptPinnedState(conversationId, pinned); }
    catch (_) { return false; }
  }

  async function reconcilePinned(conversationId, expectedPinned, headers, timeoutMs) {
    let response;
    try {
      response = await fetchWithTimeout('/backend-api/pins', {
        method: 'GET',
        credentials: 'include',
        cache: 'no-store',
        headers,
        __elonPrivateTransport: 'conversation_pin_reconcile_v1'
      }, Math.max(250, Number(timeoutMs) || RECONCILE_TIMEOUT_MS));
    } catch (_) {
      return Object.freeze({ confirmed: false, known: false });
    }
    if (!response || !response.ok || typeof response.json !== 'function') {
      return Object.freeze({ confirmed: false, known: false });
    }
    let payload;
    try { payload = await response.json(); }
    catch (_) { return Object.freeze({ confirmed: false, known: false }); }
    const observed = pinStateFromPayload(payload, conversationId);
    if (observed.known && observed.pinned === expectedPinned) {
      acceptPinnedState(conversationId, observed.pinned);
    }
    return Object.freeze({
      confirmed: observed.known && observed.pinned === expectedPinned,
      known: observed.known
    });
  }

  async function reconcileUncertainWrite(conversationId, expectedPinned, headers) {
    const deadline = now() + UNCERTAIN_RECONCILE_WINDOW_MS;
    let known = false;
    for (const delayMs of UNCERTAIN_RECONCILE_BACKOFF_MS) {
      const beforeDelay = deadline - now();
      if (beforeDelay <= 0) break;
      await sleep(Math.min(delayMs, beforeDelay));
      const remaining = deadline - now();
      if (remaining <= 0) break;
      const observed = await reconcilePinned(
        conversationId,
        expectedPinned,
        headers,
        Math.min(RECONCILE_TIMEOUT_MS, remaining)
      );
      known = known || observed.known;
      if (observed.confirmed) {
        return Object.freeze({ confirmed: true, known: true });
      }
    }
    return Object.freeze({ confirmed: false, known });
  }

  function recordFailure(outcome) {
    failures = Math.min(10, failures + 1);
    const longCooldown = failures >= MAX_FAILURES || outcome === 'auth';
    cooldownUntil = now() + (longCooldown ? CIRCUIT_COOLDOWN_MS : RETRY_COOLDOWN_MS);
    lastOutcome = outcome;
  }

  function recordSuccess(latencyMs) {
    failures = 0;
    cooldownUntil = 0;
    lastOutcome = 'success';
    lastLatencyMs = Math.max(0, Math.min(30000, Number(latencyMs) || 0));
  }

  function failureCode(error) {
    const message = String(error && error.message || 'network');
    if (message === 'timeout') return 'mutation_timeout';
    if (/auth|401|403/.test(message)) return 'mutation_auth_unavailable';
    return 'mutation_network_failure';
  }

  function rejected(code, attempted, reconciled) {
    return Object.freeze({
      ok: false,
      code,
      attempted: attempted === true,
      reconciled: reconciled === true
    });
  }

  async function executePinned(target, pinned) {
    let headers;
    try {
      headers = await acquireHeaders();
    } catch (error) {
      recordFailure('auth');
      return rejected(failureCode(error), false);
    }
    const startedAt = now();
    let response;
    try {
      response = await fetchWithTimeout(
        '/backend-api/conversation/' + encodeURIComponent(target.id),
        {
          method: 'PATCH',
          credentials: 'include',
          cache: 'no-store',
          headers,
          body: JSON.stringify({ is_starred: pinned }),
          __elonPrivateTransport: 'conversation_pin_v1'
        },
        WRITE_TIMEOUT_MS
      );
    } catch (error) {
      const code = failureCode(error);
      if (code === 'mutation_timeout' || code === 'mutation_network_failure') {
        const reconciliation = await reconcileUncertainWrite(target.id, pinned, headers);
        if (reconciliation.confirmed) {
          recordSuccess(now() - startedAt);
          return Object.freeze({
            ok: true,
            code: code === 'mutation_timeout'
              ? 'mutation_confirmed_after_timeout'
              : 'mutation_confirmed_after_transport_error',
            attempted: true,
            reconciled: true
          });
        }
        recordFailure(code === 'mutation_timeout' ? 'timeout' : 'network');
        return rejected(code, true, reconciliation.known);
      }
      recordFailure(code === 'mutation_auth_unavailable' ? 'auth' : code === 'mutation_timeout' ? 'timeout' : 'network');
      return rejected(code, true);
    }
    if (!response || !response.ok) {
      const status = Math.max(0, Number(response && response.status) || 0);
      if (status === 401 || status === 403) {
        const authContext = root.__elonChatGptPrivateAuthContext;
        if (authContext && typeof authContext.invalidate === 'function') {
          authContext.invalidate('conversation_mutation_rejected');
        }
      }
      recordFailure(status === 401 || status === 403 ? 'auth' : 'http');
      return rejected('mutation_http_' + status, true);
    }
    recordSuccess(now() - startedAt);
    acceptPinnedState(target.id, pinned);
    const reconciliation = await reconcilePinned(target.id, pinned, headers);
    return Object.freeze({
      ok: true,
      code: reconciliation.confirmed ? 'mutation_confirmed' : 'mutation_server_acknowledged',
      attempted: true,
      reconciled: reconciliation.confirmed
    });
  }

  function setPinned(rawPath, pinned) {
    const target = targetFromPath(rawPath);
    if (!target || typeof pinned !== 'boolean') return Promise.resolve(rejected('invalid_mutation', false));
    if (!supported()) return Promise.resolve(rejected('mutation_unavailable', false));
    if (active) return Promise.resolve(rejected('mutation_busy', false));
    if (cooldownUntil > now()) return Promise.resolve(rejected('mutation_circuit_open', false));
    const request = executePinned(target, pinned).finally(() => {
      if (active === request) active = null;
    });
    active = request;
    return request;
  }

  function handle(action, command, respond, scheduleSnapshot, directoryRequests) {
    if (action !== 'set_conversation_pinned') return false;
    setPinned(String(command && command.value || ''), command && command.selected)
      .then((outcome) => {
        const value = outcome && typeof outcome === 'object' ? outcome : {};
        if (value.ok === true) {
          if (directoryRequests && typeof directoryRequests.emitSnapshot === 'function') {
            directoryRequests.emitSnapshot(null);
          }
          if (typeof scheduleSnapshot === 'function') scheduleSnapshot(true);
        }
        respond(action, value.ok === true, String(value.code || 'mutation_failed'));
      })
      .catch(() => respond(action, false, 'mutation_failed'));
    return true;
  }

  return Object.freeze({ version: VERSION, enabled, setPinned, handle, state });
});

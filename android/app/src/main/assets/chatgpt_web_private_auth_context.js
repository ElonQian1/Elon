(function (root, factory) {
  'use strict';

  const exported = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (!root || !root.location || root.location.origin !== 'https://chatgpt.com') return;
  const current = root.__elonChatGptPrivateAuthContext;
  if (current && Number(current.version) >= exported.version) return;
  const context = Object.freeze(factory(root));
  root.__elonChatGptPrivateAuthContext = context;
  if (context.enabled) Promise.resolve().then(context.prewarm).catch(() => {});
})(typeof window === 'object' ? window : globalThis, function (root) {
  'use strict';

  const VERSION = 1;
  const AUTH_PATH = '/api/auth/session';
  const REQUEST_TIMEOUT_MS = 5000;
  const FALLBACK_TTL_MS = 10 * 60 * 1000;
  const MAX_TTL_MS = 15 * 60 * 1000;
  const REFRESH_EARLY_MS = 60 * 1000;
  const RETRY_COOLDOWN_MS = 5000;
  const CIRCUIT_COOLDOWN_MS = 45 * 1000;
  const listeners = new Set();
  const enabled = root.__elonChatGptPrivateAuthContextEnabled === true ||
    root.__elonChatGptPrivateConversationPrefetchEnabled === true ||
    root.__elonChatGptPrivateDictationEnabled === true ||
    root.__elonChatGptPrivateReadAloudEnabled === true;
  let authorization = '';
  let refreshAt = 0;
  let failures = 0;
  let cooldownUntil = 0;
  let lastOutcome = 'none';
  let lastSuccessAt = 0;
  let lastLatencyMs = 0;
  let inFlight = null;

  function now() {
    return Date.now();
  }

  function supported() {
    return enabled && root && typeof root.fetch === 'function';
  }

  function hasFreshAuthorization() {
    return authorization.length > 0 && refreshAt > now();
  }

  function phase() {
    if (hasFreshAuthorization()) return 'ready';
    if (inFlight) return 'loading';
    if (cooldownUntil > now()) return 'cooldown';
    return supported() ? 'idle' : 'unavailable';
  }

  function snapshot() {
    return Object.freeze({
      version: VERSION,
      enabled,
      supported: supported(),
      state: phase(),
      ready: hasFreshAuthorization(),
      loading: !!inFlight,
      failures,
      cooldownRemainingMs: Math.max(0, cooldownUntil - now()),
      refreshRemainingMs: Math.max(0, refreshAt - now()),
      lastOutcome,
      lastSuccessAt,
      lastLatencyMs
    });
  }

  function notify() {
    const value = snapshot();
    listeners.forEach((listener) => {
      try { listener(value); } catch (_) {}
    });
  }

  function authorizationFrom(source) {
    let value = '';
    if (!source) return value;
    try {
      if (typeof source.forEach === 'function') {
        source.forEach((entry, name) => {
          if (String(name).toLowerCase() === 'authorization') value = String(entry || '');
        });
      } else if (typeof source === 'object') {
        Object.keys(source).forEach((name) => {
          if (String(name).toLowerCase() === 'authorization') value = String(source[name] || '');
        });
      }
    } catch (_) {
      return '';
    }
    return /^Bearer\s+\S{8,65536}$/.test(value) ? value : '';
  }

  function payloadRefreshAt(payload) {
    const current = now();
    const raw = payload && payload.expires;
    let expiresAt = 0;
    if (typeof raw === 'number' && Number.isFinite(raw)) {
      expiresAt = raw < 1000000000000 ? raw * 1000 : raw;
    } else if (typeof raw === 'string' && raw.trim()) {
      const numeric = Number(raw);
      expiresAt = Number.isFinite(numeric) && numeric > 0
        ? (numeric < 1000000000000 ? numeric * 1000 : numeric)
        : Date.parse(raw);
    }
    const bounded = Number.isFinite(expiresAt) && expiresAt > current + REFRESH_EARLY_MS
      ? Math.min(MAX_TTL_MS, expiresAt - current - REFRESH_EARLY_MS)
      : FALLBACK_TTL_MS;
    return current + Math.max(30000, bounded);
  }

  function acceptAuthorization(value, expiresAt, outcome, latencyMs) {
    const normalized = authorizationFrom({ Authorization: value });
    if (!normalized) return false;
    authorization = normalized;
    refreshAt = Math.max(now() + 30000, Number(expiresAt) || now() + FALLBACK_TTL_MS);
    failures = 0;
    cooldownUntil = 0;
    lastOutcome = String(outcome || 'ready').slice(0, 32);
    lastSuccessAt = now();
    lastLatencyMs = Math.max(0, Math.min(30000, Number(latencyMs) || 0));
    notify();
    return true;
  }

  function acceptObservedHeaders(headers) {
    const observed = authorizationFrom(headers);
    return observed
      ? acceptAuthorization(observed, now() + FALLBACK_TTL_MS, 'official_observed', 0)
      : false;
  }

  function clearAuthorization() {
    authorization = '';
    refreshAt = 0;
  }

  function recordFailure(error) {
    clearAuthorization();
    failures = Math.min(10, failures + 1);
    const message = String(error && error.message || 'network');
    const longCooldown = /^auth_(?:http_)?(?:401|403)$/.test(message) || failures >= 2;
    cooldownUntil = now() + (longCooldown ? CIRCUIT_COOLDOWN_MS : RETRY_COOLDOWN_MS);
    lastOutcome = /^auth_/.test(message) ? 'auth' : message === 'timeout' ? 'timeout' : 'network';
    notify();
  }

  function copyRequestHeaders() {
    if (!hasFreshAuthorization()) {
      if (authorization) {
        clearAuthorization();
        notify();
        Promise.resolve().then(prewarm).catch(() => {});
      }
      return null;
    }
    return { Authorization: authorization };
  }

  async function fetchSession() {
    const startedAt = now();
    const controller = typeof root.AbortController === 'function' ? new root.AbortController() : null;
    let timedOut = false;
    const timer = controller ? root.setTimeout(() => {
      timedOut = true;
      controller.abort();
    }, REQUEST_TIMEOUT_MS) : null;
    try {
      const response = await root.fetch(AUTH_PATH, {
        method: 'GET',
        credentials: 'include',
        cache: 'no-store',
        headers: { Accept: 'application/json' },
        signal: controller ? controller.signal : undefined
      });
      if (!response || !response.ok) throw new Error('auth_http_' + Number(response && response.status));
      const payload = await response.json();
      const accessToken = payload && typeof payload.accessToken === 'string' ? payload.accessToken : '';
      if (!acceptAuthorization(
        'Bearer ' + accessToken,
        payloadRefreshAt(payload),
        'session_ready',
        now() - startedAt
      )) {
        throw new Error('auth_missing');
      }
      return copyRequestHeaders();
    } catch (error) {
      if (timedOut) throw new Error('timeout');
      throw error;
    } finally {
      if (timer !== null) root.clearTimeout(timer);
    }
  }

  function canAcquire() {
    return supported() && cooldownUntil <= now();
  }

  function acquireRequestHeaders() {
    const cached = copyRequestHeaders();
    if (cached) return Promise.resolve(cached);
    if (!supported()) return Promise.reject(new Error('auth_unavailable'));
    if (cooldownUntil > now()) return Promise.reject(new Error('auth_cooldown'));
    if (inFlight) return inFlight;
    lastOutcome = 'loading';
    inFlight = fetchSession().catch((error) => {
      recordFailure(error);
      throw error;
    }).finally(() => {
      inFlight = null;
      notify();
    });
    notify();
    return inFlight;
  }

  function prewarm() {
    return acquireRequestHeaders();
  }

  function invalidate(reason) {
    clearAuthorization();
    cooldownUntil = 0;
    lastOutcome = String(reason || 'invalidated').slice(0, 32);
    notify();
  }

  return {
    version: VERSION,
    enabled,
    canAcquire,
    prewarm,
    acquireRequestHeaders,
    copyRequestHeaders,
    acceptObservedHeaders,
    invalidate,
    state: snapshot,
    subscribe: (listener) => {
      if (typeof listener !== 'function') return function () {};
      listeners.add(listener);
      return () => listeners.delete(listener);
    }
  };
});

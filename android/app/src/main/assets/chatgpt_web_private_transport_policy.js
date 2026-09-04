(function (root, factory) {
  'use strict';
  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptPrivateTransportPolicy = api;
})(typeof window !== 'undefined' ? window : globalThis, function () {
  'use strict';

  const VERSION = 1;
  const STORAGE_KEY = 'elon.chatgpt.private.transport.health.v1';
  const MAX_STATE_AGE_MS = 24 * 60 * 60 * 1000;
  const OFFICIAL_FRESH_MS = 2 * 60 * 1000;
  const MAX_COOLDOWN_MS = 10 * 60 * 1000;
  const OUTCOMES = new Set([
    'none', 'success', 'timeout', 'auth', 'context', 'http', 'network', 'parse',
    'empty', 'official_error'
  ]);

  function finite(value, fallback, minimum, maximum) {
    const parsed = Number(value);
    if (!Number.isFinite(parsed)) return fallback;
    return Math.max(minimum, Math.min(maximum, Math.round(parsed)));
  }

  function emptyState(now) {
    return {
      schema: VERSION,
      updatedAt: now,
      officialOkAt: 0,
      officialLatencyMs: 0,
      privateLatencyMs: 0,
      successes: 0,
      failures: 0,
      consecutiveFailures: 0,
      cooldownUntil: 0,
      lastOutcome: 'none'
    };
  }

  function sanitize(raw, now) {
    if (!raw || Number(raw.schema) !== VERSION) return emptyState(now);
    const updatedAt = finite(raw.updatedAt, 0, 0, now + 60_000);
    if (!updatedAt || now - updatedAt > MAX_STATE_AGE_MS) return emptyState(now);
    return {
      schema: VERSION,
      updatedAt,
      officialOkAt: finite(raw.officialOkAt, 0, 0, now + 60_000),
      officialLatencyMs: finite(raw.officialLatencyMs, 0, 0, 60_000),
      privateLatencyMs: finite(raw.privateLatencyMs, 0, 0, 60_000),
      successes: finite(raw.successes, 0, 0, 1000),
      failures: finite(raw.failures, 0, 0, 1000),
      consecutiveFailures: finite(raw.consecutiveFailures, 0, 0, 20),
      cooldownUntil: finite(raw.cooldownUntil, 0, 0, now + MAX_COOLDOWN_MS),
      lastOutcome: OUTCOMES.has(String(raw.lastOutcome)) ? String(raw.lastOutcome) : 'none'
    };
  }

  function read(storage, now) {
    if (!storage || typeof storage.getItem !== 'function') return emptyState(now);
    try {
      return sanitize(JSON.parse(storage.getItem(STORAGE_KEY) || 'null'), now);
    } catch (_) {
      return emptyState(now);
    }
  }

  function create(options) {
    const config = options || {};
    const now = typeof config.now === 'function' ? config.now : Date.now;
    const storage = config.storage || null;
    const enabled = config.enabled === true;
    let state = read(storage, now());

    function persist() {
      state.updatedAt = now();
      if (!storage || typeof storage.setItem !== 'function') return;
      try {
        storage.setItem(STORAGE_KEY, JSON.stringify(state));
      } catch (_) {
        // Health metadata is optional; private requests never depend on storage.
      }
    }

    function average(previous, current) {
      return previous > 0 ? Math.round((previous * 0.65) + (current * 0.35)) : current;
    }

    function recordOfficial(status, elapsedMs) {
      const current = now();
      const elapsed = finite(elapsedMs, 0, 0, 60_000);
      if (status >= 200 && status < 300) {
        state.officialOkAt = current;
        state.officialLatencyMs = average(state.officialLatencyMs, elapsed);
      } else if (status === 401 || status === 403) {
        state.lastOutcome = 'auth';
        state.cooldownUntil = Math.max(state.cooldownUntil, current + (5 * 60 * 1000));
      } else if (status >= 500) {
        state.lastOutcome = 'official_error';
        state.cooldownUntil = Math.max(state.cooldownUntil, current + 30_000);
      }
      persist();
    }

    function canAttempt(contextReady) {
      const current = now();
      if (!enabled || contextReady !== true) return false;
      if (state.cooldownUntil > current) return false;
      return state.officialOkAt > 0 && current - state.officialOkAt <= OFFICIAL_FRESH_MS;
    }

    function attemptBudgetMs() {
      const baseline = state.privateLatencyMs || state.officialLatencyMs || 500;
      const maximum = state.successes > 0 ? 4000 : 5000;
      return finite((baseline * 1.35) + 2200, 3500, 3000, maximum);
    }

    function recordSuccess(elapsedMs) {
      const elapsed = finite(elapsedMs, 0, 0, 60_000);
      state.successes = Math.min(1000, state.successes + 1);
      state.consecutiveFailures = 0;
      state.cooldownUntil = 0;
      state.privateLatencyMs = average(state.privateLatencyMs, elapsed);
      state.lastOutcome = 'success';
      persist();
    }

    function recordFailure(outcome) {
      const current = now();
      const kind = OUTCOMES.has(String(outcome)) ? String(outcome) : 'network';
      state.failures = Math.min(1000, state.failures + 1);
      state.consecutiveFailures = Math.min(20, state.consecutiveFailures + 1);
      state.lastOutcome = kind;
      const cooldown = kind === 'auth' || kind === 'context'
        ? 5 * 60 * 1000
        : kind === 'timeout' ? 60_000
          : kind === 'parse' || kind === 'empty' ? 30_000
            : state.consecutiveFailures >= 2 ? 30_000 : 10_000;
      state.cooldownUntil = Math.max(state.cooldownUntil, current + cooldown);
      persist();
    }

    function snapshot() {
      const current = now();
      return Object.freeze({
        version: VERSION,
        enabled,
        officialFresh: state.officialOkAt > 0 && current - state.officialOkAt <= OFFICIAL_FRESH_MS,
        cooldownRemainingMs: Math.max(0, state.cooldownUntil - current),
        officialLatencyMs: state.officialLatencyMs,
        privateLatencyMs: state.privateLatencyMs,
        successes: state.successes,
        failures: state.failures,
        consecutiveFailures: state.consecutiveFailures,
        lastOutcome: state.lastOutcome,
        attemptBudgetMs: attemptBudgetMs()
      });
    }

    return Object.freeze({
      canAttempt,
      attemptBudgetMs,
      recordOfficial,
      recordSuccess,
      recordFailure,
      snapshot
    });
  }

  return Object.freeze({ version: VERSION, storageKey: STORAGE_KEY, create });
});

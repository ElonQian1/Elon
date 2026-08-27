(function (root, factory) {
  'use strict';
  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) {
    root.__elonWinChatGptPrivateTransportHealth = Object.freeze({
      version: api.version,
      snapshot: () => api.snapshot(root),
      enrich: (payload) => api.enrich(root, payload)
    });
  }
})(typeof window !== 'undefined' ? window : globalThis, function () {
  'use strict';

  const VERSION = 1;
  const OUTCOMES = new Set([
    'none', 'success', 'timeout', 'auth', 'context', 'http', 'network', 'parse',
    'empty', 'official_error'
  ]);

  function integer(value, fallback, minimum, maximum) {
    const parsed = Number(value);
    if (!Number.isFinite(parsed)) return fallback;
    return Math.max(minimum, Math.min(maximum, Math.round(parsed)));
  }

  function snapshot(root) {
    const transport = root && root.__elonChatGptPrivateTransport;
    if (!transport || typeof transport.health !== 'function') return null;
    let health;
    try { health = transport.health(); } catch (_) { return null; }
    if (!health || typeof health !== 'object') return null;
    let prefetchReady = false;
    try {
      prefetchReady = typeof transport.conversationPrefetchReady === 'function' &&
        transport.conversationPrefetchReady() === true;
    } catch (_) {}
    return {
      version: VERSION,
      prefetchEnabled: transport.conversationPrefetchEnabled === true,
      prefetchReady,
      officialFresh: health.officialFresh === true,
      cooldownRemainingMs: integer(health.cooldownRemainingMs, 0, 0, 10 * 60 * 1000),
      officialLatencyMs: integer(health.officialLatencyMs, 0, 0, 60 * 1000),
      privateLatencyMs: integer(health.privateLatencyMs, 0, 0, 60 * 1000),
      successes: integer(health.successes, 0, 0, 1000),
      failures: integer(health.failures, 0, 0, 1000),
      consecutiveFailures: integer(health.consecutiveFailures, 0, 0, 20),
      lastOutcome: OUTCOMES.has(String(health.lastOutcome)) ? String(health.lastOutcome) : 'none',
      attemptBudgetMs: integer(health.attemptBudgetMs, 700, 350, 1200),
      sampledAtMs: integer(Date.now(), 0, 0, Number.MAX_SAFE_INTEGER)
    };
  }

  function enrich(root, payload) {
    const original = String(payload || '');
    let envelope;
    try { envelope = JSON.parse(original); } catch (_) { return original; }
    if (!envelope || !envelope.event || envelope.event.type !== 'message_snapshot') return original;
    const health = snapshot(root);
    if (!health) return original;
    envelope.event.privateTransportHealth = health;
    return JSON.stringify(envelope);
  }

  return Object.freeze({ version: VERSION, snapshot, enrich });
});

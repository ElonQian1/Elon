(function (root) {
  'use strict';
  const LIMITS = Object.freeze({ records: 128, bytes: 1024 * 1024, eventBytes: 16384,
    durationMs: 15 * 60 * 1000, perMinute: 120 });
  const REASONS = new Set(['idle', 'capturing', 'stopped', 'expired', 'navigated',
    'cleared', 'replaced', 'unavailable']);
  const count = (n) => Number.isSafeInteger(n) && n >= 0 ? Math.min(n, 999999) : 0;
  function create(tabId, documentId, sessionId, now) {
    return { tabId, documentId, sessionId, startedAt: now, expiresAt: now + LIMITS.durationMs,
      active: true, reason: 'capturing', records: [], observations: 0, dropped: 0,
      rateStart: now, rateCount: 0 };
  }
  function expire(state, now) {
    if (state?.active && now >= state.expiresAt) stop(state, 'expired');
    return state;
  }
  function stop(state, reason = 'stopped') {
    if (state) { state.active = false; state.reason = REASONS.has(reason) ? reason : 'stopped'; }
    return state;
  }
  function accept(state, sender, envelope, sanitize, now) {
    expire(state, now);
    if (!state?.active || sender.tab?.id !== state.tabId || sender.frameId !== 0 ||
        sender.documentId !== state.documentId || sender.origin !== 'https://www.binance.com' ||
        envelope?.sessionId !== state.sessionId) return false;
    // Do not persist or log page messages before strict schema reconstruction.
    if (now - state.rateStart >= 60000) { state.rateStart = now; state.rateCount = 0; }
    state.rateCount += 1;
    if (state.rateCount > LIMITS.perMinute) { state.dropped = count(state.dropped + 1); return false; }
    let observation;
    try {
      if (JSON.stringify(envelope.observation).length > LIMITS.eventBytes) return false;
      observation = sanitize(envelope.observation);
    } catch { return false; }
    if (!observation) return false;
    const key = JSON.stringify(observation);
    state.observations = count(state.observations + 1);
    const found = state.records.find((record) => JSON.stringify(record.observation) === key);
    if (found) { found.count = count(found.count + 1); found.lastSeen = now; return true; }
    const record = { observation, count: 1, firstSeen: now, lastSeen: now };
    if (state.records.length >= LIMITS.records ||
        JSON.stringify(state.records).length + JSON.stringify(record).length > LIMITS.bytes) {
      state.dropped = count(state.dropped + 1); return false;
    }
    state.records.push(record);
    return true;
  }
  function status(state, now) {
    expire(state, now);
    return { active: !!state?.active, records: state?.records.length || 0,
      observations: count(state?.observations), dropped: count(state?.dropped),
      startedAt: state?.startedAt || 0, expiresAt: state?.expiresAt || 0,
      reason: state?.reason || 'idle' };
  }
  function report(state, now) {
    expire(state, now);
    return { schema: 'binance-grid-observer-report.v1', toolVersion: '0.1.0',
      provenance: 'untrusted_page_observation', capturedAt: state.startedAt, exportedAt: now,
      status: status(state, now), coverage: {
        transport: ['fetch', 'xhr'], phase: 'requests_started_after_enable',
        missing: ['earlier_requests', 'saved_transport_references', 'workers', 'websockets'],
        zeroSamplesMeans: 'coverage_unknown', contractVerified: false, tradingEnabled: false,
        requestValuesIncluded: false, headersIncluded: false, cookiesIncluded: false,
        unknownFields: 'collapsed', pathSegments: 'allowlisted_template',
        arrayElementSampleLimit: 3, unobservedOrFilteredRequestsCounted: false },
      records: state.records.map((entry) => ({ ...entry })) };
  }
  // Session storage is extension-owned, but revalidate it after every worker restart.
  function restore(value, sanitize, now) {
    if (!value || !Number.isSafeInteger(value.tabId) || value.tabId < 0 ||
        typeof value.documentId !== 'string' || value.documentId.length > 128 ||
        !/^[a-zA-Z0-9_-]{16,128}$/.test(value.sessionId || '') ||
        !Number.isSafeInteger(value.startedAt) || value.startedAt < 0 || value.startedAt > now ||
        value.expiresAt !== value.startedAt + LIMITS.durationMs || !Array.isArray(value.records) ||
        value.records.length > LIMITS.records) return null;
    const state = create(value.tabId, value.documentId, value.sessionId, value.startedAt);
    state.active = value.active === true;
    state.reason = REASONS.has(value.reason) ? value.reason : 'unavailable';
    state.observations = count(value.observations); state.dropped = count(value.dropped);
    state.rateStart = Number.isSafeInteger(value.rateStart) && value.rateStart <= now ? value.rateStart : now;
    state.rateCount = count(value.rateCount);
    for (const entry of value.records) {
      const observation = sanitize(entry?.observation);
      if (!observation || !Number.isSafeInteger(entry.firstSeen) || !Number.isSafeInteger(entry.lastSeen) ||
          entry.firstSeen < state.startedAt || entry.lastSeen < entry.firstSeen || entry.lastSeen > now) return null;
      state.records.push({ observation, count: count(entry.count), firstSeen: entry.firstSeen, lastSeen: entry.lastSeen });
    }
    if (JSON.stringify(state.records).length > LIMITS.bytes) return null;
    return expire(state, now);
  }
  const api = Object.freeze({ LIMITS, create, expire, stop, accept, status, report, restore });
  root.BinanceGridStore = api;
  if (typeof module !== 'undefined') module.exports = api;
})(globalThis);
